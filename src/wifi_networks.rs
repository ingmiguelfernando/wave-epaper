//! The Wi-Fi networks the device knows: the one in `WIFI.TXT` plus the ones
//! saved from the portal, and the choice among them from a scan.
//!
//! Saved networks live in the encrypted NVS (see `espidf::WifiNetworkStore`),
//! never on the card, as the AI keys do. Everything here is plain data so the
//! rules run on the host.

use crate::network_config::validate_credentials;

/// Saved networks, not counting `WIFI.TXT`.
pub const MAX_SAVED: usize = 6;
/// A scan lists at most this many networks to the portal.
pub const MAX_SCAN_ROWS: usize = 20;

/// A network and its password. The password never reaches a log or a screen.
#[derive(Clone, Eq, PartialEq)]
pub struct SavedNetwork {
    pub ssid: String,
    pub password: String,
}

impl core::fmt::Debug for SavedNetwork {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        formatter
            .debug_struct("SavedNetwork")
            .field("ssid", &self.ssid)
            .field("password", &"<redacted>")
            .finish()
    }
}

/// What saving a network did.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Saved {
    Added,
    /// The name was known; its password was replaced.
    Updated,
}

/// The networks saved from the portal, in the order they were first saved.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct NetworkBook {
    networks: Vec<SavedNetwork>,
}

impl NetworkBook {
    #[must_use]
    pub fn networks(&self) -> &[SavedNetwork] {
        &self.networks
    }

    pub fn add(&mut self, ssid: &str, password: &str) -> Result<Saved, String> {
        let ssid = ssid.trim();
        validate_credentials(ssid, password).map_err(|error| error.to_string())?;
        if let Some(known) = self.networks.iter_mut().find(|known| known.ssid == ssid) {
            known.password = password.to_owned();
            return Ok(Saved::Updated);
        }
        if self.networks.len() >= MAX_SAVED {
            return Err(format!(
                "{MAX_SAVED} networks are saved already; remove one first"
            ));
        }
        self.networks.push(SavedNetwork {
            ssid: ssid.to_owned(),
            password: password.to_owned(),
        });
        Ok(Saved::Added)
    }

    pub fn remove(&mut self, ssid: &str) -> bool {
        let before = self.networks.len();
        self.networks.retain(|known| known.ssid != ssid);
        self.networks.len() != before
    }

    /// One line per network, name and password in hex so no character needs
    /// escaping: `6e616d65:70617373`.
    #[must_use]
    pub fn serialized(&self) -> String {
        let mut text = String::new();
        for network in &self.networks {
            text.push_str(&hex(network.ssid.as_bytes()));
            text.push(':');
            text.push_str(&hex(network.password.as_bytes()));
            text.push('\n');
        }
        text
    }

    /// Lines that do not parse, or break the limits, are skipped: a damaged
    /// entry must not hide the others.
    #[must_use]
    pub fn parse(text: &str) -> Self {
        let mut book = Self::default();
        for line in text.lines() {
            let Some((ssid, password)) = line.trim().split_once(':') else {
                continue;
            };
            let (Some(ssid), Some(password)) = (unhex(ssid), unhex(password)) else {
                continue;
            };
            let _ = book.add(&ssid, &password);
        }
        book
    }

    /// The networks to try: `WIFI.TXT` first, then the saved ones, each name
    /// once.
    #[must_use]
    pub fn candidates(&self, primary: Option<SavedNetwork>) -> Vec<SavedNetwork> {
        let mut all: Vec<SavedNetwork> = primary.into_iter().collect();
        for network in &self.networks {
            if !all.iter().any(|known| known.ssid == network.ssid) {
                all.push(network.clone());
            }
        }
        all
    }
}

/// A network the radio heard.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ScanEntry {
    pub ssid: String,
    pub rssi_dbm: i32,
    pub secured: bool,
}

/// The scan as the portal lists it: hidden names dropped, each name once at
/// its strongest, strongest first, at most [`MAX_SCAN_ROWS`].
#[must_use]
pub fn tidy_scan(mut found: Vec<ScanEntry>) -> Vec<ScanEntry> {
    found.retain(|entry| !entry.ssid.is_empty());
    found.sort_by(|a, b| b.rssi_dbm.cmp(&a.rssi_dbm));
    let mut seen: Vec<String> = Vec::new();
    found.retain(|entry| {
        if seen.contains(&entry.ssid) {
            false
        } else {
            seen.push(entry.ssid.clone());
            true
        }
    });
    found.truncate(MAX_SCAN_ROWS);
    found
}

/// The candidate to connect to: the strongest one the scan heard. `None`
/// when it heard none of them; the caller then tries the first.
#[must_use]
pub fn choose(candidates: &[SavedNetwork], scan: &[ScanEntry]) -> Option<usize> {
    let mut best: Option<(usize, i32)> = None;
    for (index, candidate) in candidates.iter().enumerate() {
        let Some(heard) = scan
            .iter()
            .filter(|entry| entry.ssid == candidate.ssid)
            .map(|entry| entry.rssi_dbm)
            .max()
        else {
            continue;
        };
        // An earlier candidate wins a tie.
        if best.is_none_or(|(_, rssi)| heard > rssi) {
            best = Some((index, heard));
        }
    }
    best.map(|(index, _)| index)
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn unhex(text: &str) -> Option<String> {
    if text.len() % 2 != 0 || !text.is_ascii() {
        return None;
    }
    let bytes: Option<Vec<u8>> = (0..text.len())
        .step_by(2)
        .map(|index| u8::from_str_radix(&text[index..index + 2], 16).ok())
        .collect();
    String::from_utf8(bytes?).ok()
}

#[cfg(target_os = "espidf")]
pub mod espidf {
    use anyhow::{ensure, Context, Result};
    use esp_idf_svc::{
        nvs::{EspDefaultNvsPartition, EspNvs, NvsDefault},
        sys,
    };

    use super::{tidy_scan, NetworkBook, ScanEntry, MAX_SAVED};

    const NAMESPACE: &str = "wifi_nets";
    const KEY: &str = "book";
    /// Six networks, name and password in hex, plus separators.
    const BOOK_BUFFER_BYTES: usize = MAX_SAVED * (2 * (32 + 63) + 2) + 16;
    const SCAN_RECORDS: u16 = 40;

    /// The saved networks in the default NVS partition, which this firmware
    /// encrypts.
    #[derive(Clone)]
    pub struct WifiNetworkStore {
        partition: EspDefaultNvsPartition,
    }

    impl WifiNetworkStore {
        #[must_use]
        pub fn new(partition: EspDefaultNvsPartition) -> Self {
            Self { partition }
        }

        fn open(&self) -> Result<EspNvs<NvsDefault>> {
            EspNvs::new(self.partition.clone(), NAMESPACE, true)
                .context("opening the Wi-Fi network store")
        }

        /// The saved networks; a store that does not open has none.
        #[must_use]
        pub fn load(&self) -> NetworkBook {
            let Ok(nvs) = self.open() else {
                return NetworkBook::default();
            };
            let mut buffer = [0_u8; BOOK_BUFFER_BYTES];
            let book = nvs
                .get_str(KEY, &mut buffer)
                .ok()
                .flatten()
                .map(NetworkBook::parse)
                .unwrap_or_default();
            buffer.fill(0);
            book
        }

        pub fn save(&self, book: &NetworkBook) -> Result<()> {
            let text = book.serialized();
            ensure!(text.len() < BOOK_BUFFER_BYTES, "too many networks");
            self.open()?
                .set_str(KEY, &text)
                .context("saving the Wi-Fi networks")
        }
    }

    /// Listen for networks and wait for the answer, about two seconds. The
    /// station must be started. Names come back tidied for a list.
    pub fn scan() -> Result<Vec<ScanEntry>> {
        // SAFETY: plain C calls on a started Wi-Fi driver; the record buffer
        // outlives them and the driver's own list is freed by the last call.
        unsafe {
            let config = sys::wifi_scan_config_t {
                show_hidden: false,
                ..Default::default()
            };
            let started = sys::esp_wifi_scan_start(&config, true);
            ensure!(
                started == sys::ESP_OK,
                "the Wi-Fi scan did not run ({started})"
            );
            let mut count = SCAN_RECORDS;
            let mut records = vec![sys::wifi_ap_record_t::default(); usize::from(count)];
            let read = sys::esp_wifi_scan_get_ap_records(&mut count, records.as_mut_ptr());
            ensure!(
                read == sys::ESP_OK,
                "the Wi-Fi scan list could not be read ({read})"
            );
            records.truncate(usize::from(count));
            let found = records
                .iter()
                .map(|record| {
                    let name = record
                        .ssid
                        .iter()
                        .take_while(|byte| **byte != 0)
                        .copied()
                        .collect::<Vec<u8>>();
                    ScanEntry {
                        ssid: String::from_utf8_lossy(&name).into_owned(),
                        rssi_dbm: i32::from(record.rssi),
                        secured: record.authmode != sys::wifi_auth_mode_t_WIFI_AUTH_OPEN,
                    }
                })
                .collect();
            Ok(tidy_scan(found))
        }
    }
}

#[cfg(test)]
mod behaviour {
    use super::*;

    fn net(ssid: &str) -> SavedNetwork {
        SavedNetwork {
            ssid: ssid.into(),
            password: "password1".into(),
        }
    }

    fn heard(ssid: &str, rssi_dbm: i32) -> ScanEntry {
        ScanEntry {
            ssid: ssid.into(),
            rssi_dbm,
            secured: true,
        }
    }

    #[test]
    fn a_book_saves_updates_and_removes() {
        let mut book = NetworkBook::default();
        assert_eq!(book.add("Home", "homepass1"), Ok(Saved::Added));
        assert_eq!(book.add("Phone", "hotspot-1"), Ok(Saved::Added));
        assert_eq!(book.add(" Home ", "newpass12"), Ok(Saved::Updated));
        assert_eq!(book.networks().len(), 2);
        assert_eq!(book.networks()[0].password, "newpass12");
        assert!(book.remove("Home"));
        assert!(!book.remove("Home"));
        assert_eq!(book.networks()[0].ssid, "Phone");
    }

    #[test]
    fn the_book_refuses_what_wifi_refuses_and_a_seventh_network() {
        let mut book = NetworkBook::default();
        assert!(book.add("", "password1").is_err());
        assert!(book.add(&"x".repeat(33), "password1").is_err());
        assert!(book.add("Short", "1234567").is_err());
        assert!(book.add("Open cafe", "").is_ok());
        for index in 1..MAX_SAVED {
            book.add(&format!("net{index}"), "password1").unwrap();
        }
        let full = book.add("one-too-many", "password1").unwrap_err();
        assert!(full.contains("remove one"), "{full}");
        // A known name still updates when the book is full.
        assert_eq!(book.add("net1", "another-pass"), Ok(Saved::Updated));
    }

    #[test]
    fn the_book_round_trips_any_characters_and_skips_damage() {
        let mut book = NetworkBook::default();
        book.add("Café 5G: «guest»", "pa:ss wörd1").unwrap();
        book.add("Open", "").unwrap();
        let text = book.serialized();
        assert!(!text.contains("Café"), "stored as hex");
        assert_eq!(NetworkBook::parse(&text), book);
        let damaged = format!("zz:zz\nnocolon\n{text}6:\n");
        assert_eq!(NetworkBook::parse(&damaged), book);
    }

    #[test]
    fn the_sd_network_comes_first_and_names_are_not_repeated() {
        let mut book = NetworkBook::default();
        book.add("Phone", "hotspot-1").unwrap();
        book.add("Home", "from-the-portal").unwrap();
        let all = book.candidates(Some(net("Home")));
        let names: Vec<&str> = all.iter().map(|n| n.ssid.as_str()).collect();
        assert_eq!(names, ["Home", "Phone"]);
        assert_eq!(all[0].password, "password1", "WIFI.TXT wins");
        assert_eq!(book.candidates(None).len(), 2);
    }

    #[test]
    fn the_strongest_known_network_in_range_is_chosen() {
        let candidates = [net("Home"), net("Phone"), net("Work")];
        let scan = [
            heard("Neighbour", -30),
            heard("Phone", -48),
            heard("Home", -71),
        ];
        assert_eq!(choose(&candidates, &scan), Some(1));
        assert_eq!(choose(&candidates, &[heard("Neighbour", -30)]), None);
        assert_eq!(choose(&candidates, &[]), None);
        // Equal signal: the earlier candidate.
        let tie = [heard("Work", -50), heard("Home", -50)];
        assert_eq!(choose(&candidates, &tie), Some(0));
        // Two access points of one name count at the stronger.
        let twin = [heard("Home", -80), heard("Home", -40), heard("Phone", -50)];
        assert_eq!(choose(&candidates, &twin), Some(0));
    }

    #[test]
    fn the_scan_list_is_unique_sorted_and_bounded() {
        let mut found = vec![
            heard("", -20),
            heard("B", -70),
            heard("A", -50),
            heard("B", -40),
        ];
        for index in 0..30 {
            found.push(heard(&format!("n{index}"), -90));
        }
        let tidy = tidy_scan(found);
        assert_eq!(tidy.len(), MAX_SCAN_ROWS);
        assert_eq!(tidy[0].ssid, "B");
        assert_eq!(tidy[0].rssi_dbm, -40);
        assert_eq!(tidy[1].ssid, "A");
        assert!(tidy.iter().all(|entry| !entry.ssid.is_empty()));
    }

    #[test]
    fn a_saved_network_never_prints_its_password() {
        let debug = format!("{:?}", net("Home"));
        assert!(debug.contains("<redacted>") && !debug.contains("password1"));
    }
}
