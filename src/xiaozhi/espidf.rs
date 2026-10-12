//! The XiaoZhi call on the device. A thread on the second core owns the
//! speaker and the microphone during the call: it checks in with the OTA
//! server, waits while the device is linked, then talks over a WebSocket
//! with Opus audio both ways. `main.rs` lends it the audio runtime and takes
//! it back when the thread ends.

use std::{
    collections::VecDeque,
    ffi::{c_void, CStr, CString},
    path::PathBuf,
    sync::{
        mpsc::{self, Receiver, RecvTimeoutError, Sender, TryRecvError},
        Arc, Mutex, MutexGuard, PoisonError,
    },
    thread::JoinHandle,
    time::{Duration, Instant},
};

use anyhow::{anyhow, ensure, Context, Result};
use embedded_svc::http::Method;
use esp_idf_svc::{
    hal::delay::TickType,
    http::client::{Configuration, EspHttpConnection},
    nvs::{EspDefaultNvsPartition, EspNvs, NvsDefault},
    sys::{self, esp, opus as codec},
};
use log::{info, warn};

use super::{
    activation::{self, Activation, DeviceInfo, OtaReply, Partition, WebSocketConfig},
    call::{CallCommand, CallEvent, CallPhase, Conversation, Effect},
    mcp::{self, BibleRequest, McpDevice},
    protocol::{self, DeviceMessage, FrameType, ServerMessage},
};
use crate::{network::NetworkSnapshot, voice_notes::VoiceMicGain};

/// TLS for the OTA check, then Opus: both need room.
const CALL_STACK_BYTES: usize = 40 * 1024;
/// Above the main loop, so a screen redraw never starves the audio.
const CALL_PRIORITY: u8 = 4;
const OTA_TIMEOUT: Duration = Duration::from_secs(15);
/// The activation request waits on the server until the code is typed.
const ACTIVATE_TIMEOUT: Duration = Duration::from_secs(45);
const LINK_TIMEOUT: Duration = Duration::from_secs(10 * 60);
const HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(10);
/// A call with nothing from the server for this long ends.
const QUIET_LIMIT: Duration = Duration::from_secs(120);
const REPLY_LIMIT_BYTES: usize = 16 * 1024;
/// Microphone audio read per step: 20 ms of 16 kHz mono PCM16.
const MIC_CHUNK_BYTES: usize = 640;
/// Stereo scratch for the I2S driver: 120 ms of mono becomes this.
const STEREO_BYTES: usize = 8 * 1024;
/// Speech queued for the speaker, about six seconds.
const PLAYBACK_LIMIT: usize = 100;
/// Decoded audio of one packet, up to 120 ms at 16 kHz.
const DECODE_BYTES: usize = 3_840;
const WS_SEND_TIMEOUT_MS: u64 = 5_000;
/// Biggest message kept while its parts arrive.
const WS_MESSAGE_LIMIT: usize = 64 * 1024;
const NVS_NAMESPACE: &str = "xiaozhi";

/// What a call needs from the speaker and the microphone.
pub trait CallAudio: Send + 'static {
    /// Wake the codec with the speaker off.
    fn start_call(&mut self) -> Result<()>;
    fn set_speaker(&mut self, on: bool) -> Result<()>;
    /// Mono PCM16 into `mono` within `timeout_ms`; the bytes read. `stereo`
    /// is scratch of twice the size.
    fn read_mic(
        &mut self,
        stereo: &mut [u8],
        mono: &mut [u8],
        gain: VoiceMicGain,
        timeout_ms: u32,
    ) -> Result<usize>;
    /// Drop what the microphone captured so far.
    fn flush_mic(&mut self, stereo: &mut [u8]) -> Result<()>;
    /// Queue mono PCM16 for the speaker; returns once it is queued.
    fn play(&mut self, mono: &[u8], stereo: &mut [u8]) -> Result<()>;
    fn call_volume(&self) -> u8;
    fn set_call_volume(&mut self, percent: u8) -> Result<()>;
    /// Speaker off and the codec asleep again.
    fn end_call(&mut self);
}

/// What the call thread needs from the main loop.
#[derive(Clone, Debug)]
pub struct CallSetup {
    pub device: DeviceInfo,
    pub ota_url: String,
    pub mic_gain: VoiceMicGain,
    pub battery_percent: Option<u8>,
    pub rssi: Option<i32>,
    pub bible_root: PathBuf,
    pub bible_translation: Option<String>,
}

/// A running call: keys go in, events come out, the audio comes back.
pub struct XiaozhiCall<A> {
    thread: Option<JoinHandle<()>>,
    commands: Sender<CallCommand>,
    events: Receiver<CallEvent>,
    audio: Arc<Mutex<Option<A>>>,
}

impl<A: CallAudio> XiaozhiCall<A> {
    /// Start a call with `audio`. When the thread cannot start, the audio
    /// comes back with the error.
    pub fn start(audio: A, setup: CallSetup) -> Result<Self, (Option<A>, std::io::Error)> {
        let (command_sender, commands) = mpsc::channel();
        let (event_sender, events) = mpsc::channel();
        let handed = Arc::new(Mutex::new(Some(audio)));
        let slot = Arc::clone(&handed);
        crate::runtime_worker::wait_for_stack_memory(CALL_STACK_BYTES);
        let builder = std::thread::Builder::new()
            .name("xiaozhi".into())
            .stack_size(CALL_STACK_BYTES);
        let task = move || run_call(&slot, &setup, &commands, &event_sender);
        match crate::photos::worker::spawn_on_second_core_at(builder, CALL_PRIORITY, task) {
            Ok(thread) => Ok(Self {
                thread: Some(thread),
                commands: command_sender,
                events,
                audio: handed,
            }),
            Err(error) => Err((lock(&handed).take(), error)),
        }
    }

    /// Events since the last poll.
    #[must_use]
    pub fn poll(&self) -> Vec<CallEvent> {
        self.events.try_iter().collect()
    }

    pub fn send(&self, command: CallCommand) {
        let _ = self.commands.send(command);
    }

    #[must_use]
    pub fn is_finished(&self) -> bool {
        self.thread.as_ref().map_or(true, JoinHandle::is_finished)
    }

    /// The audio, while the conversation has not taken it: a call ended
    /// before it opened gives the speaker back at once.
    pub fn reclaim_audio(&self) -> Option<A> {
        lock(&self.audio).take()
    }

    /// Wait for the thread to end and take the audio back.
    pub fn finish(mut self) -> Option<A> {
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
        lock(&self.audio).take()
    }
}

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

/// The client id and the linked flag, kept in NVS between calls.
#[derive(Clone)]
pub struct XiaozhiIdentity {
    partition: EspDefaultNvsPartition,
}

impl XiaozhiIdentity {
    #[must_use]
    pub fn new(partition: EspDefaultNvsPartition) -> Self {
        Self { partition }
    }

    fn open(&self) -> Result<EspNvs<NvsDefault>> {
        EspNvs::new(self.partition.clone(), NVS_NAMESPACE, true)
            .context("opening the XiaoZhi settings")
    }

    /// The UUID this device sends as its Client-Id, made on first use.
    pub fn client_id(&self) -> Result<String> {
        let nvs = self.open()?;
        let mut buffer = [0_u8; 40];
        if let Some(id) = nvs.get_str("uuid", &mut buffer)? {
            if !id.is_empty() {
                return Ok(id.to_owned());
            }
        }
        let mut random = [0_u8; 16];
        unsafe { sys::esp_fill_random(random.as_mut_ptr().cast(), random.len()) };
        let id = activation::uuid_v4(random);
        nvs.set_str("uuid", &id)?;
        Ok(id)
    }

    #[must_use]
    pub fn linked(&self) -> bool {
        self.open()
            .ok()
            .and_then(|nvs| nvs.get_u8("linked").ok().flatten())
            == Some(1)
    }

    pub fn set_linked(&self) -> Result<()> {
        self.open()?
            .set_u8("linked", 1)
            .context("saving the XiaoZhi link")
    }
}

/// What the device tells the OTA server, read from ESP-IDF.
#[must_use]
pub fn device_info(client_id: String, network: &NetworkSnapshot) -> DeviceInfo {
    let mut mac = [0_u8; 6];
    unsafe { sys::esp_read_mac(mac.as_mut_ptr(), sys::esp_mac_type_t_ESP_MAC_WIFI_STA) };
    let app = unsafe { &*sys::esp_app_get_description() };
    let mut flash_size = 0_u32;
    unsafe { sys::esp_flash_get_size(core::ptr::null_mut(), &mut flash_size) };
    let running = unsafe { sys::esp_ota_get_running_partition().as_ref() };
    DeviceInfo {
        mac: activation::mac_text(mac),
        client_id,
        firmware_version: crate::build_info::FIRMWARE_VERSION.into(),
        compile_time: format!("{}T{}Z", c_text(&app.date), c_text(&app.time)),
        idf_version: c_text(&app.idf_ver),
        elf_sha256: app
            .app_elf_sha256
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect(),
        // esp-idf-sys does not bind esp_chip_info: an ESP32-S3, Wi-Fi and BLE.
        chip_model: 9,
        chip_cores: 2,
        chip_revision: 0,
        chip_features: 0x12,
        flash_size,
        minimum_free_heap: unsafe { sys::esp_get_minimum_free_heap_size() },
        partitions: partitions(),
        running_partition: running.map_or_else(String::new, |partition| c_text(&partition.label)),
        ssid: network.ssid.clone().unwrap_or_default(),
        rssi: network.rssi_dbm.unwrap_or_default(),
        channel: 0,
        ip: network.ipv4_address.clone().unwrap_or_default(),
    }
}

/// A NUL-terminated C string field as text.
fn c_text(chars: &[core::ffi::c_char]) -> String {
    if !chars.contains(&0) {
        return String::new();
    }
    unsafe { CStr::from_ptr(chars.as_ptr()) }
        .to_string_lossy()
        .into_owned()
}

fn partitions() -> Vec<Partition> {
    let mut list = Vec::new();
    let mut iterator = unsafe {
        sys::esp_partition_find(
            sys::esp_partition_type_t_ESP_PARTITION_TYPE_ANY,
            sys::esp_partition_subtype_t_ESP_PARTITION_SUBTYPE_ANY,
            core::ptr::null(),
        )
    };
    while !iterator.is_null() {
        if let Some(partition) = unsafe { sys::esp_partition_get(iterator).as_ref() } {
            list.push(Partition {
                label: c_text(&partition.label),
                kind: partition.type_ as u32,
                subtype: partition.subtype as u32,
                address: partition.address,
                size: partition.size,
            });
        }
        // The last step releases the iterator and returns null.
        iterator = unsafe { sys::esp_partition_next(iterator) };
    }
    list
}

/// Streaming needs the radio awake; between calls it may doze again.
fn set_wifi_power_save(on: bool) {
    let mode = if on {
        sys::wifi_ps_type_t_WIFI_PS_MIN_MODEM
    } else {
        sys::wifi_ps_type_t_WIFI_PS_NONE
    };
    unsafe { sys::esp_wifi_set_ps(mode) };
}

fn run_call<A: CallAudio>(
    slot: &Mutex<Option<A>>,
    setup: &CallSetup,
    commands: &Receiver<CallCommand>,
    events: &Sender<CallEvent>,
) {
    info!("rustmix-wave=xiaozhi status=starting ota={}", setup.ota_url);
    set_wifi_power_save(false);
    let outcome = converse(slot, setup, commands, events);
    set_wifi_power_save(true);
    let error = outcome.err().map(|error| {
        warn!("rustmix-wave=xiaozhi status=failed error={error:#}");
        error.to_string()
    });
    if error.is_none() {
        info!("rustmix-wave=xiaozhi status=ended");
    }
    let _ = events.send(CallEvent::Phase(CallPhase::Ended { error }));
}

fn converse<A: CallAudio>(
    slot: &Mutex<Option<A>>,
    setup: &CallSetup,
    commands: &Receiver<CallCommand>,
    events: &Sender<CallEvent>,
) -> Result<()> {
    let show = |event: CallEvent| {
        let _ = events.send(event);
    };
    show(CallEvent::Phase(CallPhase::Connecting));
    let reply = check_in(setup).context("Could not reach the XiaoZhi server.")?;
    if let Some(activation) = &reply.activation {
        info!("rustmix-wave=xiaozhi status=linking");
        show(CallEvent::Phase(CallPhase::Linking {
            code: activation.code.clone(),
        }));
        if !wait_for_link(setup, activation, commands)? {
            return Ok(());
        }
        show(CallEvent::Phase(CallPhase::Connecting));
    }
    show(CallEvent::Linked);
    let websocket = reply
        .websocket
        .context("The XiaoZhi server sent no conversation address.")?;
    let (inbox_sender, inbox) = mpsc::channel();
    let headers = activation::websocket_headers(&websocket, &setup.device);
    let client = WsClient::connect(&websocket.url, &headers, inbox_sender)
        .context("Could not open the conversation.")?;
    let Some(hello) = handshake(&client, &inbox, &websocket, commands)? else {
        return Ok(());
    };
    info!(
        "rustmix-wave=xiaozhi status=connected version={} frame-ms={}",
        websocket.version, hello.frame_ms
    );
    // A hang-up may have taken the speaker back already.
    let Some(mut audio) = lock(slot).take() else {
        return Ok(());
    };
    let outcome = (|| {
        let mut talk = Talk {
            encoder: OpusEncoder::new().context("The microphone encoder did not start.")?,
            decoder: OpusDecoder::new(hello.frame_ms)
                .context("The speech decoder did not start.")?,
            audio: &mut audio,
            setup,
            client: &client,
            events,
            session: hello.session_id,
            version: websocket.version,
            conversation: Conversation::default(),
            playback: VecDeque::new(),
            frame: Vec::new(),
            mono: vec![0; MIC_CHUNK_BYTES],
            stereo: vec![0; STEREO_BYTES],
            packet: Vec::new(),
            pcm: Vec::new(),
            started: Instant::now(),
            last_heard: Instant::now(),
        };
        talk.run(&inbox, commands)
    })();
    audio.end_call();
    *lock(slot) = Some(audio);
    outcome
}

/// POST the system information to the OTA server.
fn check_in(setup: &CallSetup) -> Result<OtaReply> {
    let body = activation::system_info_json(&setup.device);
    let (status, reply) = post(&setup.ota_url, &setup.device, &body, OTA_TIMEOUT)?;
    ensure!(status == 200, "The XiaoZhi server answered {status}.");
    activation::parse_ota_reply(&reply)
        .map_err(|error| anyhow!("The XiaoZhi server's reply was not understood: {error}"))
}

/// Wait while the owner types the code at xiaozhi.me. `false` when the call
/// was ended first.
fn wait_for_link(
    setup: &CallSetup,
    activation: &Activation,
    commands: &Receiver<CallCommand>,
) -> Result<bool> {
    let url = activation::activate_url(&setup.ota_url);
    let timeout = activation
        .timeout_ms
        .map_or(ACTIVATE_TIMEOUT, |ms| {
            Duration::from_millis(ms) + OTA_TIMEOUT
        })
        .min(ACTIVATE_TIMEOUT);
    let deadline = Instant::now() + LINK_TIMEOUT;
    while Instant::now() < deadline {
        if hung_up(commands) {
            return Ok(false);
        }
        let wait = match post(&url, &setup.device, "{}", timeout) {
            Ok((200, _)) => {
                info!("rustmix-wave=xiaozhi status=linked");
                return Ok(true);
            }
            Ok((202, _)) => Duration::from_secs(3),
            Ok((status, body)) => {
                warn!("rustmix-wave=xiaozhi status=activate-refused http={status} body={body}");
                Duration::from_secs(10)
            }
            Err(error) => {
                warn!("rustmix-wave=xiaozhi status=activate-failed error={error:#}");
                Duration::from_secs(10)
            }
        };
        if !pause(wait, commands) {
            return Ok(false);
        }
    }
    Err(anyhow!("The device was not linked within ten minutes."))
}

/// Sleep, unless the call ends first; `false` then.
fn pause(wait: Duration, commands: &Receiver<CallCommand>) -> bool {
    let until = Instant::now() + wait;
    while Instant::now() < until {
        if hung_up(commands) {
            return false;
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    true
}

/// A hang-up among the queued keys; the others do nothing before the call.
fn hung_up(commands: &Receiver<CallCommand>) -> bool {
    loop {
        match commands.try_recv() {
            Ok(CallCommand::HangUp) | Err(TryRecvError::Disconnected) => return true,
            Ok(_) => {}
            Err(TryRecvError::Empty) => return false,
        }
    }
}

fn post(url: &str, device: &DeviceInfo, body: &str, timeout: Duration) -> Result<(u16, String)> {
    let config = Configuration {
        crt_bundle_attach: Some(sys::esp_crt_bundle_attach),
        timeout: Some(timeout),
        buffer_size: Some(2048),
        buffer_size_tx: Some(1024),
        ..Default::default()
    };
    let mut connection = EspHttpConnection::new(&config).context("HTTP client")?;
    let length = body.len().to_string();
    let named = activation::request_headers(device);
    let mut headers: Vec<(&str, &str)> = named
        .iter()
        .map(|(name, value)| (*name, value.as_str()))
        .collect();
    headers.push(("Content-Length", &length));
    connection.initiate_request(Method::Post, url, &headers)?;
    connection.write_all(body.as_bytes())?;
    connection.initiate_response()?;
    let status = connection.status();
    let mut reply = Vec::new();
    let mut buffer = [0_u8; 512];
    loop {
        let read = connection.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        ensure!(
            reply.len() + read <= REPLY_LIMIT_BYTES,
            "The XiaoZhi server's reply is too large."
        );
        reply.extend_from_slice(&buffer[..read]);
    }
    Ok((status, String::from_utf8_lossy(&reply).into_owned()))
}

struct ServerHello {
    session_id: String,
    frame_ms: u32,
}

/// Wait for the WebSocket, say hello and wait for the server's hello.
/// `None` when the call ended first.
fn handshake(
    client: &WsClient,
    inbox: &Receiver<WsEvent>,
    websocket: &WebSocketConfig,
    commands: &Receiver<CallCommand>,
) -> Result<Option<ServerHello>> {
    let deadline = Instant::now() + HANDSHAKE_TIMEOUT;
    let mut connected = false;
    while Instant::now() < deadline {
        if hung_up(commands) {
            return Ok(None);
        }
        match inbox.recv_timeout(Duration::from_millis(100)) {
            Ok(WsEvent::Connected) => {
                connected = true;
                let hello = DeviceMessage::Hello {
                    version: websocket.version,
                };
                client.send_text(&hello.to_json(""))?;
            }
            Ok(WsEvent::Text(text)) => {
                if let Ok(ServerMessage::Hello {
                    session_id,
                    audio_params,
                }) = ServerMessage::from_json(&text)
                {
                    return Ok(Some(ServerHello {
                        session_id: session_id.unwrap_or_default(),
                        frame_ms: audio_params.map_or(60, |params| params.frame_duration_ms),
                    }));
                }
            }
            Ok(WsEvent::Closed(reason)) => {
                return Err(anyhow!(
                    "The XiaoZhi server closed the conversation ({reason})."
                ));
            }
            Ok(WsEvent::Binary(_)) | Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => {
                return Err(anyhow!("The conversation closed."));
            }
        }
    }
    if connected {
        Err(anyhow!("The XiaoZhi server did not answer."))
    } else {
        Err(anyhow!("The conversation did not open."))
    }
}

/// The open conversation: microphone to server, server to speaker.
struct Talk<'a, A: CallAudio> {
    audio: &'a mut A,
    setup: &'a CallSetup,
    client: &'a WsClient,
    events: &'a Sender<CallEvent>,
    session: String,
    version: u16,
    conversation: Conversation,
    encoder: OpusEncoder,
    decoder: OpusDecoder,
    playback: VecDeque<Vec<u8>>,
    /// Microphone audio gathered for the next Opus frame.
    frame: Vec<u8>,
    mono: Vec<u8>,
    stereo: Vec<u8>,
    packet: Vec<u8>,
    pcm: Vec<u8>,
    started: Instant,
    last_heard: Instant,
}

impl<A: CallAudio> Talk<'_, A> {
    fn run(&mut self, inbox: &Receiver<WsEvent>, commands: &Receiver<CallCommand>) -> Result<()> {
        self.audio.start_call()?;
        self.show(CallEvent::Volume(self.audio.call_volume()));
        let effects = self.conversation.start();
        self.apply(effects)?;
        loop {
            loop {
                let effects = match commands.try_recv() {
                    Ok(CallCommand::HangUp) | Err(TryRecvError::Disconnected) => return Ok(()),
                    Ok(CallCommand::Interrupt) => self.conversation.interrupt(),
                    Ok(CallCommand::ToggleMute) => self.conversation.toggle_mute(),
                    Ok(CallCommand::Volume(percent)) => {
                        self.audio.set_call_volume(percent)?;
                        vec![Effect::Show(CallEvent::Volume(self.audio.call_volume()))]
                    }
                    Err(TryRecvError::Empty) => break,
                };
                self.apply(effects)?;
            }
            loop {
                match inbox.try_recv() {
                    Ok(WsEvent::Text(text)) => self.on_text(&text)?,
                    Ok(WsEvent::Binary(bytes)) => self.on_audio(bytes),
                    Ok(WsEvent::Connected) => {}
                    Ok(WsEvent::Closed(reason)) => {
                        info!("rustmix-wave=xiaozhi status=closed-by-server reason={reason}");
                        return Ok(());
                    }
                    Err(TryRecvError::Empty) => break,
                    Err(TryRecvError::Disconnected) => {
                        return Err(anyhow!("The conversation closed."));
                    }
                }
            }
            if let Some(packet) = self.playback.pop_front() {
                self.speak(&packet)?;
                continue;
            }
            let effects = self.conversation.on_playback_idle();
            self.apply(effects)?;
            if self.conversation.sends_audio() {
                self.listen()?;
            } else {
                std::thread::sleep(Duration::from_millis(20));
            }
            if self.last_heard.elapsed() >= QUIET_LIMIT {
                info!("rustmix-wave=xiaozhi status=quiet-end");
                return Ok(());
            }
        }
    }

    fn show(&self, event: CallEvent) {
        let _ = self.events.send(event);
    }

    fn apply(&mut self, effects: Vec<Effect>) -> Result<()> {
        for effect in effects {
            match effect {
                Effect::Send(message) => self.client.send_text(&message.to_json(&self.session))?,
                Effect::Speaker(on) => self.audio.set_speaker(on)?,
                Effect::FlushMic => {
                    self.audio.flush_mic(&mut self.stereo)?;
                    self.frame.clear();
                }
                Effect::ClearPlayback => self.playback.clear(),
                Effect::Show(event) => self.show(event),
            }
        }
        Ok(())
    }

    fn on_text(&mut self, text: &str) -> Result<()> {
        self.last_heard = Instant::now();
        match ServerMessage::from_json(text) {
            Ok(ServerMessage::Mcp { payload }) => {
                let mut device = CallDevice {
                    audio: &mut *self.audio,
                    setup: self.setup,
                };
                let outcome = mcp::handle(&payload, &mut device);
                if let Some(reply) = outcome.reply {
                    let message = DeviceMessage::Mcp { payload: reply };
                    self.client.send_text(&message.to_json(&self.session))?;
                }
                if let Some(tool) = outcome.tool {
                    info!("rustmix-wave=xiaozhi status=tool tool={tool}");
                    self.show(CallEvent::Tool(tool));
                    self.show(CallEvent::Volume(self.audio.call_volume()));
                }
            }
            Ok(message) => {
                let effects = self.conversation.on_message(&message);
                self.apply(effects)?;
            }
            Err(error) => warn!("rustmix-wave=xiaozhi status=bad-message error={error}"),
        }
        Ok(())
    }

    fn on_audio(&mut self, bytes: Vec<u8>) {
        self.last_heard = Instant::now();
        if !self.conversation.accepts_audio() {
            return;
        }
        let payload = match self.version {
            2 => protocol::decode_v2(&bytes).map(|frame| (frame.kind, frame.payload)),
            3 => protocol::decode_v3(&bytes).map(|frame| (frame.kind, frame.payload)),
            _ => Ok((FrameType::Opus, bytes)),
        };
        match payload {
            Ok((FrameType::Opus, payload)) if self.playback.len() < PLAYBACK_LIMIT => {
                self.playback.push_back(payload);
            }
            Ok((FrameType::Opus, _)) => warn!("rustmix-wave=xiaozhi status=speech-dropped"),
            Ok((FrameType::Json, _)) => {}
            Err(error) => warn!("rustmix-wave=xiaozhi status=bad-audio error={error}"),
        }
    }

    /// Decode one packet and queue it for the speaker; the queue paces this.
    fn speak(&mut self, packet: &[u8]) -> Result<()> {
        if let Err(error) = self.decoder.decode(packet, &mut self.pcm) {
            warn!("rustmix-wave=xiaozhi status=decode-failed error={error:#}");
            return Ok(());
        }
        for part in self.pcm.chunks(STEREO_BYTES / 2) {
            self.audio.play(part, &mut self.stereo)?;
        }
        Ok(())
    }

    /// Gather microphone audio; each full frame goes out as one packet.
    fn listen(&mut self) -> Result<()> {
        let room = self.encoder.frame_bytes().saturating_sub(self.frame.len());
        let want = room.min(MIC_CHUNK_BYTES);
        let read = self.audio.read_mic(
            &mut self.stereo[..want * 2],
            &mut self.mono[..want],
            self.setup.mic_gain,
            20,
        )?;
        self.frame.extend_from_slice(&self.mono[..read]);
        if self.frame.len() < self.encoder.frame_bytes() {
            return Ok(());
        }
        self.encoder.encode(&self.frame, &mut self.packet)?;
        self.frame.clear();
        let timestamp = u32::try_from(self.started.elapsed().as_millis()).unwrap_or(u32::MAX);
        let message = match self.version {
            2 => protocol::encode_v2(FrameType::Opus, timestamp, &self.packet),
            3 => protocol::encode_v3(FrameType::Opus, &self.packet),
            _ => self.packet.clone(),
        };
        self.client.send_binary(&message)
    }
}

/// The device as MCP tools see it during a call.
struct CallDevice<'a, A: CallAudio> {
    audio: &'a mut A,
    setup: &'a CallSetup,
}

impl<A: CallAudio> McpDevice for CallDevice<'_, A> {
    fn status(&self) -> String {
        mcp::status_json(
            self.audio.call_volume(),
            self.setup.battery_percent,
            &self.setup.device.ssid,
            self.setup.rssi,
        )
    }

    fn set_volume(&mut self, percent: u8) -> Result<(), String> {
        self.audio
            .set_call_volume(percent)
            .map_err(|error| format!("{error:#}"))
    }

    fn read_bible(&mut self, request: &BibleRequest) -> Result<String, String> {
        let translation = self.setup.bible_translation.as_deref();
        mcp::bible_passage(&self.setup.bible_root, translation, request)
            .map_err(|error| format!("{error:#}"))
    }
}

/// What the WebSocket task hands the call thread.
enum WsEvent {
    Connected,
    Text(String),
    Binary(Vec<u8>),
    Closed(&'static str),
}

/// Lives on the heap for the WebSocket task; it joins the parts of a
/// message that arrive in several events.
struct WsInbox {
    sender: Sender<WsEvent>,
    message: Vec<u8>,
    text: bool,
}

impl WsInbox {
    fn send(&self, event: WsEvent) {
        let _ = self.sender.send(event);
    }

    fn data(&mut self, data: &sys::esp_websocket_event_data_t) {
        // Ping, pong and close frames are the client's business.
        if data.op_code > 2 {
            return;
        }
        let length = usize::try_from(data.data_len).unwrap_or(0);
        let bytes = if length == 0 || data.data_ptr.is_null() {
            &[][..]
        } else {
            unsafe { core::slice::from_raw_parts(data.data_ptr.cast::<u8>(), length) }
        };
        if data.op_code != 0 && data.payload_offset == 0 {
            self.message.clear();
            self.text = data.op_code == 1;
        }
        if self.message.len() + bytes.len() > WS_MESSAGE_LIMIT {
            self.message.clear();
            return;
        }
        self.message.extend_from_slice(bytes);
        let frame_done = data.payload_offset + data.data_len >= data.payload_len;
        if frame_done && data.fin {
            let message = core::mem::take(&mut self.message);
            self.send(if self.text {
                WsEvent::Text(String::from_utf8_lossy(&message).into_owned())
            } else {
                WsEvent::Binary(message)
            });
        }
    }
}

unsafe extern "C" fn on_ws_event(
    arg: *mut c_void,
    _base: sys::esp_event_base_t,
    id: i32,
    data: *mut c_void,
) {
    let Some(inbox) = arg.cast::<WsInbox>().as_mut() else {
        return;
    };
    match id {
        sys::esp_websocket_event_id_t_WEBSOCKET_EVENT_CONNECTED => {
            inbox.send(WsEvent::Connected);
        }
        sys::esp_websocket_event_id_t_WEBSOCKET_EVENT_DISCONNECTED => {
            inbox.send(WsEvent::Closed("disconnected"));
        }
        sys::esp_websocket_event_id_t_WEBSOCKET_EVENT_CLOSED => {
            inbox.send(WsEvent::Closed("closed"));
        }
        sys::esp_websocket_event_id_t_WEBSOCKET_EVENT_DATA => {
            if let Some(data) = data.cast::<sys::esp_websocket_event_data_t>().as_ref() {
                inbox.data(data);
            }
        }
        _ => {}
    }
}

/// ESP-IDF's WebSocket client, used through its C API so that closing a
/// dropped connection is never an error.
struct WsClient {
    handle: sys::esp_websocket_client_handle_t,
    inbox: *mut WsInbox,
    _uri: CString,
    _headers: CString,
}

impl WsClient {
    fn connect(url: &str, headers: &str, sender: Sender<WsEvent>) -> Result<Self> {
        let uri = CString::new(url).context("the conversation address")?;
        let headers = CString::new(headers).context("the conversation headers")?;
        let config = sys::esp_websocket_client_config_t {
            uri: uri.as_ptr(),
            headers: headers.as_ptr(),
            buffer_size: 4096,
            task_stack: 8192,
            network_timeout_ms: 10_000,
            disable_auto_reconnect: true,
            crt_bundle_attach: Some(sys::esp_crt_bundle_attach),
            ..Default::default()
        };
        let handle = unsafe { sys::esp_websocket_client_init(&config) };
        ensure!(!handle.is_null(), "the WebSocket client did not start");
        let inbox = Box::into_raw(Box::new(WsInbox {
            sender,
            message: Vec::new(),
            text: false,
        }));
        let client = Self {
            handle,
            inbox,
            _uri: uri,
            _headers: headers,
        };
        esp!(unsafe {
            sys::esp_websocket_register_events(
                handle,
                sys::esp_websocket_event_id_t_WEBSOCKET_EVENT_ANY,
                Some(on_ws_event),
                inbox.cast(),
            )
        })?;
        esp!(unsafe { sys::esp_websocket_client_start(handle) })?;
        Ok(client)
    }

    fn send_text(&self, text: &str) -> Result<()> {
        let sent = unsafe {
            sys::esp_websocket_client_send_text(
                self.handle,
                text.as_ptr().cast(),
                i32::try_from(text.len())?,
                TickType::new_millis(WS_SEND_TIMEOUT_MS).ticks(),
            )
        };
        ensure!(sent >= 0, "sending to the XiaoZhi server failed");
        Ok(())
    }

    fn send_binary(&self, data: &[u8]) -> Result<()> {
        let sent = unsafe {
            sys::esp_websocket_client_send_bin(
                self.handle,
                data.as_ptr().cast(),
                i32::try_from(data.len())?,
                TickType::new_millis(WS_SEND_TIMEOUT_MS).ticks(),
            )
        };
        ensure!(sent >= 0, "sending audio to the XiaoZhi server failed");
        Ok(())
    }
}

impl Drop for WsClient {
    fn drop(&mut self) {
        unsafe {
            if sys::esp_websocket_client_is_connected(self.handle) {
                sys::esp_websocket_client_close(self.handle, TickType::new_millis(2_000).ticks());
            }
            // Destroy stops the client task, so the inbox is free to go.
            sys::esp_websocket_client_destroy(self.handle);
            drop(Box::from_raw(self.inbox));
        }
    }
}

/// The microphone's Opus encoder: 16 kHz mono, 60 ms frames, settings of
/// upstream's audio service.
struct OpusEncoder {
    handle: *mut c_void,
    frame_bytes: usize,
    packet_bytes: usize,
}

impl OpusEncoder {
    fn new() -> Result<Self> {
        let mut config: codec::esp_opus_enc_config_t = unsafe { core::mem::zeroed() };
        config.sample_rate = 16_000;
        config.channel = 1;
        config.bits_per_sample = 16;
        // ESP_OPUS_BITRATE_AUTO
        config.bitrate = -1000;
        config.frame_duration =
            codec::esp_opus_enc_frame_duration_t_ESP_OPUS_ENC_FRAME_DURATION_60_MS;
        config.application_mode = codec::esp_opus_enc_application_t_ESP_OPUS_ENC_APPLICATION_AUDIO;
        config.complexity = 0;
        config.enable_dtx = true;
        config.enable_vbr = true;
        let mut handle = core::ptr::null_mut();
        let size = u32::try_from(core::mem::size_of::<codec::esp_opus_enc_config_t>())?;
        let opened = unsafe {
            codec::esp_opus_enc_open(
                (&mut config as *mut codec::esp_opus_enc_config_t).cast(),
                size,
                &mut handle,
            )
        };
        ensure!(
            opened == 0 && !handle.is_null(),
            "Opus encoder error {opened}"
        );
        let mut encoder = Self {
            handle,
            frame_bytes: 0,
            packet_bytes: 0,
        };
        let (mut frame_bytes, mut packet_bytes) = (0, 0);
        unsafe { codec::esp_opus_enc_get_frame_size(handle, &mut frame_bytes, &mut packet_bytes) };
        encoder.frame_bytes = usize::try_from(frame_bytes).unwrap_or(0);
        encoder.packet_bytes = usize::try_from(packet_bytes).unwrap_or(0).max(256);
        ensure!(encoder.frame_bytes > 0, "Opus encoder has no frame size");
        Ok(encoder)
    }

    /// Bytes of mono PCM16 in one frame.
    const fn frame_bytes(&self) -> usize {
        self.frame_bytes
    }

    fn encode(&mut self, pcm: &[u8], packet: &mut Vec<u8>) -> Result<()> {
        packet.resize(self.packet_bytes, 0);
        let mut input: codec::esp_audio_enc_in_frame_t = unsafe { core::mem::zeroed() };
        input.buffer = pcm.as_ptr().cast_mut();
        input.len = u32::try_from(pcm.len())?;
        let mut output: codec::esp_audio_enc_out_frame_t = unsafe { core::mem::zeroed() };
        output.buffer = packet.as_mut_ptr();
        output.len = u32::try_from(packet.len())?;
        let encoded = unsafe { codec::esp_opus_enc_process(self.handle, &mut input, &mut output) };
        ensure!(encoded == 0, "Opus encoding error {encoded}");
        packet.truncate(output.encoded_bytes as usize);
        Ok(())
    }
}

impl Drop for OpusEncoder {
    fn drop(&mut self) {
        unsafe { codec::esp_opus_enc_close(self.handle) };
    }
}

/// The speech decoder, at the speaker's 16 kHz whatever rate the server
/// encodes at: Opus decodes to any of its rates.
struct OpusDecoder {
    handle: *mut c_void,
}

impl OpusDecoder {
    fn new(frame_ms: u32) -> Result<Self> {
        let mut config: codec::esp_opus_dec_cfg_t = unsafe { core::mem::zeroed() };
        config.sample_rate = crate::audio::AUDIO_SAMPLE_RATE_HZ;
        config.channel = 1;
        config.frame_duration = match frame_ms {
            20 => codec::esp_opus_dec_frame_duration_t_ESP_OPUS_DEC_FRAME_DURATION_20_MS,
            40 => codec::esp_opus_dec_frame_duration_t_ESP_OPUS_DEC_FRAME_DURATION_40_MS,
            60 => codec::esp_opus_dec_frame_duration_t_ESP_OPUS_DEC_FRAME_DURATION_60_MS,
            120 => codec::esp_opus_dec_frame_duration_t_ESP_OPUS_DEC_FRAME_DURATION_120_MS,
            // Counted as 60 ms by the decoder.
            _ => codec::esp_opus_dec_frame_duration_t_ESP_OPUS_DEC_FRAME_DURATION_INVALID,
        };
        config.self_delimited = false;
        let mut handle = core::ptr::null_mut();
        let size = u32::try_from(core::mem::size_of::<codec::esp_opus_dec_cfg_t>())?;
        let opened = unsafe {
            codec::esp_opus_dec_open(
                (&mut config as *mut codec::esp_opus_dec_cfg_t).cast(),
                size,
                &mut handle,
            )
        };
        ensure!(
            opened == 0 && !handle.is_null(),
            "Opus decoder error {opened}"
        );
        Ok(Self { handle })
    }

    fn decode(&mut self, packet: &[u8], pcm: &mut Vec<u8>) -> Result<()> {
        pcm.resize(DECODE_BYTES, 0);
        let mut raw: codec::esp_audio_dec_in_raw_t = unsafe { core::mem::zeroed() };
        raw.buffer = packet.as_ptr().cast_mut();
        raw.len = u32::try_from(packet.len())?;
        let mut frame: codec::esp_audio_dec_out_frame_t = unsafe { core::mem::zeroed() };
        frame.buffer = pcm.as_mut_ptr();
        frame.len = u32::try_from(pcm.len())?;
        let mut info: codec::esp_audio_dec_info_t = unsafe { core::mem::zeroed() };
        let decoded =
            unsafe { codec::esp_opus_dec_decode(self.handle, &mut raw, &mut frame, &mut info) };
        ensure!(decoded == 0, "Opus decoding error {decoded}");
        pcm.truncate(frame.decoded_size as usize);
        Ok(())
    }
}

impl Drop for OpusDecoder {
    fn drop(&mut self) {
        unsafe { codec::esp_opus_dec_close(self.handle) };
    }
}
