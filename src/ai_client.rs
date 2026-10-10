//! OpenAI-compatible request and response bodies for Voice Notes v2. No
//! network access and no keys live here; `ai_jobs` sends the requests.

use crate::{
    ai_config::{AiLanguage, SummaryStyle},
    json_lite::{self, JsonValue},
    voice_notes::bytes_per_second,
};

/// Separates the multipart fields. Long and odd, so a recording never
/// contains it by chance.
pub const MULTIPART_BOUNDARY: &str = "----WaveVoiceNote7d3f9a2c41b8e6";

/// Longest audio in one transcription request: ten minutes of 16 kHz mono
/// PCM is 19.2 MB, under the 25 MB the providers accept.
pub const MAX_PART_SECONDS: u32 = 10 * 60;

/// Content-Type and byte body of the transcription request, paired.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Multipart {
    pub content_type: String,
    pub body: Vec<u8>,
}

/// The `multipart/form-data` fields of `POST /audio/transcriptions` around
/// the WAV file: the bytes before the file and the bytes after it. Without
/// a language the provider detects it.
#[must_use]
pub fn transcription_form(
    model: &str,
    language: Option<&str>,
    boundary: &str,
) -> (Vec<u8>, Vec<u8>) {
    let mut head: Vec<u8> = Vec::new();
    let part = |body: &mut Vec<u8>, text: &str| body.extend_from_slice(text.as_bytes());
    let fields = [
        ("model", Some(model)),
        ("language", language),
        ("response_format", Some("json")),
    ];
    for (name, value) in fields {
        let Some(value) = value else {
            continue;
        };
        part(&mut head, "--");
        part(&mut head, boundary);
        part(&mut head, "\r\nContent-Disposition: form-data; name=\"");
        part(&mut head, name);
        part(&mut head, "\"\r\n\r\n");
        part(&mut head, value);
        part(&mut head, "\r\n");
    }
    part(&mut head, "--");
    part(&mut head, boundary);
    part(
        &mut head,
        "\r\nContent-Disposition: form-data; name=\"file\"; filename=\"note.wav\"\r\n\
         Content-Type: audio/wav\r\n\r\n",
    );
    let tail = format!("\r\n--{boundary}--\r\n").into_bytes();
    (head, tail)
}

/// The whole transcription request for a WAV held in memory: model,
/// language, JSON response format and the file, byte-exact.
#[must_use]
pub fn transcription_request(
    model: &str,
    language: Option<&str>,
    wav: &[u8],
    boundary: &str,
) -> Multipart {
    let (mut body, tail) = transcription_form(model, language, boundary);
    body.extend_from_slice(wav);
    body.extend_from_slice(&tail);
    Multipart {
        content_type: format!("multipart/form-data; boundary={boundary}"),
        body,
    }
}

/// The ISO 639-1 code sent with a transcription, or `None` to let the
/// provider detect the language.
#[must_use]
pub fn language_code(language: AiLanguage) -> Option<&'static str> {
    (language != AiLanguage::Auto).then(|| language.marker())
}

/// A recording's audio in parts of at most `MAX_PART_SECONDS`, as (offset,
/// length) pairs over its PCM bytes. The parts are equal, so none is a
/// short tail, and they end on whole samples.
#[must_use]
pub fn wav_parts(pcm_bytes: u32) -> Vec<(u32, u32)> {
    let total = pcm_bytes & !1;
    if total == 0 {
        return Vec::new();
    }
    let limit = MAX_PART_SECONDS * bytes_per_second();
    let count = total.div_ceil(limit);
    let size = total.div_ceil(count).next_multiple_of(2);
    (0..count)
        .map(|index| index * size)
        .take_while(|&offset| offset < total)
        .map(|offset| (offset, size.min(total - offset)))
        .collect()
}

/// `https://api.groq.com/openai/v1` and `/audio/transcriptions` joined with
/// one slash. A URL that already ends with the path stays as it is.
#[must_use]
pub fn endpoint(base_url: &str, path: &str) -> String {
    let base = base_url.trim().trim_end_matches('/');
    if base.ends_with(path) {
        base.to_string()
    } else {
        format!("{base}{path}")
    }
}

/// The `text` field of a successful transcription response.
pub fn parse_transcription(json: &str) -> Result<String, String> {
    let value = json_lite::parse(json).map_err(|error| error.to_string())?;
    value
        .get("text")
        .and_then(JsonValue::as_str)
        .map(String::from)
        .ok_or_else(|| "transcription response has no text field".into())
}

/// The `chat/completions` request body: a system prompt that asks for a
/// title line, then the summary in the style chosen in Settings › AI, as
/// plain text in the transcript's language.
#[must_use]
pub fn summary_request(model: &str, transcript: &str, style: SummaryStyle) -> String {
    let shape = match style {
        SummaryStyle::BulletsTodos => {
            "the key points, one per line, each starting with \"\u{2022} \". If the \
note mentions tasks, add a line with the word for to-dos in the transcript's \
language, then one task per line, each starting with \"\u{2610} \"."
        }
        SummaryStyle::Bullets => {
            "the key points, one per line, each starting with \"\u{2022} \"."
        }
        SummaryStyle::Paragraph => "one short paragraph.",
    };
    let system = format!(
        "You summarise voice notes. Reply in plain text without Markdown, in the \
same language as the transcript. First line: a title of at most 40 characters. \
Then {shape}"
    );
    let user = format!("Transcript:\n{transcript}");
    format!(
        "{{\"model\":\"{}\",\"messages\":[{{\"role\":\"system\",\"content\":\"{}\"}},\
{{\"role\":\"user\",\"content\":\"{}\"}}],\"temperature\":0.2,\"max_tokens\":1000}}",
        json_lite::escape(model),
        json_lite::escape(&system),
        json_lite::escape(&user),
    )
}

/// The assistant reply of a successful completion: `choices[0].message.content`.
pub fn parse_chat_completion(json: &str) -> Result<String, String> {
    let value = json_lite::parse(json).map_err(|error| error.to_string())?;
    let content = value
        .get("choices")
        .and_then(|choices| choices.index(0))
        .and_then(|choice| choice.get("message"))
        .and_then(|message| message.get("content"))
        .and_then(JsonValue::as_str)
        .ok_or_else(|| "completion response has no choices[0].message.content".to_string())?;
    Ok(content.to_string())
}

/// The reply without the Markdown a model may add anyway: `**` and `__`
/// marks go, `#` heading marks go, and `- ` or `* ` list marks become `• `.
#[must_use]
pub fn clean_reply(content: &str) -> String {
    let plain = content.replace("**", "").replace("__", "");
    let lines: Vec<String> = plain
        .lines()
        .map(|line| {
            let trimmed = line.trim_start();
            if trimmed.starts_with('#') {
                trimmed.trim_start_matches('#').trim_start().to_string()
            } else if let Some(rest) = trimmed
                .strip_prefix("- ")
                .or_else(|| trimmed.strip_prefix("* "))
            {
                format!("\u{2022} {rest}")
            } else {
                line.to_string()
            }
        })
        .collect();
    lines.join("\n").trim().to_string()
}

/// Split the assistant reply into `(title, summary)`. The title is the first
/// line without quotes, clamped to 40 characters; the summary is the rest,
/// trimmed.
#[must_use]
pub fn split_title(content: &str) -> (String, String) {
    let (first, rest) = match content.split_once('\n') {
        Some((first, rest)) => (first, rest),
        None => (content, ""),
    };
    let title: String = first
        .trim()
        .trim_matches(['"', '\u{201c}', '\u{201d}'])
        .trim()
        .chars()
        .take(40)
        .collect();
    (title, rest.trim().to_string())
}

/// Statuses worth another try later: timeouts, rate limits and server errors.
#[must_use]
pub const fn is_retryable_status(status: u16) -> bool {
    matches!(status, 408 | 425 | 429 | 500..=599)
}

/// The provider's `error.message` when present, otherwise a generic line.
#[must_use]
pub fn api_error(status: u16, json: &str) -> String {
    if let Ok(value) = json_lite::parse(json) {
        if let Some(message) = value
            .get("error")
            .and_then(|error| error.get("message"))
            .and_then(JsonValue::as_str)
        {
            return format!("HTTP {status}: {message}");
        }
    }
    format!("HTTP {status}: request failed")
}

#[cfg(test)]
mod tests {
    use super::{
        api_error, clean_reply, endpoint, is_retryable_status, language_code,
        parse_chat_completion, parse_transcription, split_title, summary_request,
        transcription_form, transcription_request, wav_parts, MAX_PART_SECONDS,
    };
    use crate::{
        ai_config::{AiLanguage, SummaryStyle},
        voice_notes::bytes_per_second,
    };

    /// A 44-byte header plus one PCM sample, with a high byte to prove
    /// byte-exact round trips.
    fn tiny_wav() -> Vec<u8> {
        let mut wav = vec![0_u8; 44];
        wav[..4].copy_from_slice(b"RIFF");
        wav[8..12].copy_from_slice(b"WAVE");
        wav.extend_from_slice(&[0x12, 0x34, 0x56, 0x7F, 0x80, 0xFF, 0xAB, 0xCD]);
        wav
    }

    #[test]
    fn multipart_body_is_byte_exact() {
        let wav = tiny_wav();
        let request = transcription_request("whisper-large-v3", Some("es"), &wav, "wave7");
        assert_eq!(request.content_type, "multipart/form-data; boundary=wave7");

        let mut expected: Vec<u8> = Vec::new();
        let part = |expected: &mut Vec<u8>, text: &str| {
            expected.extend_from_slice(text.as_bytes());
        };
        part(&mut expected, "--wave7\r\n");
        part(
            &mut expected,
            "Content-Disposition: form-data; name=\"model\"\r\n\r\n",
        );
        part(&mut expected, "whisper-large-v3\r\n");
        part(&mut expected, "--wave7\r\n");
        part(
            &mut expected,
            "Content-Disposition: form-data; name=\"language\"\r\n\r\n",
        );
        part(&mut expected, "es\r\n");
        part(&mut expected, "--wave7\r\n");
        part(
            &mut expected,
            "Content-Disposition: form-data; name=\"response_format\"\r\n\r\n",
        );
        part(&mut expected, "json\r\n");
        part(&mut expected, "--wave7\r\n");
        part(
            &mut expected,
            "Content-Disposition: form-data; name=\"file\"; filename=\"note.wav\"\r\n",
        );
        part(&mut expected, "Content-Type: audio/wav\r\n\r\n");
        expected.extend_from_slice(&wav);
        part(&mut expected, "\r\n--wave7--\r\n");

        assert_eq!(request.body, expected);
        // The WAV payload survives byte for byte, 0x80 and 0xFF included.
        let file_start = expected
            .windows(wav.len())
            .position(|window| window == wav.as_slice())
            .unwrap();
        assert_eq!(
            &request.body[file_start..file_start + wav.len()],
            wav.as_slice()
        );
    }

    #[test]
    fn the_form_wraps_the_file_and_auto_language_is_left_out() {
        let wav = tiny_wav();
        let (head, tail) = transcription_form("whisper-large-v3-turbo", None, "b0");
        let whole = transcription_request("whisper-large-v3-turbo", None, &wav, "b0");
        assert_eq!(
            whole.body,
            [head.as_slice(), wav.as_slice(), tail.as_slice()].concat()
        );
        let head = String::from_utf8(head).unwrap();
        assert!(!head.contains("name=\"language\""));
        assert!(head.contains("name=\"response_format\"\r\n\r\njson\r\n"));
        assert_eq!(tail, b"\r\n--b0--\r\n");
        assert_eq!(language_code(AiLanguage::Auto), None);
        assert_eq!(language_code(AiLanguage::Spanish), Some("es"));
        assert_eq!(language_code(AiLanguage::English), Some("en"));
    }

    #[test]
    fn long_recordings_split_into_equal_parts_of_at_most_ten_minutes() {
        let limit = MAX_PART_SECONDS * bytes_per_second();
        assert!(wav_parts(0).is_empty());
        assert_eq!(wav_parts(64_000), [(0, 64_000)]);
        assert_eq!(wav_parts(limit), [(0, limit)]);
        // Ten minutes and two bytes: two halves, not ten minutes and a sliver.
        let parts = wav_parts(limit + 2);
        assert_eq!(parts, [(0, limit / 2 + 2), (limit / 2 + 2, limit / 2)]);
        // Thirty minutes and an odd byte: three full parts, whole samples only.
        let parts = wav_parts(3 * limit + 1);
        assert_eq!(parts.len(), 3);
        let mut next = 0;
        for (offset, length) in parts {
            assert_eq!(offset, next);
            assert!(length <= limit && length % 2 == 0);
            next = offset + length;
        }
        assert_eq!(next, 3 * limit);
    }

    #[test]
    fn endpoints_join_with_one_slash() {
        assert_eq!(
            endpoint("https://api.groq.com/openai/v1", "/audio/transcriptions"),
            "https://api.groq.com/openai/v1/audio/transcriptions"
        );
        assert_eq!(
            endpoint(" https://openrouter.ai/api/v1/ ", "/chat/completions"),
            "https://openrouter.ai/api/v1/chat/completions"
        );
        assert_eq!(
            endpoint("http://10.0.0.5/v1/chat/completions", "/chat/completions"),
            "http://10.0.0.5/v1/chat/completions"
        );
    }

    #[test]
    fn replies_lose_markdown_and_keep_their_bullets() {
        let reply = "**Lista de compras**\n## Puntos\n- Leche\n* Pan\n\u{2022} Manzanas\n";
        assert_eq!(
            clean_reply(reply),
            "Lista de compras\nPuntos\n\u{2022} Leche\n\u{2022} Pan\n\u{2022} Manzanas"
        );
        let (title, _) = split_title("\"Idea: club de lectura\"\nresto");
        assert_eq!(title, "Idea: club de lectura");
    }

    #[test]
    fn busy_and_broken_servers_are_worth_another_try() {
        for status in [408, 425, 429, 500, 502, 503, 504] {
            assert!(is_retryable_status(status), "{status}");
        }
        for status in [400, 401, 403, 404, 413, 422] {
            assert!(!is_retryable_status(status), "{status}");
        }
    }

    #[test]
    fn transcription_text_with_quotes_and_accents() {
        let json = r#"{"text":"Dijo \"hola\" — hasta mañana, ño.","language":"es"}"#;
        assert_eq!(
            parse_transcription(json).unwrap(),
            "Dijo \"hola\" — hasta mañana, ño."
        );
        assert!(parse_transcription(r#"{"no_text":true}"#).is_err());
    }

    #[test]
    fn summary_request_carries_model_prompt_and_limits() {
        let transcript = "Notas del mercado:\n- subió el \"euro\"";
        let body = summary_request(
            "llama-3.1-8b-instant",
            transcript,
            SummaryStyle::BulletsTodos,
        );
        assert!(body.contains("temperature\":0.2"));
        assert!(body.contains("max_tokens\":1000"));
        assert!(body.contains("at most 40 characters"));
        assert!(body.contains("\u{2610}"), "to-dos are asked for");
        let bullets = summary_request("m", transcript, SummaryStyle::Bullets);
        assert!(bullets.contains("\u{2022}"));
        assert!(!bullets.contains("\u{2610}"));
        let paragraph = summary_request("m", transcript, SummaryStyle::Paragraph);
        assert!(paragraph.contains("one short paragraph"));
        assert!(!paragraph.contains("\u{2022}"));
        let parsed = crate::json_lite::parse(&body).unwrap();
        assert_eq!(
            parsed.get("model").and_then(|model| model.as_str()),
            Some("llama-3.1-8b-instant")
        );
        // The server must read the transcript exactly once escaped.
        let user = parsed
            .get("messages")
            .and_then(|messages| messages.index(1))
            .and_then(|message| message.get("content"))
            .and_then(|content| content.as_str());
        assert_eq!(user, Some(format!("Transcript:\n{transcript}").as_str()));
    }

    #[test]
    fn chat_completion_response_from_provider_docs() {
        let json = concat!(
            r#"{"id":"chatcmpl-123","object":"chat.completion","created":1,"#,
            r#""model":"llama-3.1-8b-instant","choices":[{"index":0,"#,
            r#""message":{"role":"assistant","content":"Lista de compras\n"#,
            r#"Leche, pan y manzanas."},"logprobs":null,"finish_reason":"stop"}],"#,
            r#""usage":{"prompt_tokens":20,"completion_tokens":8,"total_tokens":28}}"#,
        );
        let content = parse_chat_completion(json).unwrap();
        assert_eq!(content, "Lista de compras\nLeche, pan y manzanas.");
        let (title, summary) = split_title(&content);
        assert_eq!(title, "Lista de compras");
        assert_eq!(summary, "Leche, pan y manzanas.");
    }

    #[test]
    fn split_title_clamps_long_first_lines_and_keeps_the_rest() {
        let (title, summary) =
            split_title("This line is far longer than forty characters in total\nbody");
        assert_eq!(title.chars().count(), 40);
        assert_eq!(summary, "body");
        let (title, summary) = split_title("Solo");
        assert_eq!(title, "Solo");
        assert_eq!(summary, "");
    }

    #[test]
    fn api_errors_read_the_provider_message() {
        let groq = r#"{"error":{"message":"Rate limit reached for model","type":"tokens","code":"rate_limit_exceeded"}}"#;
        assert_eq!(
            api_error(429, groq),
            "HTTP 429: Rate limit reached for model"
        );
        let openrouter = r#"{"error":{"message":"No allowed providers are available for the selected model.","code":404}}"#;
        assert_eq!(
            api_error(404, openrouter),
            "HTTP 404: No allowed providers are available for the selected model."
        );
        assert_eq!(api_error(500, "not json"), "HTTP 500: request failed");
    }
}
