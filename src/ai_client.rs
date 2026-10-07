//! OpenAI-compatible request and response bodies for Voice Notes v2. No
//! network access and no keys live here; Phase 7 adds HTTPS and screens.

use crate::json_lite::{self, JsonValue};

/// Content-Type and byte body of the transcription request, paired.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Multipart {
    pub content_type: String,
    pub body: Vec<u8>,
}

/// The `multipart/form-data` body for `POST /audio/transcriptions`: model,
/// language, JSON response format and the WAV file, byte-exact.
#[must_use]
pub fn transcription_request(model: &str, language: &str, wav: &[u8], boundary: &str) -> Multipart {
    let mut body: Vec<u8> = Vec::new();
    let part = |body: &mut Vec<u8>, text: &str| body.extend_from_slice(text.as_bytes());
    for (name, value) in [("model", model), ("language", language)] {
        part(&mut body, "--");
        part(&mut body, boundary);
        part(&mut body, "\r\nContent-Disposition: form-data; name=\"");
        part(&mut body, name);
        part(&mut body, "\"\r\n\r\n");
        part(&mut body, value);
        part(&mut body, "\r\n");
    }
    part(&mut body, "--");
    part(&mut body, boundary);
    part(
        &mut body,
        "\r\nContent-Disposition: form-data; name=\"response_format\"\r\n\r\n",
    );
    part(&mut body, "json\r\n");
    part(&mut body, "--");
    part(&mut body, boundary);
    part(
        &mut body,
        "\r\nContent-Disposition: form-data; name=\"file\"; filename=\"note.wav\"\r\n\
         Content-Type: audio/wav\r\n\r\n",
    );
    body.extend_from_slice(wav);
    part(&mut body, "\r\n--");
    part(&mut body, boundary);
    part(&mut body, "--\r\n");
    Multipart {
        content_type: format!("multipart/form-data; boundary={boundary}"),
        body,
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
/// title line then a short summary in the transcript's language.
#[must_use]
pub fn summary_request(model: &str, transcript: &str) -> String {
    let system = "You summarise voice notes. Reply with a title on the first \
line (at most 40 characters), then a short summary in the same language as the \
transcript.";
    let user = format!("Transcript:\n{transcript}");
    format!(
        "{{\"model\":\"{}\",\"messages\":[{{\"role\":\"system\",\"content\":\"{}\"}},\
{{\"role\":\"user\",\"content\":\"{}\"}}],\"temperature\":0.2,\"max_tokens\":300}}",
        json_lite::escape(model),
        json_lite::escape(system),
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

/// Split the assistant reply into `(title, summary)`. The title is the first
/// line, clamped to 40 characters; the summary is the rest, trimmed.
#[must_use]
pub fn split_title(content: &str) -> (String, String) {
    let (first, rest) = match content.split_once('\n') {
        Some((first, rest)) => (first, rest),
        None => (content, ""),
    };
    let title: String = first.chars().take(40).collect();
    (title, rest.trim().to_string())
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
        api_error, parse_chat_completion, parse_transcription, split_title, summary_request,
        transcription_request,
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
        let request = transcription_request("whisper-large-v3", "es", &wav, "wave7");
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
        let body = summary_request("llama-3.1-8b-instant", transcript);
        assert!(body.contains("temperature\":0.2"));
        assert!(body.contains("max_tokens\":300"));
        assert!(body.contains("at most 40 characters"));
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
