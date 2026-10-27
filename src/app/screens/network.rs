//! Wi-Fi, SNTP and explicitly activated SD-card transfer portal screens.

use core::convert::Infallible;

use embedded_graphics::{
    pixelcolor::BinaryColor,
    prelude::{Drawable, Point, Primitive, Size},
    primitives::{PrimitiveStyle, Rectangle},
};

use crate::{
    app::{
        state::AppState,
        typography::{Text, UiTextStyle},
        widgets::{
            bottom_bar::{draw_bottom_bar, KeyCap, BACK_HINTS, OPEN_HINTS},
            header::draw_header,
            status_row::{draw_status_row, StatusRow},
        },
    },
    network::WifiConnectionState,
    orientation::OrientedFrameBuffer,
};

/// Text starts here and boxes end at [`RIGHT`].
const LEFT: i32 = 22;
const RIGHT: i32 = 458;
/// Values of the label/value lines start here.
const VALUE_X: i32 = 176;

const STOP_HINTS: [(KeyCap, &str); 2] = [
    (KeyCap::Select, "stop"),
    (KeyCap::Boot, "hold: stop + back"),
];

const FORGET_HINTS: [(KeyCap, &str); 3] = [
    (KeyCap::UpDown, "move"),
    (KeyCap::Select, "forget"),
    (KeyCap::Boot, "hold: back"),
];

const CONFIRM_HINTS: [(KeyCap, &str); 3] = [
    (KeyCap::Select, "forget"),
    (KeyCap::UpDown, "keep"),
    (KeyCap::Boot, "hold: keep"),
];

/// Known networks: the first row's top, one row every [`ROW_PITCH`], at most
/// [`VISIBLE_ROWS`] (the book holds six).
const LIST_TOP: i32 = 180;
const ROW_PITCH: i32 = 36;
const ROW_HEIGHT: u32 = 34;
const VISIBLE_ROWS: usize = 6;
/// Below the list: the hint line or the forget question.
const QUESTION_TOP: i32 = LIST_TOP + ROW_PITCH * VISIBLE_ROWS as i32 + 8;
const QUESTION_HEIGHT: u32 = 64;
const REGIONAL_Y: i32 = 510;

pub fn render_network(
    display: &mut OrientedFrameBuffer<'_>,
    state: &AppState,
) -> Result<(), Infallible> {
    let heading = state.display.heading_style();
    let body = state.display.body_style();
    let network = &state.network;
    let rssi = network.rssi_label();
    let transfer_label = if state.wifi_transfer.is_active() {
        "Stop Wi-Fi Transfer"
    } else {
        "Start Wi-Fi Transfer"
    };

    draw_header(
        display,
        state.display,
        "NETWORK",
        "WI-FI, TIME AND SD TRANSFER",
    )?;
    draw_status_row(
        display,
        state.display,
        StatusRow {
            left: network.wifi_state.label(),
            middle: network.ntp_state.label(),
            right: state.wifi_transfer.state.label(),
        },
    )?;

    Text::new("Wi-Fi station", Point::new(22, 158), heading).draw(display)?;
    line(display, 206, "State", network.wifi_state.label(), body)?;
    line(display, 246, "SSID", network.ssid_label(), body)?;
    line(display, 286, "IPv4", network.ipv4_label(), body)?;
    line(display, 326, "RSSI", &rssi, body)?;

    Text::new("Actions", Point::new(22, 404), heading).draw(display)?;
    draw_action(
        display,
        448,
        transfer_label,
        state.network_action_selected == 0,
        body,
    )?;
    draw_action(
        display,
        516,
        "Add a Wi-Fi network",
        state.network_action_selected == 1,
        body,
    )?;
    draw_action(
        display,
        584,
        "Known networks",
        state.network_action_selected == 2,
        body,
    )?;
    draw_bottom_bar(display, state.display, &OPEN_HINTS)?;
    Ok(())
}

pub fn render_wifi_transfer(
    display: &mut OrientedFrameBuffer<'_>,
    state: &AppState,
) -> Result<(), Infallible> {
    let heading = state.display.heading_style();
    let body = state.display.body_style();
    let detail = state.display.detail_style();
    let transfer = &state.wifi_transfer;

    if transfer.access_point.is_some() {
        return render_wifi_setup(display, state);
    }
    draw_header(
        display,
        state.display,
        "WI-FI TRANSFER",
        "TEMPORARY LAN SD PORTAL",
    )?;
    draw_status_row(
        display,
        state.display,
        StatusRow {
            left: transfer.state.label(),
            middle: "LAN ONLY",
            right: "RUSTMIX",
        },
    )?;

    Text::new("Portal status", Point::new(22, 164), heading).draw(display)?;
    line(display, 212, "State", transfer.state.label(), body)?;
    line(display, 252, "Open", transfer.url_label(), detail)?;
    line(display, 292, "Code", transfer.code_label(), body)?;
    line(display, 332, "Root", "/RUSTMIX", body)?;

    Text::new("Last request", Point::new(22, 412), heading).draw(display)?;
    full_line(display, 460, &transfer.last_action, detail)?;
    let bytes = format!("{} bytes", transfer.last_bytes);
    full_line(display, 500, &bytes, detail)?;
    if let Some(error) = transfer.error.as_deref() {
        full_line(display, 548, error, detail)?;
    }

    draw_action(display, 640, "Stop and return", true, body)?;
    draw_bottom_bar(display, state.display, &STOP_HINTS)?;
    Ok(())
}

/// The device's own Wi-Fi, waiting for a phone to pick a network.
fn render_wifi_setup(
    display: &mut OrientedFrameBuffer<'_>,
    state: &AppState,
) -> Result<(), Infallible> {
    let heading = state.display.heading_style();
    let body = state.display.body_style();
    let detail = state.display.detail_style();
    let transfer = &state.wifi_transfer;
    let name = transfer.access_point.as_deref().unwrap_or("");

    draw_header(
        display,
        state.display,
        "ADD A NETWORK",
        "SET UP FROM A PHONE",
    )?;
    draw_status_row(
        display,
        state.display,
        StatusRow {
            left: transfer.state.label(),
            middle: "OWN WI-FI",
            right: "SETUP",
        },
    )?;
    Text::new("On your phone", Point::new(22, 164), heading).draw(display)?;
    line(
        display,
        212,
        "1 Join",
        if name.is_empty() { "--" } else { name },
        body,
    )?;
    line(display, 252, "Password", transfer.code_label(), body)?;
    line(display, 292, "2 Open", transfer.url_label(), body)?;
    Text::new("3 Scan, pick the network, type", Point::new(22, 340), body).draw(display)?;
    Text::new("its password and save.", Point::new(22, 380), body).draw(display)?;
    Text::new("Saved", Point::new(22, 440), heading).draw(display)?;
    full_line(display, 488, &transfer.last_action, detail)?;
    if let Some(error) = transfer.error.as_deref() {
        full_line(display, 536, error, detail)?;
    }
    // A phone joined to this Wi-Fi stops sharing its hotspot; the device
    // only hears 2.4 GHz.
    full_line(
        display,
        584,
        "Phone hotspot: Maximize Compatibility on,",
        detail,
    )?;
    full_line(
        display,
        608,
        "Hotspot screen open until the device joins.",
        detail,
    )?;
    draw_action(display, 640, "Done", true, body)?;
    draw_bottom_bar(display, state.display, &STOP_HINTS)?;
    Ok(())
}

pub fn render_network_details(
    display: &mut OrientedFrameBuffer<'_>,
    state: &AppState,
) -> Result<(), Infallible> {
    let heading = state.display.heading_style();
    let body = state.display.body_style();
    let detail = state.display.detail_style();
    let network = &state.network;
    let zone = state.regional.timezone_label_for_rtc(state.board.rtc);
    let error = network.error.as_deref().unwrap_or("none");

    draw_header(display, state.display, "NETWORK DETAILS", "KNOWN NETWORKS")?;
    draw_status_row(
        display,
        state.display,
        StatusRow {
            left: network.wifi_state.label(),
            middle: network.ntp_state.label(),
            right: "DETAILS",
        },
    )?;

    Text::new("Known networks", Point::new(LEFT, 164), heading).draw(display)?;
    let rows = network_rows(state);
    let names = state.saved_networks.len();
    let selected = state.known_networks.selected(names);
    let first = selected.saturating_sub(VISIBLE_ROWS - 1);
    let mut top = LIST_TOP;
    for (index, row) in rows.iter().enumerate().skip(first).take(VISIBLE_ROWS) {
        draw_known_row(display, top, row, names > 0 && index == selected, body)?;
        top += ROW_PITCH;
    }
    let question = state.known_networks.confirming();
    match question {
        Some(name) => draw_question(display, name, body, detail)?,
        None => full_line(
            display,
            QUESTION_TOP + 24,
            "Save more in the Wi-Fi portal.",
            detail,
        )?,
    }

    Text::new("Regional settings", Point::new(LEFT, REGIONAL_Y), heading).draw(display)?;
    line(display, REGIONAL_Y + 40, "Timezone", &zone, body)?;
    line(
        display,
        REGIONAL_Y + 76,
        "RTC storage",
        &state.regional.rtc_storage_label(),
        body,
    )?;
    line(
        display,
        REGIONAL_Y + 112,
        "NTP server",
        &network.ntp_server,
        body,
    )?;

    Text::new("Last error", Point::new(LEFT, REGIONAL_Y + 154), heading).draw(display)?;
    full_line(display, REGIONAL_Y + 186, error, detail)?;
    let hints: &[(KeyCap, &str)] = if names == 0 {
        &BACK_HINTS
    } else if question.is_some() {
        &CONFIRM_HINTS
    } else {
        &FORGET_HINTS
    };
    draw_bottom_bar(display, state.display, hints)?;
    Ok(())
}

/// One known network; the selected one is white on black.
fn draw_known_row(
    display: &mut OrientedFrameBuffer<'_>,
    top: i32,
    row: &str,
    selected: bool,
    style: UiTextStyle,
) -> Result<(), Infallible> {
    let style = if selected {
        Rectangle::new(
            Point::new(LEFT, top),
            Size::new((RIGHT - LEFT) as u32, ROW_HEIGHT),
        )
        .into_styled(PrimitiveStyle::with_fill(BinaryColor::On))
        .draw(display)?;
        style.with_color(BinaryColor::Off)
    } else {
        style
    };
    let row = fit_row(style, row, RIGHT - LEFT - 24);
    let baseline = top + (ROW_HEIGHT as i32 + style.cap_height()) / 2;
    Text::new(&row, Point::new(LEFT + 12, baseline), style).draw(display)?;
    Ok(())
}

/// The forget question, in a box under the list.
fn draw_question(
    display: &mut OrientedFrameBuffer<'_>,
    name: &str,
    body: UiTextStyle,
    detail: UiTextStyle,
) -> Result<(), Infallible> {
    Rectangle::new(
        Point::new(LEFT, QUESTION_TOP),
        Size::new((RIGHT - LEFT) as u32, QUESTION_HEIGHT),
    )
    .into_styled(PrimitiveStyle::with_stroke(BinaryColor::On, 3))
    .draw(display)?;
    let width = RIGHT - LEFT - 24;
    let budget = width - body.text_width("Forget ?");
    let question = format!("Forget {}?", cut(body, name, budget));
    Text::new(&question, Point::new(LEFT + 12, QUESTION_TOP + 28), body).draw(display)?;
    let help = cut(detail, "Select again forgets it; a move keeps it.", width);
    Text::new(&help, Point::new(LEFT + 12, QUESTION_TOP + 52), detail).draw(display)?;
    Ok(())
}

const JOINED: &str = " · joined";

/// The saved networks, one row each; the one the radio joined is marked.
pub(crate) fn network_rows(state: &AppState) -> Vec<String> {
    let joined = state.network.ssid.as_deref();
    let mark = |name: &str| {
        if joined == Some(name) && state.network.wifi_state == WifiConnectionState::Connected {
            format!("{name}{JOINED}")
        } else {
            name.to_owned()
        }
    };
    if state.saved_networks.is_empty() {
        return vec!["None saved (only WIFI.TXT)".into()];
    }
    state.saved_networks.iter().map(|name| mark(name)).collect()
}

fn line(
    display: &mut OrientedFrameBuffer<'_>,
    y: i32,
    label: &str,
    value: &str,
    style: UiTextStyle,
) -> Result<(), Infallible> {
    Text::new(label, Point::new(LEFT, y), style).draw(display)?;
    let value = cut(style, value, RIGHT - VALUE_X);
    Text::new(&value, Point::new(VALUE_X, y), style).draw(display)?;
    Ok(())
}

/// One line from the left margin to the right one, cut if longer.
fn full_line(
    display: &mut OrientedFrameBuffer<'_>,
    y: i32,
    text: &str,
    style: UiTextStyle,
) -> Result<(), Infallible> {
    let text = cut(style, text, RIGHT - LEFT);
    Text::new(&text, Point::new(LEFT, y), style).draw(display)?;
    Ok(())
}

/// `text` as is when it fits `max_width`, or cut with `…`.
pub(crate) fn cut(style: UiTextStyle, text: &str, max_width: i32) -> String {
    if style.text_width(text) <= max_width {
        return text.to_owned();
    }
    let budget = max_width - style.text_width("…");
    let mut width = 0;
    let mut kept = String::new();
    for character in text.chars() {
        width += style.char_advance(character);
        if width > budget {
            break;
        }
        kept.push(character);
    }
    kept.push('…');
    kept
}

/// A known-network row cut to `max_width`, keeping its ` · joined` mark.
fn fit_row(style: UiTextStyle, row: &str, max_width: i32) -> String {
    match row.strip_suffix(JOINED) {
        Some(name) => {
            let name = cut(style, name, max_width - style.text_width(JOINED));
            format!("{name}{JOINED}")
        }
        None => cut(style, row, max_width),
    }
}

fn draw_action(
    display: &mut OrientedFrameBuffer<'_>,
    top: i32,
    label: &str,
    selected: bool,
    style: UiTextStyle,
) -> Result<(), Infallible> {
    Rectangle::new(Point::new(22, top), Size::new(436, 52))
        .into_styled(if selected {
            PrimitiveStyle::with_stroke(BinaryColor::On, 6)
        } else {
            PrimitiveStyle::with_stroke(BinaryColor::On, 2)
        })
        .draw(display)?;
    Text::new(
        if selected { ">" } else { " " },
        Point::new(38, top + 34),
        style,
    )
    .draw(display)?;
    let label = cut(style, label, RIGHT - 68 - 12);
    Text::new(&label, Point::new(68, top + 34), style).draw(display)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{
        cut, fit_row, network_rows, render_network, render_network_details, render_wifi_transfer,
        JOINED, RIGHT,
    };
    use crate::{app::AppState, framebuffer::FrameBuffer, orientation::OrientedFrameBuffer};

    #[test]
    fn network_overview_details_and_transfer_render_without_configuration() {
        let mut frame = FrameBuffer::new_white();
        let mut display = OrientedFrameBuffer::new(&mut frame, Default::default());
        let state = AppState::default();
        render_network(&mut display, &state).unwrap();
        render_network_details(&mut display, &state).unwrap();
        render_wifi_transfer(&mut display, &state).unwrap();
    }

    use crate::app::{
        display::{DisplayPreferences, UiFontFamily, UiFontSize},
        render_current_screen,
        widgets::bottom_bar::{draw_bottom_bar, KeyCap, BACK_HINTS, BOTTOM_BAR_TOP, OPEN_HINTS},
        ScreenRoute,
    };
    use crate::buttons::ButtonEvent;
    use crate::network::WifiConnectionState;
    use crate::orientation::DisplayOrientation;
    use crate::wifi_transfer::{WifiTransferSnapshot, WifiTransferState};
    use embedded_graphics::prelude::Point;

    /// 32 bytes, the longest Wi-Fi name, in the widest letter.
    const LONG: &str = "WWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWW";
    const SHORT: &str = "Home";

    #[derive(Clone, Copy, Debug)]
    enum Page {
        Network,
        Details,
        /// Known networks asking to forget the first name.
        Asking,
        Setup,
        Transfer,
    }
    const PAGES: [Page; 5] = [
        Page::Network,
        Page::Details,
        Page::Asking,
        Page::Setup,
        Page::Transfer,
    ];

    /// The page with every name `name`: the joined network, six saved ones,
    /// the setup and portal lines and an error.
    fn page_state(page: Page, preferences: DisplayPreferences, name: &str) -> AppState {
        let mut state = AppState::default();
        state.display = preferences;
        state.network.wifi_state = WifiConnectionState::Connected;
        state.network.ssid = Some(name.into());
        state.network.ipv4_address = Some("192.168.100.200".into());
        state.network.rssi_dbm = Some(-67);
        state.network.error = Some(format!("{name} refused the password {name}"));
        state.saved_networks = (0..6).map(|n| format!("{name}{n}")).collect();
        state.saved_networks[0] = name.into();
        let transfer = WifiTransferSnapshot {
            state: WifiTransferState::Ready,
            url: Some("http://192.168.100.200/".into()),
            code: Some("12345678".into()),
            last_action: format!("Saved {name}"),
            last_bytes: 1_234_567,
            error: Some(format!("{name} could not be saved, try again")),
            access_point: None,
        };
        match page {
            Page::Network => state.router.navigate_to(ScreenRoute::Network),
            Page::Details => state.router.navigate_to(ScreenRoute::NetworkDetails),
            Page::Asking => {
                state.router.navigate_to(ScreenRoute::NetworkDetails);
                state.apply(ButtonEvent::Select);
            }
            Page::Setup => {
                state.wifi_transfer = WifiTransferSnapshot {
                    access_point: Some("Wave-3FA2".into()),
                    url: Some("http://192.168.4.1/".into()),
                    ..transfer
                };
                state.router.navigate_to(ScreenRoute::WifiTransfer);
            }
            Page::Transfer => {
                state.wifi_transfer = transfer;
                state.router.navigate_to(ScreenRoute::WifiTransfer);
            }
        }
        state
    }

    fn hints(page: Page) -> &'static [(KeyCap, &'static str)] {
        match page {
            Page::Network => &OPEN_HINTS,
            Page::Details => &super::FORGET_HINTS,
            Page::Asking => &super::CONFIRM_HINTS,
            Page::Setup | Page::Transfer => &super::STOP_HINTS,
        }
    }

    fn render(state: &AppState) -> FrameBuffer {
        let mut frame = FrameBuffer::new_white();
        render_current_screen(&mut frame, state).unwrap();
        frame
    }

    /// Logical portrait (x, y) is native (y, 479 - x).
    fn ink(frame: &FrameBuffer, x: i32, y: i32) -> bool {
        frame.is_black(Point::new(y, 479 - x)) == Some(true)
    }

    fn every_preference() -> Vec<DisplayPreferences> {
        let mut all = Vec::new();
        for font_family in UiFontFamily::ALL {
            for font_size in UiFontSize::ALL {
                all.push(DisplayPreferences {
                    font_family,
                    font_size,
                });
            }
        }
        all
    }

    /// Under the bar's top line there is only the bar, and the content ends
    /// at least 8 px above it.
    fn assert_only_the_bar_below(
        screen: &FrameBuffer,
        preferences: DisplayPreferences,
        hints: &[(KeyCap, &str)],
        what: &str,
    ) {
        let mut bar = FrameBuffer::new_white();
        let mut display = OrientedFrameBuffer::new(&mut bar, DisplayOrientation::Portrait);
        draw_bottom_bar(&mut display, preferences, hints).unwrap();
        drop(display);
        for y in BOTTOM_BAR_TOP..800 {
            for x in 0..480 {
                assert!(
                    !ink(screen, x, y) || ink(&bar, x, y),
                    "{what} {preferences:?}: ink at ({x}, {y}) under the bar"
                );
            }
        }
        let last = (0..BOTTOM_BAR_TOP)
            .rev()
            .find(|&y| (0..480).any(|x| ink(screen, x, y)))
            .unwrap();
        assert!(
            last < BOTTOM_BAR_TOP - 8,
            "{what} {preferences:?}: content ends at {last}"
        );
    }

    /// D88: under the bar's top line there is only the bar.
    #[test]
    fn nothing_but_the_bar_is_drawn_under_its_top_line() {
        for preferences in every_preference() {
            for page in PAGES {
                let screen = render(&page_state(page, preferences, LONG));
                assert_only_the_bar_below(&screen, preferences, hints(page), &format!("{page:?}"));
            }
        }
    }

    /// D89: Known networks with zero, one and six names, asking or not, at
    /// every size: the bar is clear and only a listed name is highlighted.
    #[test]
    fn known_networks_fit_with_zero_one_and_six_names() {
        use super::{CONFIRM_HINTS, FORGET_HINTS, LEFT, LIST_TOP};
        for preferences in every_preference() {
            for count in [0, 1, 6] {
                for asking in [false, true] {
                    let mut state = page_state(Page::Details, preferences, LONG);
                    state.saved_networks.truncate(count);
                    // Up from the first row selects the last one.
                    state.apply(ButtonEvent::Up);
                    if asking {
                        state.apply(ButtonEvent::Select);
                    }
                    let screen = render(&state);
                    let hints: &[(KeyCap, &str)] = match (count, asking) {
                        (0, _) => &BACK_HINTS,
                        (_, true) => &CONFIRM_HINTS,
                        (_, false) => &FORGET_HINTS,
                    };
                    let what = format!("{count} names, asking {asking}");
                    assert_only_the_bar_below(&screen, preferences, hints, &what);
                    let last_row = LIST_TOP + 36 * (count.max(1) as i32 - 1) + 2;
                    assert_eq!(
                        ink(&screen, LEFT + 2, last_row),
                        count > 0,
                        "{what} {preferences:?}: the last row is selected"
                    );
                    assert_eq!(
                        ink(&screen, LEFT + 2, LIST_TOP + 2),
                        count == 1,
                        "{what} {preferences:?}: the first row is not"
                    );
                }
            }
        }
    }

    /// D89 through the keys: Network › Known networks, move, ask, cancel,
    /// confirm, and the request carried.
    #[test]
    fn known_networks_forget_through_the_keys() {
        use crate::app::known_networks::NetworkRequest;
        let mut state = AppState::default();
        state.saved_networks = vec!["Home".into(), "Phone".into(), "Office".into()];
        state.router.navigate_to(ScreenRoute::Network);
        state.apply(ButtonEvent::Down);
        state.apply(ButtonEvent::Down);
        state.apply(ButtonEvent::Select);
        assert_eq!(state.active_route(), ScreenRoute::NetworkDetails);
        state.apply(ButtonEvent::Down);
        state.apply(ButtonEvent::Select);
        assert_eq!(state.known_networks.confirming(), Some("Phone"));
        assert_eq!(state.take_network_request(), None, "one press asks only");

        // Hold BOOT drops the question and stays; a second hold leaves.
        state.back();
        assert_eq!(state.active_route(), ScreenRoute::NetworkDetails);
        assert_eq!(state.known_networks.confirming(), None);
        assert_eq!(state.take_network_request(), None);

        state.apply(ButtonEvent::Select);
        state.apply(ButtonEvent::Select);
        assert_eq!(
            state.take_network_request(),
            Some(NetworkRequest::Forget("Phone".into()))
        );
        assert_eq!(state.take_network_request(), None, "taken once");
        // The main loop replaces the list; the row now holds the next name.
        state.saved_networks = vec!["Home".into(), "Office".into()];
        assert_eq!(state.known_networks.selected(2), 1);
        state.back();
        assert_eq!(state.active_route(), ScreenRoute::Network);
    }

    /// D88: a 32-byte name is cut before the right margin on every page (a
    /// last glyph may overhang its advance by a pixel or two).
    #[test]
    fn a_32_byte_name_stops_before_the_right_edge() {
        for preferences in every_preference() {
            for page in PAGES {
                let short = render(&page_state(page, preferences, SHORT));
                let long = render(&page_state(page, preferences, LONG));
                for y in 0..BOTTOM_BAR_TOP {
                    for x in RIGHT + 4..480 {
                        assert_eq!(
                            ink(&short, x, y),
                            ink(&long, x, y),
                            "{page:?} {preferences:?}: the long name reaches ({x}, {y})"
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn long_text_is_cut_with_an_ellipsis_and_keeps_the_joined_mark() {
        let style = DisplayPreferences::default().body_style();
        assert_eq!(cut(style, SHORT, 400), SHORT);
        let cut_long = cut(style, LONG, 200);
        assert!(cut_long.ends_with('…'));
        assert!(style.text_width(&cut_long) <= 200);
        let row = fit_row(style, &format!("{LONG}{JOINED}"), 300);
        assert!(row.ends_with(&format!("…{JOINED}")));
        assert!(style.text_width(&row) <= 300);
    }

    #[test]
    fn known_networks_list_marks_the_joined_one() {
        let mut state = AppState::default();
        assert_eq!(network_rows(&state), ["None saved (only WIFI.TXT)"]);
        state.saved_networks = vec!["Home".into(), "Phone".into()];
        state.network.ssid = Some("Phone".into());
        assert_eq!(network_rows(&state), ["Home", "Phone"], "not joined yet");
        state.network.wifi_state = WifiConnectionState::Connected;
        assert_eq!(network_rows(&state), ["Home", "Phone · joined"]);
        state.saved_networks = (1..=6).map(|n| format!("net{n}")).collect();
        let rows = network_rows(&state);
        assert_eq!(rows.len(), 6, "every saved network has a row");
        assert_eq!(rows[5], "net6");
        let mut frame = FrameBuffer::new_white();
        let mut display = OrientedFrameBuffer::new(&mut frame, Default::default());
        render_network_details(&mut display, &state).unwrap();
    }
}
