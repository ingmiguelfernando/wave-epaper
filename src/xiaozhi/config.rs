//! The XiaoZhi server the call talks to: the official one unless the owner
//! writes another address in `/RUSTMIX/XIAOZHI.TXT` on the card.
//!
//! The file is one line, `server=https://host/xiaozhi/ota/`, the address a
//! xiaozhi-esp32 firmware calls an OTA URL. The server's reply names the
//! conversation WebSocket and, when the device is new to it, a linking code.
//! `http://` works for a server on the home network; `https://` needs a
//! certificate from a public authority, since the device holds no others.

use super::activation::OTA_URL;

/// The official server's address.
pub const OFFICIAL_URL: &str = OTA_URL;

/// Optional server file on the card.
pub const CONFIG_PATH: &str = "/sdcard/RUSTMIX/XIAOZHI.TXT";
const MAX_URL_BYTES: usize = 200;
/// What the call screen says when the file is refused.
pub const BAD_FILE_MESSAGE: &str = "XIAOZHI.TXT on the card is not a valid server address.";

/// The OTA address the file asks for, or the official one when the file is
/// missing or empty. A file that cannot be understood is an error, so a typo
/// never sends the device's identity to the wrong server unseen.
pub fn server_url(contents: Option<&str>) -> Result<String, String> {
    let Some(contents) = contents else {
        return Ok(OTA_URL.into());
    };
    let mut url = None;
    for (index, raw) in contents.lines().enumerate() {
        let line = raw.trim().trim_start_matches('\u{feff}');
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            return Err(format!("line {} must be server=ADDRESS", index + 1));
        };
        if key.trim() != "server" {
            return Err(format!(
                "line {} uses unknown key {:?}",
                index + 1,
                key.trim()
            ));
        }
        if url.replace(value.trim().to_owned()).is_some() {
            return Err("server is given twice".into());
        }
    }
    match url {
        None => Ok(OTA_URL.into()),
        Some(url) if url.is_empty() => Ok(OTA_URL.into()),
        Some(url) => check(&url).map(|()| url),
    }
}

fn check(url: &str) -> Result<(), String> {
    let rest = url
        .strip_prefix("https://")
        .or_else(|| url.strip_prefix("http://"))
        .ok_or("the server address must start with http:// or https://")?;
    let host = rest.split(['/', '?']).next().unwrap_or("");
    if host.is_empty() || host.starts_with(':') {
        return Err("the server address has no host".into());
    }
    if url.len() > MAX_URL_BYTES {
        return Err("the server address is too long".into());
    }
    if url.bytes().any(|byte| byte <= b' ' || byte >= 0x7f) {
        return Err("the server address has spaces or odd characters".into());
    }
    Ok(())
}

/// The host of an address, for the screen and the log (never the path, which
/// can carry a secret).
#[must_use]
pub fn host(url: &str) -> &str {
    let rest = url.split_once("://").map_or(url, |(_, rest)| rest);
    rest.split(['/', '?']).next().unwrap_or(rest)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_file_or_an_empty_one_means_the_official_server() {
        assert_eq!(server_url(None).unwrap(), OTA_URL);
        assert_eq!(server_url(Some("")).unwrap(), OTA_URL);
        assert_eq!(server_url(Some("# nothing\n\nserver=\n")).unwrap(), OTA_URL);
    }

    #[test]
    fn a_home_server_is_taken_as_written() {
        let url = server_url(Some(
            "# mine\nserver = http://192.168.1.20:8002/xiaozhi/ota/\n",
        ))
        .unwrap();
        assert_eq!(url, "http://192.168.1.20:8002/xiaozhi/ota/");
        assert_eq!(host(&url), "192.168.1.20:8002");
        assert_eq!(
            server_url(Some("\u{feff}server=https://xz.example.org/ota")).unwrap(),
            "https://xz.example.org/ota"
        );
    }

    #[test]
    fn a_file_that_cannot_be_understood_is_refused_not_ignored() {
        for text in [
            "server=ftp://x/ota",
            "server=example.org/ota",
            "server=example.org",
            "server=http://",
            "server=http:///ota",
            "server=http://a b/ota",
            "serve=http://x/ota",
            "http://x/ota",
            "server=http://a/\nserver=http://b/",
        ] {
            assert!(server_url(Some(text)).is_err(), "{text:?}");
        }
        let long = format!("server=http://x/{}", "a".repeat(200));
        assert!(server_url(Some(&long)).is_err());
    }

    #[test]
    fn the_host_hides_the_path() {
        assert_eq!(
            host("https://api.tenclass.net/xiaozhi/ota/"),
            "api.tenclass.net"
        );
        assert_eq!(host("http://h:1/x?token=secret"), "h:1");
    }
}
