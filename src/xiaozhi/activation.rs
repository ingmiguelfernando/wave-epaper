//! The XiaoZhi OTA check, ported from `main/ota.cc` of xiaozhi-esp32: the
//! device posts what it is and gets the WebSocket address and token back,
//! plus a six-digit code while it is not linked to an account yet.

use crate::json_lite::{self, JsonValue};

/// The official server's OTA endpoint.
pub const OTA_URL: &str = "https://api.tenclass.net/xiaozhi/ota/";
/// How the board names itself to the server.
pub const BOARD_TYPE: &str = "wave-epaper";
pub const BOARD_NAME: &str = "Wave e-paper";
pub const BOARD_MANUFACTURER: &str = "waveshare";
/// The language the server answers the OTA check in.
pub const LANGUAGE: &str = "en-US";

/// One flash partition, as the system information lists it.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Partition {
    pub label: String,
    pub kind: u32,
    pub subtype: u32,
    pub address: u32,
    pub size: u32,
}

/// What the device says about itself.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct DeviceInfo {
    /// `aa:bb:cc:dd:ee:ff`, the Wi-Fi station MAC.
    pub mac: String,
    /// The UUID made on the first call and kept in NVS.
    pub client_id: String,
    pub firmware_version: String,
    /// `Oct 12 2026T15:04:05Z`, the app descriptor's date and time.
    pub compile_time: String,
    pub idf_version: String,
    pub elf_sha256: String,
    pub chip_model: u32,
    pub chip_cores: u32,
    pub chip_revision: u32,
    pub chip_features: u32,
    pub flash_size: u32,
    pub minimum_free_heap: u32,
    pub partitions: Vec<Partition>,
    pub running_partition: String,
    pub ssid: String,
    pub rssi: i32,
    pub channel: u32,
    pub ip: String,
}

/// Where the conversation runs, from the OTA reply.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WebSocketConfig {
    pub url: String,
    pub token: String,
    /// Binary frame version: 1 is plain Opus, 2 and 3 add a header.
    pub version: u16,
}

/// The code to type at xiaozhi.me while the device is not linked.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Activation {
    pub code: String,
    pub message: String,
    pub challenge: Option<String>,
    pub timeout_ms: Option<u64>,
}

/// The parts of the OTA reply this firmware uses; the firmware offer is
/// ignored, as updates come from this project.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OtaReply {
    pub websocket: Option<WebSocketConfig>,
    pub activation: Option<Activation>,
}

/// `wave-epaper/0.12.0`, the User-Agent of every request.
#[must_use]
pub fn user_agent(firmware_version: &str) -> String {
    format!("{BOARD_TYPE}/{firmware_version}")
}

/// Headers of the OTA check and of the activation poll.
#[must_use]
pub fn request_headers(device: &DeviceInfo) -> Vec<(&'static str, String)> {
    vec![
        ("Activation-Version", "1".into()),
        ("Device-Id", device.mac.clone()),
        ("Client-Id", device.client_id.clone()),
        ("User-Agent", user_agent(&device.firmware_version)),
        ("Accept-Language", LANGUAGE.into()),
        ("Content-Type", "application/json".into()),
    ]
}

/// The OTA check's body, shaped as `Board::GetSystemInfoJson` upstream.
#[must_use]
pub fn system_info_json(device: &DeviceInfo) -> String {
    let text = |value: &str| format!("\"{}\"", json_lite::escape(value));
    let partitions = device
        .partitions
        .iter()
        .map(|partition| {
            format!(
                "{{\"label\":{},\"type\":{},\"subtype\":{},\"address\":{},\"size\":{}}}",
                text(&partition.label),
                partition.kind,
                partition.subtype,
                partition.address,
                partition.size
            )
        })
        .collect::<Vec<_>>()
        .join(",");
    let mut json = String::from("{\"version\":2");
    json.push_str(&format!(",\"language\":{}", text(LANGUAGE)));
    json.push_str(&format!(",\"flash_size\":{}", device.flash_size));
    json.push_str(&format!(
        ",\"minimum_free_heap_size\":\"{}\"",
        device.minimum_free_heap
    ));
    json.push_str(&format!(",\"mac_address\":{}", text(&device.mac)));
    json.push_str(&format!(",\"uuid\":{}", text(&device.client_id)));
    json.push_str(",\"chip_model_name\":\"esp32s3\"");
    json.push_str(&format!(
        ",\"chip_info\":{{\"model\":{},\"cores\":{},\"revision\":{},\"features\":{}}}",
        device.chip_model, device.chip_cores, device.chip_revision, device.chip_features
    ));
    json.push_str(&format!(
        ",\"application\":{{\"name\":{},\"version\":{},\"compile_time\":{},\
\"idf_version\":{},\"elf_sha256\":{}}}",
        text(BOARD_TYPE),
        text(&device.firmware_version),
        text(&device.compile_time),
        text(&device.idf_version),
        text(&device.elf_sha256)
    ));
    json.push_str(&format!(",\"partition_table\":[{partitions}]"));
    json.push_str(&format!(
        ",\"ota\":{{\"label\":{}}}",
        text(&device.running_partition)
    ));
    json.push_str(",\"display\":{\"monochrome\":true,\"width\":480,\"height\":800}");
    json.push_str(&format!(
        ",\"board\":{{\"type\":{},\"name\":{},\"manufacturer\":{},\"ssid\":{},\
\"rssi\":{},\"channel\":{},\"ip\":{},\"mac\":{}}}",
        text(BOARD_TYPE),
        text(BOARD_NAME),
        text(BOARD_MANUFACTURER),
        text(&device.ssid),
        device.rssi,
        device.channel,
        text(&device.ip),
        text(&device.mac)
    ));
    json.push('}');
    json
}

/// Read the OTA reply: the WebSocket section and the activation code.
pub fn parse_ota_reply(text: &str) -> Result<OtaReply, String> {
    let value = json_lite::parse(text).map_err(|error| error.to_string())?;
    let field = |section: &JsonValue, key: &str| {
        section
            .get(key)
            .and_then(JsonValue::as_str)
            .map(str::to_owned)
    };
    let websocket = value.get("websocket").and_then(|section| {
        let url = field(section, "url").filter(|url| !url.is_empty())?;
        let version = section
            .get("version")
            .and_then(JsonValue::as_i64)
            .and_then(|version| u16::try_from(version).ok())
            .filter(|version| (1..=3).contains(version))
            .unwrap_or(1);
        Some(WebSocketConfig {
            url,
            token: field(section, "token").unwrap_or_default(),
            version,
        })
    });
    let activation = value.get("activation").and_then(|section| {
        let code = field(section, "code").filter(|code| !code.is_empty())?;
        Some(Activation {
            message: field(section, "message").unwrap_or_else(|| code.clone()),
            code,
            challenge: field(section, "challenge"),
            timeout_ms: section
                .get("timeout_ms")
                .and_then(JsonValue::as_i64)
                .and_then(|timeout| u64::try_from(timeout).ok()),
        })
    });
    Ok(OtaReply {
        websocket,
        activation,
    })
}

/// The activation poll's address: the OTA address plus `activate`.
#[must_use]
pub fn activate_url(ota_url: &str) -> String {
    if ota_url.ends_with('/') {
        format!("{ota_url}activate")
    } else {
        format!("{ota_url}/activate")
    }
}

/// Headers of the WebSocket handshake, one `Name: value\r\n` per line. The
/// token comes from the server, so control characters are dropped from it.
#[must_use]
pub fn websocket_headers(config: &WebSocketConfig, device: &DeviceInfo) -> String {
    let clean = |value: &str| {
        value
            .chars()
            .filter(|character| !character.is_control())
            .collect::<String>()
    };
    let mut headers = String::new();
    let token = clean(&config.token);
    if !token.is_empty() {
        let authorization = if token.contains(' ') {
            token
        } else {
            format!("Bearer {token}")
        };
        headers.push_str(&format!("Authorization: {authorization}\r\n"));
    }
    headers.push_str(&format!("Protocol-Version: {}\r\n", config.version));
    headers.push_str(&format!("Device-Id: {}\r\n", clean(&device.mac)));
    headers.push_str(&format!("Client-Id: {}\r\n", clean(&device.client_id)));
    headers
}

/// `aa:bb:cc:dd:ee:ff`, lower case as upstream.
#[must_use]
pub fn mac_text(mac: [u8; 6]) -> String {
    mac.iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<Vec<_>>()
        .join(":")
}

/// A version 4 UUID from 16 random bytes.
#[must_use]
pub fn uuid_v4(mut bytes: [u8; 16]) -> String {
    bytes[6] = (bytes[6] & 0x0F) | 0x40;
    bytes[8] = (bytes[8] & 0x3F) | 0x80;
    let hex: String = bytes.iter().map(|byte| format!("{byte:02x}")).collect();
    format!(
        "{}-{}-{}-{}-{}",
        &hex[0..8],
        &hex[8..12],
        &hex[12..16],
        &hex[16..20],
        &hex[20..32]
    )
}

#[cfg(test)]
mod tests {
    use super::{
        activate_url, mac_text, parse_ota_reply, request_headers, system_info_json, uuid_v4,
        websocket_headers, DeviceInfo, Partition, WebSocketConfig,
    };
    use crate::json_lite::{self, JsonValue};

    fn device() -> DeviceInfo {
        DeviceInfo {
            mac: "a0:b1:c2:d3:e4:f5".into(),
            client_id: "6f1d1f6c-0b8e-4a8e-9a43-3a4b5c6d7e8f".into(),
            firmware_version: "0.12.0".into(),
            compile_time: "Oct 12 2026T15:04:05Z".into(),
            idf_version: "v5.5.1".into(),
            elf_sha256: "ab".repeat(32),
            chip_model: 9,
            chip_cores: 2,
            chip_revision: 2,
            chip_features: 18,
            flash_size: 16 * 1024 * 1024,
            minimum_free_heap: 123_456,
            partitions: vec![Partition {
                label: "factory".into(),
                kind: 0,
                subtype: 0,
                address: 0x10000,
                size: 0xFA0000,
            }],
            running_partition: "factory".into(),
            ssid: "Casa \"Norte\"".into(),
            rssi: -58,
            channel: 6,
            ip: "192.168.1.40".into(),
        }
    }

    fn text<'a>(value: &'a JsonValue, path: &[&str]) -> Option<&'a str> {
        let mut current = value;
        for key in path {
            current = current.get(key)?;
        }
        current.as_str()
    }

    #[test]
    fn the_system_information_parses_and_keeps_its_fields() {
        let value = json_lite::parse(&system_info_json(&device())).unwrap();
        assert_eq!(text(&value, &["mac_address"]), Some("a0:b1:c2:d3:e4:f5"));
        assert_eq!(text(&value, &["application", "version"]), Some("0.12.0"));
        assert_eq!(text(&value, &["board", "type"]), Some("wave-epaper"));
        // A quote in the network name stays inside its string.
        assert_eq!(text(&value, &["board", "ssid"]), Some("Casa \"Norte\""));
        let table = value.get("partition_table").unwrap();
        assert_eq!(text(table.index(0).unwrap(), &["label"]), Some("factory"));
    }

    #[test]
    fn headers_name_the_device_and_its_client() {
        let headers = request_headers(&device());
        let value = |name: &str| {
            headers
                .iter()
                .find(|(header, _)| *header == name)
                .map(|(_, value)| value.as_str())
        };
        assert_eq!(value("Device-Id"), Some("a0:b1:c2:d3:e4:f5"));
        assert_eq!(
            value("Client-Id"),
            Some("6f1d1f6c-0b8e-4a8e-9a43-3a4b5c6d7e8f")
        );
        assert_eq!(value("User-Agent"), Some("wave-epaper/0.12.0"));
        assert_eq!(value("Activation-Version"), Some("1"));
    }

    #[test]
    fn a_reply_with_a_code_asks_for_linking() {
        let reply = parse_ota_reply(
            r#"{"server_time":{"timestamp":1760000000000},"activation":{"code":"428913","message":"xiaozhi.me\n428913","challenge":"c-1","timeout_ms":30000},"websocket":{"url":"wss://api.tenclass.net/xiaozhi/v1/","token":"test-token"},"firmware":{"version":"9.9.9","url":"http://x"}}"#,
        )
        .unwrap();
        let activation = reply.activation.unwrap();
        assert_eq!(activation.code, "428913");
        assert_eq!(activation.message, "xiaozhi.me\n428913");
        assert_eq!(activation.challenge.as_deref(), Some("c-1"));
        assert_eq!(activation.timeout_ms, Some(30_000));
        let websocket = reply.websocket.unwrap();
        assert_eq!(websocket.url, "wss://api.tenclass.net/xiaozhi/v1/");
        assert_eq!(websocket.token, "test-token");
        assert_eq!(websocket.version, 1);
    }

    #[test]
    fn a_linked_device_gets_only_the_websocket() {
        let json = r#"{"websocket":{"url":"wss://x/ws","version":3}}"#;
        let reply = parse_ota_reply(json).unwrap();
        assert_eq!(reply.activation, None);
        assert_eq!(reply.websocket.unwrap().version, 3);
        // An empty URL leaves nothing to connect to.
        let empty = parse_ota_reply(r#"{"websocket":{"url":""}}"#).unwrap();
        assert_eq!(empty.websocket, None);
        assert!(parse_ota_reply("<html>").is_err());
    }

    #[test]
    fn the_activation_address_appends_one_path_part() {
        assert_eq!(
            activate_url("https://api.tenclass.net/xiaozhi/ota/"),
            "https://api.tenclass.net/xiaozhi/ota/activate"
        );
        assert_eq!(
            activate_url("http://192.168.1.25:8003/xiaozhi/ota"),
            "http://192.168.1.25:8003/xiaozhi/ota/activate"
        );
    }

    #[test]
    fn the_handshake_carries_a_bearer_token_without_line_breaks() {
        let config = WebSocketConfig {
            url: "wss://example.org/ws".into(),
            token: "abc\r\nX-Evil:1".into(),
            version: 1,
        };
        let headers = websocket_headers(&config, &device());
        assert!(headers.starts_with("Authorization: Bearer abcX-Evil:1\r\n"));
        assert!(headers.contains("Protocol-Version: 1\r\n"));
        assert!(headers.contains("Device-Id: a0:b1:c2:d3:e4:f5\r\n"));
        assert_eq!(headers.matches("\r\n").count(), 4);
        // A token that already names its scheme is sent as it is.
        let scheme = WebSocketConfig {
            token: "Token xyz".into(),
            ..config
        };
        let headers = websocket_headers(&scheme, &device());
        assert!(headers.starts_with("Authorization: Token xyz\r\n"));
    }

    #[test]
    fn mac_and_uuid_texts_follow_upstream() {
        assert_eq!(
            mac_text([0xA0, 0xB1, 0x0C, 0, 0xE4, 0xF5]),
            "a0:b1:0c:00:e4:f5"
        );
        let uuid = uuid_v4([0xFF; 16]);
        assert_eq!(uuid, "ffffffff-ffff-4fff-bfff-ffffffffffff");
    }
}
