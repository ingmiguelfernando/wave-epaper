//! A XiaoZhi call: the conversation's turns as the call thread runs them,
//! and the screen's copy of the call. Audio and network live in `espidf`.

use super::protocol::{DeviceMessage, ListenMode, ListenState, ServerMessage, TtsState};

/// The reply text kept for the screen; older sentences drop off the front.
const SAID_LIMIT_CHARS: usize = 600;
/// Volume steps of ▲▼ during a call.
pub const VOLUME_STEP: u8 = 10;

/// Where the call is.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub enum CallPhase {
    /// No call yet.
    #[default]
    Idle,
    /// Wi-Fi, the OTA check and the WebSocket handshake.
    Connecting,
    /// The device is not linked to an account: type the code at xiaozhi.me.
    Linking {
        code: String,
    },
    Listening,
    /// The server heard a question and is preparing the reply.
    Thinking,
    Speaking,
    /// The call ended; an error says why it could not go on.
    Ended {
        error: Option<String>,
    },
}

impl CallPhase {
    /// From the request to the end of the call.
    #[must_use]
    pub const fn is_live(&self) -> bool {
        !matches!(self, Self::Idle | Self::Ended { .. })
    }

    /// The conversation is open: listening, thinking or speaking.
    #[must_use]
    pub const fn is_talking(&self) -> bool {
        matches!(self, Self::Listening | Self::Thinking | Self::Speaking)
    }
}

/// What the screen asks of the call thread.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CallCommand {
    /// ● while XiaoZhi speaks: stop the reply.
    Interrupt,
    /// Short BOOT: the microphone off, or on again.
    ToggleMute,
    /// ▲▼: the speaker volume, 0 to 100.
    Volume(u8),
    /// Hold BOOT: end the call.
    HangUp,
}

/// What the call thread tells the screen.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CallEvent {
    Phase(CallPhase),
    /// What the server heard.
    Heard(String),
    /// A sentence of XiaoZhi's reply.
    Said(String),
    /// A device tool XiaoZhi used, such as `bible.read(PSA 23)`.
    Tool(String),
    Muted(bool),
    Volume(u8),
    /// The device is linked to an account.
    Linked,
}

/// One step the call thread takes for the conversation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Effect {
    Send(DeviceMessage),
    Speaker(bool),
    /// Drop microphone audio captured while XiaoZhi spoke.
    FlushMic,
    /// Drop queued speech after an interruption.
    ClearPlayback,
    Show(CallEvent),
}

/// The conversation in auto mode, as upstream runs it without echo
/// cancellation: listen until the server's voice detection ends the turn,
/// play the reply with the microphone closed, then listen again.
#[derive(Clone, Debug, Default)]
pub struct Conversation {
    phase: CallPhase,
    muted: bool,
    /// An interrupted reply plays no more audio until `tts stop`.
    aborted: bool,
    /// The reply ended; listening starts once its audio has played.
    listen_when_drained: bool,
}

impl Conversation {
    #[must_use]
    pub const fn phase(&self) -> &CallPhase {
        &self.phase
    }

    /// The server said hello: start listening.
    pub fn start(&mut self) -> Vec<Effect> {
        self.listen()
    }

    fn listen(&mut self) -> Vec<Effect> {
        self.phase = CallPhase::Listening;
        self.listen_when_drained = false;
        vec![
            Effect::Speaker(false),
            Effect::FlushMic,
            Effect::Send(DeviceMessage::Listen {
                state: ListenState::Start,
                mode: Some(ListenMode::Auto),
                text: None,
            }),
            Effect::Show(CallEvent::Phase(CallPhase::Listening)),
        ]
    }

    /// A text message from the server.
    pub fn on_message(&mut self, message: &ServerMessage) -> Vec<Effect> {
        match message {
            ServerMessage::Stt { text } => {
                let mut effects = Vec::new();
                if !text.trim().is_empty() {
                    effects.push(Effect::Show(CallEvent::Heard(text.clone())));
                }
                if self.phase == CallPhase::Listening {
                    self.phase = CallPhase::Thinking;
                    effects.push(Effect::Show(CallEvent::Phase(CallPhase::Thinking)));
                }
                effects
            }
            ServerMessage::Tts {
                state: TtsState::Start,
                ..
            } => {
                self.aborted = false;
                self.listen_when_drained = false;
                if self.phase == CallPhase::Speaking {
                    return Vec::new();
                }
                self.phase = CallPhase::Speaking;
                vec![
                    Effect::Speaker(true),
                    Effect::Show(CallEvent::Phase(CallPhase::Speaking)),
                ]
            }
            ServerMessage::Tts {
                state: TtsState::SentenceStart,
                text: Some(text),
            } if !self.aborted && !text.trim().is_empty() => {
                vec![Effect::Show(CallEvent::Said(text.clone()))]
            }
            ServerMessage::Tts {
                state: TtsState::Stop,
                ..
            } if self.phase == CallPhase::Speaking => {
                self.listen_when_drained = true;
                Vec::new()
            }
            _ => Vec::new(),
        }
    }

    /// Speech from the server plays only while XiaoZhi speaks.
    #[must_use]
    pub fn accepts_audio(&self) -> bool {
        self.phase == CallPhase::Speaking && !self.aborted
    }

    /// Microphone audio goes out while listening or thinking, unless muted.
    #[must_use]
    pub fn sends_audio(&self) -> bool {
        matches!(self.phase, CallPhase::Listening | CallPhase::Thinking) && !self.muted
    }

    /// The speech queue ran dry: after a finished reply, listen again.
    pub fn on_playback_idle(&mut self) -> Vec<Effect> {
        if self.listen_when_drained {
            self.listen()
        } else {
            Vec::new()
        }
    }

    /// ● while XiaoZhi speaks stops the reply; the server confirms with
    /// `tts stop`, and listening starts again.
    pub fn interrupt(&mut self) -> Vec<Effect> {
        if self.phase != CallPhase::Speaking || self.aborted {
            return Vec::new();
        }
        self.aborted = true;
        vec![
            Effect::Send(DeviceMessage::Abort { reason: None }),
            Effect::ClearPlayback,
        ]
    }

    pub fn toggle_mute(&mut self) -> Vec<Effect> {
        self.muted = !self.muted;
        vec![Effect::Show(CallEvent::Muted(self.muted))]
    }
}

/// The call as the screen shows it, fed by the call thread's events. The
/// keys leave requests here for `main.rs`.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct XiaozhiUi {
    pub phase: CallPhase,
    /// The user's last question.
    pub heard: String,
    /// XiaoZhi's reply so far.
    pub said: String,
    /// The device tool the reply used.
    pub tool: Option<String>,
    pub muted: bool,
    pub volume: u8,
    /// Linked to an account at xiaozhi.me; kept in NVS between calls.
    pub linked: bool,
    /// Milliseconds since boot when the conversation opened.
    pub started_ms: Option<u64>,
    /// When the last event came; the call length shown runs to it.
    pub clock_ms: u64,
    start_requested: bool,
    commands: Vec<CallCommand>,
}

impl XiaozhiUi {
    /// Opening the screen, or ● after a call ended, asks for a new call.
    pub fn request_call(&mut self) {
        *self = Self {
            phase: CallPhase::Connecting,
            volume: self.volume,
            linked: self.linked,
            start_requested: true,
            ..Self::default()
        };
    }

    /// The pending request, once. The screen starts over for the new call,
    /// whatever the last events of a previous one left on it.
    #[must_use]
    pub fn take_start_request(&mut self) -> bool {
        if !self.start_requested {
            return false;
        }
        self.request_call();
        self.start_requested = false;
        true
    }

    /// A key's request for the running call.
    pub fn command(&mut self, command: CallCommand) {
        if self.phase.is_live() {
            self.commands.push(command);
        }
    }

    #[must_use]
    pub fn take_commands(&mut self) -> Vec<CallCommand> {
        core::mem::take(&mut self.commands)
    }

    /// ▲▼: the volume moves at once on screen; the call thread confirms it.
    pub fn step_volume(&mut self, up: bool) {
        if !self.phase.is_live() {
            return;
        }
        self.volume = if up {
            self.volume.saturating_add(VOLUME_STEP).min(100)
        } else {
            self.volume.saturating_sub(VOLUME_STEP)
        };
        self.command(CallCommand::Volume(self.volume));
    }

    /// Hold BOOT: end the call, or forget a request that has not started.
    pub fn hang_up(&mut self) {
        self.command(CallCommand::HangUp);
        self.start_requested = false;
        if self.phase.is_live() {
            self.phase = CallPhase::Ended { error: None };
        }
    }

    /// The call could not start; nothing runs.
    pub fn fail(&mut self, error: impl Into<String>) {
        self.start_requested = false;
        self.commands.clear();
        self.phase = CallPhase::Ended {
            error: Some(error.into()),
        };
    }

    pub fn apply(&mut self, event: CallEvent, now_ms: u64) {
        self.clock_ms = now_ms;
        match event {
            CallEvent::Phase(phase) => {
                if phase.is_talking() && self.started_ms.is_none() {
                    self.started_ms = Some(now_ms);
                }
                // A hang-up already ended the call on screen.
                if !matches!(self.phase, CallPhase::Ended { .. }) || !phase.is_live() {
                    self.phase = phase;
                }
            }
            CallEvent::Heard(text) => {
                self.heard = text;
                self.said.clear();
                self.tool = None;
            }
            CallEvent::Said(text) => {
                if !self.said.is_empty() {
                    self.said.push(' ');
                }
                self.said.push_str(text.trim());
                let excess = self.said.chars().count().saturating_sub(SAID_LIMIT_CHARS);
                if excess > 0 {
                    self.said = self.said.chars().skip(excess).collect();
                }
            }
            CallEvent::Tool(tool) => self.tool = Some(tool),
            CallEvent::Muted(muted) => self.muted = muted,
            CallEvent::Volume(volume) => self.volume = volume,
            CallEvent::Linked => self.linked = true,
        }
    }

    /// `02:14`, the call's length for the status bar.
    #[must_use]
    pub fn duration_label(&self, now_ms: u64) -> Option<String> {
        let started = self.started_ms?;
        let seconds = now_ms.saturating_sub(started) / 1000;
        Some(format!("{:02}:{:02}", seconds / 60, seconds % 60))
    }
}

#[cfg(test)]
mod tests {
    use super::{CallCommand, CallEvent, CallPhase, Conversation, Effect, XiaozhiUi};
    use crate::xiaozhi::protocol::{DeviceMessage, ListenState, ServerMessage, TtsState};

    fn tts(state: TtsState, text: Option<&str>) -> ServerMessage {
        ServerMessage::Tts {
            state,
            text: text.map(String::from),
        }
    }

    fn sends_listen(effects: &[Effect]) -> bool {
        effects.iter().any(|effect| {
            matches!(
                effect,
                Effect::Send(DeviceMessage::Listen {
                    state: ListenState::Start,
                    ..
                })
            )
        })
    }

    #[test]
    fn a_turn_listens_thinks_speaks_and_listens_again() {
        let mut call = Conversation::default();
        let effects = call.start();
        assert!(sends_listen(&effects));
        assert!(effects.contains(&Effect::Speaker(false)));
        assert!(call.sends_audio());
        let heard = call.on_message(&ServerMessage::Stt {
            text: "Read me Psalm 23".into(),
        });
        assert_eq!(call.phase(), &CallPhase::Thinking);
        assert!(heard.contains(&Effect::Show(CallEvent::Heard("Read me Psalm 23".into()))));
        let speaking = call.on_message(&tts(TtsState::Start, None));
        assert!(speaking.contains(&Effect::Speaker(true)));
        assert!(call.accepts_audio());
        assert!(
            !call.sends_audio(),
            "the microphone is closed while XiaoZhi speaks"
        );
        let sentence = tts(TtsState::SentenceStart, Some("The Lord is my shepherd."));
        let expected = Effect::Show(CallEvent::Said("The Lord is my shepherd.".into()));
        assert_eq!(call.on_message(&sentence), [expected]);
        // The reply ends, but its audio plays out before listening starts.
        assert!(call.on_message(&tts(TtsState::Stop, None)).is_empty());
        assert_eq!(call.phase(), &CallPhase::Speaking);
        let again = call.on_playback_idle();
        assert!(sends_listen(&again));
        assert!(again.contains(&Effect::FlushMic));
        assert_eq!(call.phase(), &CallPhase::Listening);
        // With nothing pending, an idle queue changes nothing.
        assert!(call.on_playback_idle().is_empty());
    }

    #[test]
    fn an_interruption_drops_the_reply_until_the_server_stops() {
        let mut call = Conversation::default();
        call.start();
        call.on_message(&tts(TtsState::Start, None));
        let effects = call.interrupt();
        assert_eq!(
            effects,
            [
                Effect::Send(DeviceMessage::Abort { reason: None }),
                Effect::ClearPlayback,
            ]
        );
        assert!(!call.accepts_audio());
        // A second press sends nothing more, and late sentences stay hidden.
        assert!(call.interrupt().is_empty());
        assert!(call
            .on_message(&tts(TtsState::SentenceStart, Some("more")))
            .is_empty());
        call.on_message(&tts(TtsState::Stop, None));
        assert!(sends_listen(&call.on_playback_idle()));
        // Nothing to interrupt while listening.
        assert!(call.interrupt().is_empty());
    }

    #[test]
    fn a_muted_microphone_sends_nothing() {
        let mut call = Conversation::default();
        call.start();
        assert_eq!(call.toggle_mute(), [Effect::Show(CallEvent::Muted(true))]);
        assert!(!call.sends_audio());
        call.toggle_mute();
        assert!(call.sends_audio());
    }

    #[test]
    fn the_screen_keeps_the_turn_and_the_call_length() {
        let mut ui = XiaozhiUi {
            volume: 60,
            linked: true,
            ..XiaozhiUi::default()
        };
        ui.request_call();
        assert!(ui.take_start_request());
        assert!(!ui.take_start_request());
        assert_eq!(ui.phase, CallPhase::Connecting);
        assert_eq!((ui.volume, ui.linked), (60, true));
        ui.apply(CallEvent::Phase(CallPhase::Listening), 5_000);
        ui.apply(CallEvent::Heard("Hola".into()), 6_000);
        ui.apply(CallEvent::Said("Hola.".into()), 7_000);
        ui.apply(CallEvent::Said("¿Qué tal?".into()), 7_500);
        assert_eq!(ui.said, "Hola. ¿Qué tal?");
        assert_eq!(ui.duration_label(139_000).as_deref(), Some("02:14"));
        // A new question starts a new reply.
        ui.apply(CallEvent::Heard("Otra".into()), 8_000);
        assert!(ui.said.is_empty());
        let long = "x".repeat(700);
        ui.apply(CallEvent::Said(long), 9_000);
        assert_eq!(ui.said.chars().count(), 600);
    }

    #[test]
    fn keys_queue_commands_only_during_a_call() {
        let mut ui = XiaozhiUi::default();
        ui.step_volume(true);
        assert!(ui.take_commands().is_empty());
        ui.request_call();
        ui.step_volume(true);
        ui.step_volume(true);
        assert_eq!(ui.volume, 20);
        assert_eq!(
            ui.take_commands(),
            [CallCommand::Volume(10), CallCommand::Volume(20)]
        );
        ui.hang_up();
        assert_eq!(ui.take_commands(), [CallCommand::HangUp]);
        assert_eq!(ui.phase, CallPhase::Ended { error: None });
        // The thread's last phases do not reopen a call the user ended.
        ui.apply(CallEvent::Phase(CallPhase::Listening), 1);
        assert_eq!(ui.phase, CallPhase::Ended { error: None });
        ui.apply(
            CallEvent::Phase(CallPhase::Ended {
                error: Some("closed".into()),
            }),
            2,
        );
        assert!(matches!(ui.phase, CallPhase::Ended { error: Some(_) }));
    }
}
