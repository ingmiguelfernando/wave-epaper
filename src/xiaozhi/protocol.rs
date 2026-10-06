//! XiaoZhi WebSocket protocol messages, ported from the xiaozhi-esp32
//! firmware (`docs/websocket.md` and `main/protocols/protocol.h` at commit
//! `af5a8c5`, "feature: support mqtt & udp", the doc's current base). Phase 7
//! adds WebSocket, Opus and the screens; this module models the messages.
//!
//! Text frames carry JSON dispatched by `type`; binary frames carry Opus
//! audio with versioned headers.

use std::format;
use std::string::String;
use std::vec::Vec;

use crate::json_lite::{self, JsonValue};

/// Opus frames the device streams: 16 kHz, mono, 60 ms.
pub const OPUS_SAMPLE_RATE: u32 = 16_000;
pub const OPUS_CHANNELS: u8 = 1;
pub const OPUS_FRAME_DURATION_MS: u32 = 60;

/// How the device listens, per `protocol.h`.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ListenMode {
    Auto,
    Manual,
    Realtime,
}

impl ListenMode {
    #[must_use]
    pub const fn marker(self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::Manual => "manual",
            Self::Realtime => "realtime",
        }
    }

    fn parse(value: &str) -> Option<Self> {
        match value {
            "auto" => Some(Self::Auto),
            "manual" => Some(Self::Manual),
            "realtime" => Some(Self::Realtime),
            _ => None,
        }
    }
}

/// Messages the device sends.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DeviceMessage {
    /// Handshake announcing version, transport and Opus parameters.
    Hello { version: u16 },
    /// Start or stop microphone capture, or a detected wake word.
    Listen {
        state: ListenState,
        mode: Option<ListenMode>,
        text: Option<String>,
    },
    /// Abort the current TTS playback or the voice channel.
    Abort { reason: String },
    /// IoT control: the JSON-RPC 2.0 payload passes through verbatim.
    Mcp { payload: String },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ListenState {
    Start,
    Stop,
    Detect,
}

impl ListenState {
    #[must_use]
    pub const fn marker(self) -> &'static str {
        match self {
            Self::Start => "start",
            Self::Stop => "stop",
            Self::Detect => "detect",
        }
    }
}

impl DeviceMessage {
    /// Serialize to the JSON text frame; `session_id` rides on every message
    /// except the hello handshake.
    #[must_use]
    pub fn to_json(&self, session_id: &str) -> String {
        match self {
            Self::Hello { version } => format!(
                "{{\"type\":\"hello\",\"version\":{version},\"features\":{{\"mcp\":true}},\
\"transport\":\"websocket\",\"audio_params\":{{\"format\":\"opus\",\
\"sample_rate\":{OPUS_SAMPLE_RATE},\"channels\":{OPUS_CHANNELS},\
\"frame_duration\":{OPUS_FRAME_DURATION_MS}}}}}"
            ),
            Self::Listen { state, mode, text } => {
                let mut out = format!(
                    "{{\"session_id\":\"{}\",\"type\":\"listen\",\"state\":\"{}\"",
                    json_lite::escape(session_id),
                    state.marker()
                );
                if let Some(mode) = mode {
                    out.push_str(&format!(",\"mode\":\"{}\"", mode.marker()));
                }
                if let Some(text) = text {
                    out.push_str(&format!(",\"text\":\"{}\"", json_lite::escape(text)));
                }
                out.push('}');
                out
            }
            Self::Abort { reason } => format!(
                "{{\"session_id\":\"{}\",\"type\":\"abort\",\"reason\":\"{}\"}}",
                json_lite::escape(session_id),
                json_lite::escape(reason)
            ),
            Self::Mcp { payload } => format!(
                "{{\"session_id\":\"{}\",\"type\":\"mcp\",\"payload\":{}}}",
                json_lite::escape(session_id),
                payload
            ),
        }
    }
}

/// Audio parameters the server hello agrees on.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AudioParams {
    pub sample_rate: u32,
    pub channels: u8,
    pub frame_duration_ms: u32,
}

/// Messages the server sends; anything unknown arrives as `Unknown`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ServerMessage {
    /// Handshake acknowledgement with the agreed session and audio.
    Hello {
        session_id: Option<String>,
        audio_params: Option<AudioParams>,
    },
    /// The speech-to-text result for the user utterance.
    Stt { text: String },
    /// Emotion update for the UI, with the emoji to show.
    Llm { emotion: String, text: String },
    /// TTS state transitions; `sentence_start` carries the subtitle text.
    Tts {
        state: TtsState,
        text: Option<String>,
    },
    /// IoT control payload (JSON-RPC 2.0), passed through verbatim.
    Mcp { payload: String },
    /// A recognized `type` this build does not model.
    Unknown { kind: String },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TtsState {
    Start,
    Stop,
    SentenceStart,
}

impl TtsState {
    #[must_use]
    pub const fn marker(self) -> &'static str {
        match self {
            Self::Start => "start",
            Self::Stop => "stop",
            Self::SentenceStart => "sentence_start",
        }
    }
}

impl ServerMessage {
    /// Parse one JSON text frame, dispatched by `type`.
    pub fn from_json(text: &str) -> Result<Self, String> {
        let value = json_lite::parse(text).map_err(|error| error.to_string())?;
        let kind = value
            .get("type")
            .and_then(JsonValue::as_str)
            .ok_or_else(|| "server message has no type".to_string())?;
        let session = || {
            value
                .get("session_id")
                .and_then(JsonValue::as_str)
                .map(String::from)
        };
        match kind {
            "hello" => Ok(Self::Hello {
                session_id: session(),
                audio_params: value.get("audio_params").and_then(|params| {
                    Some(AudioParams {
                        sample_rate: u32::try_from(
                            params.get("sample_rate").and_then(JsonValue::as_i64)?,
                        )
                        .ok()?,
                        channels: u8::try_from(params.get("channels").and_then(JsonValue::as_i64)?)
                            .ok()?,
                        frame_duration_ms: u32::try_from(
                            params.get("frame_duration").and_then(JsonValue::as_i64)?,
                        )
                        .ok()?,
                    })
                }),
            }),
            "stt" => Ok(Self::Stt {
                text: value
                    .get("text")
                    .and_then(JsonValue::as_str)
                    .unwrap_or("")
                    .into(),
            }),
            "llm" => Ok(Self::Llm {
                emotion: value
                    .get("emotion")
                    .and_then(JsonValue::as_str)
                    .unwrap_or("")
                    .into(),
                text: value
                    .get("text")
                    .and_then(JsonValue::as_str)
                    .unwrap_or("")
                    .into(),
            }),
            "tts" => {
                let state = value
                    .get("state")
                    .and_then(JsonValue::as_str)
                    .and_then(|state| match state {
                        "start" => Some(TtsState::Start),
                        "stop" => Some(TtsState::Stop),
                        "sentence_start" => Some(TtsState::SentenceStart),
                        _ => None,
                    })
                    .ok_or_else(|| format!("unknown tts state in {text}"))?;
                Ok(Self::Tts {
                    state,
                    text: value
                        .get("text")
                        .and_then(JsonValue::as_str)
                        .map(String::from),
                })
            }
            "mcp" => {
                let payload = value
                    .get("payload")
                    .ok_or_else(|| "mcp message has no payload".to_string())?;
                Ok(Self::Mcp {
                    payload: mcp_payload_json(payload),
                })
            }
            other => Ok(Self::Unknown { kind: other.into() }),
        }
    }
}

/// Re-serialize one JSON-RPC payload as compact text, preserving its fields.
fn mcp_payload_json(payload: &JsonValue) -> String {
    let JsonValue::Object(entries) = payload else {
        return "{}".to_string();
    };
    let mut out = String::from("{");
    for (index, (key, value)) in entries.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        out.push_str(&format!(
            "\"{}\":{}",
            json_lite::escape(key),
            json_text(value)
        ));
    }
    out.push('}');
    out
}

fn json_text(value: &JsonValue) -> String {
    match value {
        JsonValue::Null => "null".into(),
        JsonValue::Bool(true) => "true".into(),
        JsonValue::Bool(false) => "false".into(),
        JsonValue::Number(text) => text.clone(),
        JsonValue::String(text) => format!("\"{}\"", json_lite::escape(text)),
        JsonValue::Array(items) => {
            let items = items.iter().map(json_text).collect::<Vec<_>>().join(",");
            format!("[{items}]")
        }
        JsonValue::Object(entries) => {
            let entries = entries
                .iter()
                .map(|(key, value)| format!("\"{}\":{}", json_lite::escape(key), json_text(value)))
                .collect::<Vec<_>>()
                .join(",");
            format!("{{{entries}}}")
        }
    }
}

/// Message types inside the binary frame headers.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FrameType {
    Opus,
    Json,
}

impl FrameType {
    #[must_use]
    pub const fn code(self) -> u16 {
        match self {
            Self::Opus => 0,
            Self::Json => 1,
        }
    }

    fn from_code(code: u16) -> Option<Self> {
        match code {
            0 => Some(Self::Opus),
            1 => Some(Self::Json),
            _ => None,
        }
    }
}

/// One decoded binary frame: the message type and its Opus payload.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Frame {
    pub kind: FrameType,
    /// Version 2 carries a millisecond timestamp; version 3 leaves it zero.
    pub timestamp_ms: u32,
    pub payload: Vec<u8>,
}

/// Encode one frame with the version 2 header: version(2), type(2),
/// reserved(4), timestamp(4), payload size(4), then the payload. Big-endian.
#[must_use]
pub fn encode_v2(kind: FrameType, timestamp_ms: u32, payload: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(16 + payload.len());
    out.extend_from_slice(&2_u16.to_be_bytes());
    out.extend_from_slice(&kind.code().to_be_bytes());
    out.extend_from_slice(&0_u32.to_be_bytes());
    out.extend_from_slice(&timestamp_ms.to_be_bytes());
    out.extend_from_slice(&(payload.len() as u32).to_be_bytes());
    out.extend_from_slice(payload);
    out
}

/// Encode one frame with the version 3 header: type, reserved, payload size.
#[must_use]
pub fn encode_v3(kind: FrameType, payload: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(4 + payload.len());
    out.push(kind.code() as u8);
    out.push(0);
    out.extend_from_slice(&(payload.len() as u16).to_be_bytes());
    out.extend_from_slice(payload);
    out
}

/// Decode a version 2 frame with size checks. Header: version(2), type(2),
/// reserved(4), timestamp(4), payload size(4), then the payload.
pub fn decode_v2(bytes: &[u8]) -> Result<Frame, String> {
    if bytes.len() < 16 {
        return Err(format!(
            "v2 frame needs 16 header bytes, got {}",
            bytes.len()
        ));
    }
    let version = u16::from_be_bytes([bytes[0], bytes[1]]);
    if version != 2 {
        return Err(format!("not a v2 frame: version {version}"));
    }
    let kind = FrameType::from_code(u16::from_be_bytes([bytes[2], bytes[3]])).ok_or_else(|| {
        format!(
            "unknown v2 frame type {}",
            u16::from_be_bytes([bytes[2], bytes[3]])
        )
    })?;
    let timestamp_ms = u32::from_be_bytes([bytes[8], bytes[9], bytes[10], bytes[11]]);
    let size = u32::from_be_bytes([bytes[12], bytes[13], bytes[14], bytes[15]]);
    decode_tail(kind, timestamp_ms, &bytes[16..], size)
}

fn decode_tail(
    kind: FrameType,
    timestamp_ms: u32,
    rest: &[u8],
    size: u32,
) -> Result<Frame, String> {
    let size = usize::try_from(size).map_err(|_| "payload size overflows".to_string())?;
    if rest.len() < size {
        return Err(format!(
            "frame payload needs {size} bytes, got {}",
            rest.len()
        ));
    }
    Ok(Frame {
        kind,
        timestamp_ms,
        payload: rest[..size].to_vec(),
    })
}

/// Decode a version 3 frame with size checks.
pub fn decode_v3(bytes: &[u8]) -> Result<Frame, String> {
    if bytes.len() < 4 {
        return Err(format!(
            "v3 frame needs 4 header bytes, got {}",
            bytes.len()
        ));
    }
    let code = u16::from(bytes[0]);
    let kind = FrameType::from_code(code).ok_or_else(|| format!("unknown v3 frame type {code}"))?;
    let size = u16::from_be_bytes([bytes[2], bytes[3]]) as usize;
    if bytes.len() - 4 < size {
        return Err(format!(
            "frame payload needs {size} bytes, got {}",
            bytes.len() - 4
        ));
    }
    Ok(Frame {
        kind,
        timestamp_ms: 0,
        payload: bytes[4..4 + size].to_vec(),
    })
}

#[cfg(test)]
mod tests {
    use super::{
        decode_v2, decode_v3, encode_v2, encode_v3, AudioParams, DeviceMessage, FrameType,
        ListenMode, ListenState, ServerMessage, TtsState,
    };

    const SESSION: &str = "session-42";

    #[test]
    fn hello_announces_websocket_and_opus_params() {
        let json = DeviceMessage::Hello { version: 1 }.to_json("");
        assert!(json.contains("\"type\":\"hello\""));
        assert!(json.contains("\"version\":1"));
        assert!(json.contains("\"transport\":\"websocket\""));
        assert!(json.contains("\"features\":{\"mcp\":true}"));
        assert!(json.contains("\"format\":\"opus\""));
        assert!(json.contains("\"sample_rate\":16000"));
        assert!(json.contains("\"channels\":1"));
        assert!(json.contains("\"frame_duration\":60"));
    }

    #[test]
    fn listen_states_modes_and_detect_text() {
        let start = DeviceMessage::Listen {
            state: ListenState::Start,
            mode: Some(ListenMode::Auto),
            text: None,
        }
        .to_json(SESSION);
        assert_eq!(
            start,
            "{\"session_id\":\"session-42\",\"type\":\"listen\",\"state\":\"start\",\"mode\":\"auto\"}"
        );
        let stop = DeviceMessage::Listen {
            state: ListenState::Stop,
            mode: None,
            text: None,
        }
        .to_json(SESSION);
        assert_eq!(
            stop,
            "{\"session_id\":\"session-42\",\"type\":\"listen\",\"state\":\"stop\"}"
        );
        let detect = DeviceMessage::Listen {
            state: ListenState::Detect,
            mode: None,
            text: Some("Hola XiaoZhi".into()),
        }
        .to_json(SESSION);
        assert_eq!(
            detect,
            "{\"session_id\":\"session-42\",\"type\":\"listen\",\"state\":\"detect\",\"text\":\"Hola XiaoZhi\"}"
        );
    }

    #[test]
    fn abort_and_mcp_pass_payloads_through() {
        let abort = DeviceMessage::Abort {
            reason: "wake_word_detected".into(),
        }
        .to_json(SESSION);
        assert_eq!(
            abort,
            "{\"session_id\":\"session-42\",\"type\":\"abort\",\"reason\":\"wake_word_detected\"}"
        );
        let payload = "{\"jsonrpc\":\"2.0\",\"id\":1,\"result\":{\"content\":[{\"type\":\"text\",\"text\":\"true\"}],\"isError\":false}}";
        let mcp = DeviceMessage::Mcp {
            payload: payload.into(),
        }
        .to_json(SESSION);
        assert_eq!(
            mcp,
            format!("{{\"session_id\":\"session-42\",\"type\":\"mcp\",\"payload\":{payload}}}")
        );
    }

    #[test]
    fn server_hello_parses_session_and_audio_params() {
        let json = r#"{"type":"hello","transport":"websocket","session_id":"abc","audio_params":{"format":"opus","sample_rate":24000,"channels":1,"frame_duration":60}}"#;
        let message = ServerMessage::from_json(json).unwrap();
        match message {
            ServerMessage::Hello {
                session_id,
                audio_params,
            } => {
                assert_eq!(session_id.as_deref(), Some("abc"));
                assert_eq!(
                    audio_params,
                    Some(AudioParams {
                        sample_rate: 24_000,
                        channels: 1,
                        frame_duration_ms: 60,
                    })
                );
            }
            other => panic!("expected hello, got {other:?}"),
        }
    }

    #[test]
    fn server_events_parse_from_the_document_examples() {
        let stt = ServerMessage::from_json(
            "{\"session_id\":\"x\",\"type\":\"stt\",\"text\":\"what the user said\"}",
        )
        .unwrap();
        assert_eq!(
            stt,
            ServerMessage::Stt {
                text: "what the user said".into()
            }
        );
        let llm = ServerMessage::from_json(
            "{\"session_id\":\"x\",\"type\":\"llm\",\"emotion\":\"happy\",\"text\":\"\\ud83d\\ude00\"}",
        )
        .unwrap();
        assert_eq!(
            llm,
            ServerMessage::Llm {
                emotion: "happy".into(),
                text: "\u{1F600}".into()
            }
        );
        let start = ServerMessage::from_json("{\"type\":\"tts\",\"state\":\"start\"}").unwrap();
        assert_eq!(
            start,
            ServerMessage::Tts {
                state: TtsState::Start,
                text: None
            }
        );
        let sentence = ServerMessage::from_json(
            "{\"type\":\"tts\",\"state\":\"sentence_start\",\"text\":\"Hola.\"}",
        )
        .unwrap();
        assert_eq!(
            sentence,
            ServerMessage::Tts {
                state: TtsState::SentenceStart,
                text: Some("Hola.".into())
            }
        );
        let tools_call = ServerMessage::from_json(
            "{\"type\":\"mcp\",\"payload\":{\"jsonrpc\":\"2.0\",\"method\":\"tools/call\",\"params\":{\"name\":\"self.light.set_rgb\",\"arguments\":{\"r\":255}}}}",
        )
        .unwrap();
        assert!(matches!(tools_call, ServerMessage::Mcp { .. }));
        let unknown = ServerMessage::from_json("{\"type\":\"system\"}").unwrap();
        assert_eq!(
            unknown,
            ServerMessage::Unknown {
                kind: "system".into()
            }
        );
    }

    #[test]
    fn malformed_server_input_is_an_error() {
        assert!(ServerMessage::from_json("not json").is_err());
        assert!(ServerMessage::from_json("{\"no_type\":1}").is_err());
        assert!(ServerMessage::from_json("{\"type\":\"tts\",\"state\":\"sleep\"}").is_err());
        assert!(ServerMessage::from_json("{\"type\":\"mcp\"}").is_err());
    }

    #[test]
    fn v2_frames_round_trip_with_size_checks() {
        let payload = [0xAA_u8, 0xBB, 0xCC];
        let frame = encode_v2(FrameType::Opus, 1234, &payload);
        assert_eq!(frame.len(), 16 + 3);
        assert_eq!(&frame[..2], &[0, 2]);
        let decoded = decode_v2(&frame).unwrap();
        assert_eq!(decoded.kind, FrameType::Opus);
        assert_eq!(decoded.timestamp_ms, 1234);
        assert_eq!(decoded.payload, payload);
        // A short frame is rejected, as is an oversized declared payload.
        assert!(decode_v2(&frame[..15]).is_err());
        let mut lying = frame.clone();
        lying[12..16].copy_from_slice(&999_u32.to_be_bytes());
        assert!(decode_v2(&lying).is_err());
        let json = encode_v2(FrameType::Json, 7, b"{}");
        assert_eq!(decode_v2(&json).unwrap().kind, FrameType::Json);
    }

    #[test]
    fn v3_frames_round_trip_with_size_checks() {
        let payload = [1_u8, 2, 3, 4];
        let frame = encode_v3(FrameType::Opus, &payload);
        assert_eq!(frame.len(), 4 + 4);
        let decoded = decode_v3(&frame).unwrap();
        assert_eq!(decoded.kind, FrameType::Opus);
        assert_eq!(decoded.timestamp_ms, 0);
        assert_eq!(decoded.payload, payload);
        assert!(decode_v3(&frame[..3]).is_err());
        let mut lying = frame.clone();
        lying[2..4].copy_from_slice(&999_u16.to_be_bytes());
        assert!(decode_v3(&lying).is_err());
    }
}
