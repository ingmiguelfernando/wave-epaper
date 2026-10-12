//! The device side of MCP: XiaoZhi asks which tools this device has and
//! calls them, in JSON-RPC 2.0 payloads of `mcp` messages. The protocol is
//! ported from `main/mcp_server.cc`; the tools are this device's.

use std::path::Path;

use anyhow::{ensure, Context, Result};

use crate::{
    bible::{self, ChapterItem},
    build_info::FIRMWARE_VERSION,
    json_lite::{self, JsonValue},
};

use super::{activation::BOARD_NAME, protocol::json_text};

/// Longest passage handed to XiaoZhi; a long chapter is cut at a verse.
const PASSAGE_LIMIT_CHARS: usize = 2_400;

/// The tools, as `tools/list` describes them.
const TOOLS: &str = r#"[{"name":"self.get_device_status","description":"Provides the real-time information of the device, including the volume of the audio speaker, the battery and the network. Use this tool for: 1. answering questions about the current condition of the device; 2. as the first step to change the volume.","inputSchema":{"type":"object","properties":{}}},{"name":"self.audio_speaker.set_volume","description":"Set the volume of the audio speaker. If the current volume is unknown, call self.get_device_status first.","inputSchema":{"type":"object","properties":{"volume":{"type":"integer","minimum":0,"maximum":100}},"required":["volume"]}},{"name":"self.bible.read","description":"Read a Bible passage from the Bible on the device's SD card, in the reader's translation. Use it whenever the user asks to read, quote or explain a Bible chapter or verse. book is the USFM code, such as GEN, PSA, PRO, ISA, MAT, JHN or ROM; verse_start and verse_end are optional and give a range.","inputSchema":{"type":"object","properties":{"book":{"type":"string"},"chapter":{"type":"integer","minimum":1},"verse_start":{"type":"integer","minimum":1},"verse_end":{"type":"integer","minimum":1}},"required":["book","chapter"]}}]"#;

/// A passage XiaoZhi asks for.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BibleRequest {
    pub book: String,
    pub chapter: u16,
    pub first: Option<u16>,
    pub last: Option<u16>,
}

impl BibleRequest {
    /// `PSA 23:1-3`, for the screen's tool line.
    #[must_use]
    pub fn label(&self) -> String {
        let book = self.book.trim().to_uppercase();
        match (self.first, self.last) {
            (Some(first), Some(last)) if last > first => {
                format!("{book} {}:{first}-{last}", self.chapter)
            }
            (Some(first), _) => format!("{book} {}:{first}", self.chapter),
            _ => format!("{book} {}", self.chapter),
        }
    }
}

/// What a tool call needs from the device.
pub trait McpDevice {
    /// The device state as JSON text.
    fn status(&self) -> String;
    /// Set the speaker volume, 0 to 100.
    fn set_volume(&mut self, percent: u8) -> Result<(), String>;
    /// A passage from the Bible on the SD card.
    fn read_bible(&mut self, request: &BibleRequest) -> Result<String, String>;
}

/// The reply to send and the tool line to show.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct McpOutcome {
    /// The JSON-RPC response; `None` for a notification.
    pub reply: Option<String>,
    /// `bible.read(PSA 23)` when a tool ran.
    pub tool: Option<String>,
}

/// Answer one JSON-RPC payload.
pub fn handle(payload: &str, device: &mut dyn McpDevice) -> McpOutcome {
    let Ok(request) = json_lite::parse(payload) else {
        return McpOutcome::default();
    };
    // Without an id the message is a notification: nothing to answer.
    let Some(id) = request.get("id").map(json_text) else {
        return McpOutcome::default();
    };
    let method = request
        .get("method")
        .and_then(JsonValue::as_str)
        .unwrap_or_default();
    match method {
        "initialize" => reply(
            &id,
            &format!(
                "{{\"protocolVersion\":\"2024-11-05\",\"capabilities\":{{\"tools\":{{}}}},\
\"serverInfo\":{{\"name\":\"{BOARD_NAME}\",\"version\":\"{FIRMWARE_VERSION}\"}}}}"
            ),
        ),
        "tools/list" => reply(&id, &format!("{{\"tools\":{TOOLS}}}")),
        "tools/call" => call_tool(&id, request.get("params"), device),
        other => error(&id, -32601, &format!("Method not found: {other}")),
    }
}

fn call_tool(id: &str, params: Option<&JsonValue>, device: &mut dyn McpDevice) -> McpOutcome {
    let name = params
        .and_then(|params| params.get("name"))
        .and_then(JsonValue::as_str)
        .unwrap_or_default();
    let arguments = params.and_then(|params| params.get("arguments"));
    let number = |key: &str| {
        arguments
            .and_then(|arguments| arguments.get(key))
            .and_then(JsonValue::as_i64)
    };
    match name {
        "self.get_device_status" => McpOutcome {
            reply: text_result(id, &device.status(), false).reply,
            tool: Some("device status".into()),
        },
        "self.audio_speaker.set_volume" => {
            let Some(volume) = number("volume")
                .and_then(|volume| u8::try_from(volume).ok())
                .filter(|volume| *volume <= 100)
            else {
                return error(id, -32602, "volume must be 0 to 100");
            };
            let outcome = match device.set_volume(volume) {
                Ok(()) => text_result(id, "true", false),
                Err(failure) => text_result(id, &failure, true),
            };
            McpOutcome {
                tool: Some(format!("volume {volume}")),
                ..outcome
            }
        }
        "self.bible.read" => {
            let book = arguments
                .and_then(|arguments| arguments.get("book"))
                .and_then(JsonValue::as_str)
                .unwrap_or_default();
            let verse = |key: &str| number(key).and_then(|value| u16::try_from(value).ok());
            let Some(chapter) = verse("chapter").filter(|chapter| *chapter > 0) else {
                return error(id, -32602, "chapter must be 1 or more");
            };
            if book.trim().is_empty() {
                return error(id, -32602, "book is missing");
            }
            let request = BibleRequest {
                book: book.into(),
                chapter,
                first: verse("verse_start"),
                last: verse("verse_end"),
            };
            let outcome = match device.read_bible(&request) {
                Ok(passage) => text_result(id, &passage, false),
                Err(failure) => text_result(id, &failure, true),
            };
            McpOutcome {
                tool: Some(format!("bible.read({})", request.label())),
                ..outcome
            }
        }
        other => error(id, -32601, &format!("Unknown tool: {other}")),
    }
}

fn reply(id: &str, result: &str) -> McpOutcome {
    McpOutcome {
        reply: Some(format!(
            "{{\"jsonrpc\":\"2.0\",\"id\":{id},\"result\":{result}}}"
        )),
        tool: None,
    }
}

fn text_result(id: &str, text: &str, is_error: bool) -> McpOutcome {
    reply(
        id,
        &format!(
            "{{\"content\":[{{\"type\":\"text\",\"text\":\"{}\"}}],\"isError\":{is_error}}}",
            json_lite::escape(text)
        ),
    )
}

fn error(id: &str, code: i32, message: &str) -> McpOutcome {
    McpOutcome {
        reply: Some(format!(
            "{{\"jsonrpc\":\"2.0\",\"id\":{id},\"error\":{{\"code\":{code},\"message\":\"{}\"}}}}",
            json_lite::escape(message)
        )),
        tool: None,
    }
}

/// The device's state for `self.get_device_status`, in upstream's shape.
#[must_use]
pub fn status_json(volume: u8, battery: Option<u8>, ssid: &str, rssi: Option<i32>) -> String {
    let battery = match battery {
        Some(level) => format!("{{\"level\":{level}}}"),
        None => "null".into(),
    };
    let signal = match rssi {
        Some(rssi) if rssi >= -55 => "strong",
        Some(rssi) if rssi >= -70 => "medium",
        Some(_) => "weak",
        None => "unknown",
    };
    format!(
        "{{\"audio_speaker\":{{\"volume\":{volume}}},\"screen\":{{\"type\":\"e-paper\"}},\
\"battery\":{battery},\"network\":{{\"type\":\"wifi\",\"ssid\":\"{}\",\"signal\":\"{signal}\"}}}}",
        json_lite::escape(ssid)
    )
}

/// Read a passage from the Bible folder: the book by USFM code, name or
/// number, the whole chapter or the requested verses, with verse numbers.
pub fn bible_passage(
    root: &Path,
    translation: Option<&str>,
    request: &BibleRequest,
) -> Result<String> {
    let translation = match translation {
        Some(code) => code.to_owned(),
        None => bible::translations(root)
            .context("reading the Bible folder")?
            .into_iter()
            .next()
            .context("there is no Bible on the SD card")?,
    };
    let index = std::fs::read_to_string(root.join(&translation).join("index.tsv"))
        .context("reading the Bible's book list")?;
    let books = bible::parse_index(&index)?;
    let wanted = request.book.trim();
    let book = books
        .iter()
        .find(|book| {
            book.usfm.eq_ignore_ascii_case(wanted)
                || book.name.eq_ignore_ascii_case(wanted)
                || book.short_name.eq_ignore_ascii_case(wanted)
                || wanted.parse::<u8>().ok() == Some(book.number)
        })
        .with_context(|| format!("the Bible on the SD card has no book {wanted}"))?;
    let items = bible::load_chapter(root, &translation, book, request.chapter)?;
    let first = request.first.unwrap_or(1);
    let last = request.last.or(request.first).unwrap_or(u16::MAX);
    let mut verses = String::new();
    for item in &items {
        let ChapterItem::Verse { label, text, .. } = item else {
            continue;
        };
        let Some((start, end)) = bible::verse_span(label) else {
            continue;
        };
        if start > last || end < first {
            continue;
        }
        let verse = format!("{label} {text}");
        if verses.chars().count() + verse.chars().count() > PASSAGE_LIMIT_CHARS {
            verses.push_str(" …");
            break;
        }
        if !verses.is_empty() {
            verses.push(' ');
        }
        verses.push_str(&verse);
    }
    ensure!(
        !verses.is_empty(),
        "{} {} has no such verses",
        book.name,
        request.chapter
    );
    Ok(format!(
        "{} {} ({translation}): {verses}",
        book.name, request.chapter
    ))
}

#[cfg(test)]
mod tests {
    use std::{fs, path::PathBuf};

    use super::{bible_passage, handle, status_json, BibleRequest, McpDevice};
    use crate::json_lite::{self, JsonValue};

    #[derive(Default)]
    struct FakeDevice {
        volume: u8,
        reads: Vec<BibleRequest>,
    }

    impl McpDevice for FakeDevice {
        fn status(&self) -> String {
            status_json(self.volume, Some(76), "Casa", Some(-60))
        }

        fn set_volume(&mut self, percent: u8) -> Result<(), String> {
            self.volume = percent;
            Ok(())
        }

        fn read_bible(&mut self, request: &BibleRequest) -> Result<String, String> {
            self.reads.push(request.clone());
            Ok("Salmos 23 (RVR1960): 1 Jehová es mi pastor".into())
        }
    }

    fn call(device: &mut FakeDevice, payload: &str) -> JsonValue {
        let outcome = handle(payload, device);
        json_lite::parse(&outcome.reply.expect("a reply")).unwrap()
    }

    fn at<'a>(value: &'a JsonValue, path: &[&str]) -> Option<&'a JsonValue> {
        path.iter().try_fold(value, |current, key| current.get(key))
    }

    fn number(value: &JsonValue, path: &[&str]) -> Option<i64> {
        at(value, path).and_then(JsonValue::as_i64)
    }

    #[test]
    fn initialize_and_list_describe_the_device_and_its_tools() {
        let mut device = FakeDevice::default();
        let init = call(
            &mut device,
            r#"{"jsonrpc":"2.0","method":"initialize","params":{"capabilities":{}},"id":1}"#,
        );
        assert_eq!(number(&init, &["id"]), Some(1));
        let name = at(&init, &["result", "serverInfo", "name"]);
        assert_eq!(name.and_then(JsonValue::as_str), Some("Wave e-paper"));
        let list = call(
            &mut device,
            r#"{"jsonrpc":"2.0","method":"tools/list","params":{"cursor":""},"id":"two"}"#,
        );
        assert_eq!(at(&list, &["id"]).and_then(JsonValue::as_str), Some("two"));
        let tools = at(&list, &["result", "tools"]).unwrap();
        let name = at(tools.index(2).unwrap(), &["name"]);
        assert_eq!(name.and_then(JsonValue::as_str), Some("self.bible.read"));
    }

    #[test]
    fn tools_run_and_report_what_they_did() {
        let mut device = FakeDevice::default();
        let payload = r#"{"jsonrpc":"2.0","method":"tools/call","params":{"name":"self.audio_speaker.set_volume","arguments":{"volume":70}},"id":3}"#;
        let outcome = handle(payload, &mut device);
        assert_eq!(device.volume, 70);
        assert_eq!(outcome.tool.as_deref(), Some("volume 70"));
        let status = r#"{"jsonrpc":"2.0","method":"tools/call","params":{"name":"self.get_device_status","arguments":{}},"id":4}"#;
        let reply = call(&mut device, status);
        let content = at(&reply, &["result", "content"]).unwrap();
        let text = at(content.index(0).unwrap(), &["text"]).and_then(JsonValue::as_str);
        let state = json_lite::parse(text.unwrap()).unwrap();
        assert_eq!(number(&state, &["audio_speaker", "volume"]), Some(70));
        let read = r#"{"jsonrpc":"2.0","method":"tools/call","params":{"name":"self.bible.read","arguments":{"book":"psa","chapter":23,"verse_start":1,"verse_end":3}},"id":5}"#;
        let outcome = handle(read, &mut device);
        assert_eq!(outcome.tool.as_deref(), Some("bible.read(PSA 23:1-3)"));
        assert_eq!(device.reads[0].first, Some(1));
        assert_eq!(device.reads[0].last, Some(3));
    }

    #[test]
    fn bad_calls_get_json_rpc_errors_and_notifications_get_nothing() {
        let mut device = FakeDevice::default();
        let unknown = r#"{"jsonrpc":"2.0","method":"tools/call","params":{"name":"self.light.set_rgb"},"id":6}"#;
        let reply = call(&mut device, unknown);
        assert_eq!(number(&reply, &["error", "code"]), Some(-32601));
        let loud = r#"{"jsonrpc":"2.0","method":"tools/call","params":{"name":"self.audio_speaker.set_volume","arguments":{"volume":300}},"id":7}"#;
        let reply = call(&mut device, loud);
        assert_eq!(number(&reply, &["error", "code"]), Some(-32602));
        assert_eq!(device.volume, 0);
        let notification = r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#;
        assert_eq!(handle(notification, &mut device).reply, None);
        assert_eq!(handle("not json", &mut device).reply, None);
    }

    fn bible_root() -> PathBuf {
        let root = std::env::temp_dir().join(format!("wave-mcp-bible-{}", std::process::id()));
        let folder = root.join("RVR1960");
        fs::create_dir_all(&folder).unwrap();
        fs::write(folder.join("index.tsv"), "PSA\tSalmos\t150\tPSA.txt\n").unwrap();
        fs::write(
            folder.join("PSA.txt"),
            "C\t23\nH\tJehová es mi pastor\nV\t1\t1\tJehová es mi pastor; nada me faltará.\n\
V\t2\t0\tEn lugares de delicados pastos me hará descansar.\nV\t3-4\t0\tConfortará mi alma.\n",
        )
        .unwrap();
        root
    }

    #[test]
    fn a_passage_comes_from_the_sd_card_bible() {
        let root = bible_root();
        let whole = BibleRequest {
            book: "PSA".into(),
            chapter: 23,
            first: None,
            last: None,
        };
        let passage = bible_passage(&root, None, &whole).unwrap();
        assert!(passage.starts_with("Salmos 23 (RVR1960): 1 Jehová es mi pastor"));
        assert!(passage.ends_with("3-4 Confortará mi alma."));
        // One verse by its book name; a range label counts when it overlaps.
        let one = BibleRequest {
            book: "salmos".into(),
            first: Some(4),
            ..whole.clone()
        };
        let verse = bible_passage(&root, Some("RVR1960"), &one).unwrap();
        assert_eq!(verse, "Salmos 23 (RVR1960): 3-4 Confortará mi alma.");
        let missing = BibleRequest {
            book: "JHN".into(),
            ..whole
        };
        assert!(bible_passage(&root, None, &missing).is_err());
        let _ = fs::remove_dir_all(root);
    }
}
