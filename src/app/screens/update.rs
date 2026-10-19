//! `Settings › System › Check for updates`, as the mockup's `Ajustes y
//! reposo` figures: checking, an update available, up to date, downloading,
//! installed and refused. The state is `update_state::UpdateUi`.

use core::convert::Infallible;

use embedded_graphics::{
    pixelcolor::BinaryColor,
    prelude::{Drawable, Point, Primitive, Size},
    primitives::{Circle, Line, PrimitiveStyle, Rectangle, Triangle},
};

use crate::{
    app::{
        screens::games::draw_info_box,
        state::AppState,
        typography::{Text, UiTextStyle},
        widgets::{
            bottom_bar::{draw_bottom_bar, KeyCap},
            chips::{chip_row_width, draw_chip_row, Chip},
            display_numerals::{draw_numerals, Numerals},
            list_row::{draw_list_row, ListRow, LIST_ROW_HEIGHT},
            status_bar::{draw_status_bar, draw_status_text, STATUS_BAR_RIGHT},
        },
    },
    orientation::OrientedFrameBuffer,
    update_state::{Step, UpdatePhase},
};

const CHOOSE_HINTS: [(KeyCap, &str); 3] = [
    (KeyCap::UpDown, "choose"),
    (KeyCap::Select, "select"),
    (KeyCap::Boot, "hold: back"),
];
const CANCEL_HINTS: [(KeyCap, &str); 1] = [(KeyCap::Boot, "hold: cancel")];
const RESTART_HINTS: [(KeyCap, &str); 2] = [(KeyCap::UpDown, "choose"), (KeyCap::Select, "select")];

const CHIPS_TOP: i32 = 62;
const CENTER: i32 = 240;

pub fn render_update(
    display: &mut OrientedFrameBuffer<'_>,
    state: &AppState,
) -> Result<(), Infallible> {
    let ui = &state.update;
    let working = matches!(
        ui.phase(),
        UpdatePhase::Working { .. } | UpdatePhase::Installed(_)
    );
    let title = match ui.phase() {
        UpdatePhase::Working { .. } => state
            .update
            .target()
            .map_or_else(|| "Updating".to_string(), |v| format!("Updating to {v}")),
        UpdatePhase::Installed(version) => format!("Updating to {version}"),
        _ => "Check for updates".to_string(),
    };
    draw_status_bar(display, state.display, &title)?;
    let right = if working {
        state
            .board
            .power
            .and_then(|power| power.battery_percent)
            .map(|percent| format!("{percent}%"))
    } else {
        ui.checked_at().map(str::to_string)
    };
    if let Some(right) = right {
        draw_status_text(display, state.display, &right, STATUS_BAR_RIGHT)?;
    }

    let hints: &[(KeyCap, &str)] = match ui.phase() {
        UpdatePhase::Checking | UpdatePhase::Working { .. } => &CANCEL_HINTS,
        UpdatePhase::Installed(_) => &RESTART_HINTS,
        _ => &CHOOSE_HINTS,
    };
    match ui.phase() {
        UpdatePhase::Checking => draw_checking(display, state)?,
        UpdatePhase::Available(found) => draw_available(display, state, found)?,
        UpdatePhase::UpToDate => draw_up_to_date(display, state)?,
        UpdatePhase::Working { step, done, total } => {
            draw_working(display, state, *step, *done, *total)?;
        }
        UpdatePhase::Installed(version) => draw_installed(display, state, &version.to_string())?,
        UpdatePhase::Failed(failure) => {
            draw_failed(display, state, &failure.title(), &failure.detail())?;
        }
    }
    draw_bottom_bar(display, state.display, hints)
}

/// `1.3 MB`: tenths of a megabyte, in integers.
#[must_use]
pub fn megabytes(bytes: u64) -> String {
    let tenths = bytes * 10 / 1_000_000;
    format!("{}.{} MB", tenths / 10, tenths % 10)
}

fn centered(
    display: &mut OrientedFrameBuffer<'_>,
    text: &str,
    baseline: i32,
    style: UiTextStyle,
) -> Result<(), Infallible> {
    let left = CENTER - style.text_width(text) / 2;
    Text::new(text, Point::new(left, baseline), style).draw(display)?;
    Ok(())
}

/// The four step chips, the current one filled.
fn draw_steps(
    display: &mut OrientedFrameBuffer<'_>,
    state: &AppState,
    current: Step,
) -> Result<(), Infallible> {
    let label = state.display.extrabold_body_style();
    let chips: Vec<Chip<'_>> = Step::ALL
        .iter()
        .map(|step| Chip::new(step.label(), *step == current))
        .collect();
    let left = (480 - chip_row_width(label, &chips, 6)) / 2;
    draw_chip_row(display, left, CHIPS_TOP, 6, label, &chips)
}

/// The rows under the picture, from `top`; the chosen one inverted.
fn draw_rows(
    display: &mut OrientedFrameBuffer<'_>,
    state: &AppState,
    top: i32,
    values: &[&str],
) -> Result<i32, Infallible> {
    let ui = &state.update;
    let mut y = top;
    for (index, label) in ui.rows().iter().enumerate() {
        draw_list_row(
            display,
            state.display,
            y,
            ListRow {
                title: label,
                subtitle: "",
                value: values.get(index).copied().unwrap_or(""),
                selected: index == ui.row(),
            },
        )?;
        y += LIST_ROW_HEIGHT;
    }
    Ok(y)
}

fn draw_checking(
    display: &mut OrientedFrameBuffer<'_>,
    state: &AppState,
) -> Result<(), Infallible> {
    draw_steps(display, state, Step::Check)?;
    centered(
        display,
        "Checking GitHub\u{2026}",
        300,
        state.display.heading_style(),
    )?;
    centered(
        display,
        "Wi-Fi turns on for the check.",
        342,
        state.display.body_style(),
    )?;
    draw_info_box(
        display,
        state.display,
        "Nothing is downloaded until you choose it.",
        420,
    )
}

fn draw_available(
    display: &mut OrientedFrameBuffer<'_>,
    state: &AppState,
    found: &crate::update_state::Found,
) -> Result<(), Infallible> {
    let ink = BinaryColor::On;
    let detail = state.display.detail_style();
    let heading = state.display.heading_style();
    let body = state.display.body_style();
    centered(display, "INSTALLED", 76, detail)?;
    centered(display, &state.update.installed().to_string(), 112, heading)?;
    // The mockup's ▼ between the two versions.
    Triangle::new(
        Point::new(CENTER - 9, 126),
        Point::new(CENTER + 9, 126),
        Point::new(CENTER, 140),
    )
    .into_styled(PrimitiveStyle::with_fill(ink))
    .draw(display)?;
    centered(display, "AVAILABLE", 172, detail)?;
    draw_numerals(
        display,
        &found.version.to_string(),
        Numerals::Temperature,
        (16, 464),
        262,
        ink,
    )?;
    centered(
        display,
        &format!("Release {}", found.release.tag),
        296,
        body,
    )?;

    // Wi-Fi, battery and size, as the mockup's three columns.
    let wifi = state
        .network
        .ssid
        .clone()
        .unwrap_or_else(|| "not set".to_string());
    let battery = state
        .board
        .power
        .and_then(|power| power.battery_percent)
        .map_or_else(|| "?".to_string(), |percent| format!("{percent}%"));
    let size = megabytes(found.size);
    for (left, label, value) in [
        (16, "WI-FI", wifi.as_str()),
        (200, "BATTERY", battery.as_str()),
        (340, "DOWNLOAD", size.as_str()),
    ] {
        Text::new(label, Point::new(left, 340), detail).draw(display)?;
        let fitted = heading.fit(value, 140);
        Text::new(&fitted, Point::new(left, 370), heading).draw(display)?;
    }
    let bottom = draw_rows(display, state, 392, &[size.as_str()])?;
    draw_info_box(
        display,
        state.display,
        "Takes about two minutes. The version you have now stays on the device until the new one has started correctly.",
        bottom + 12,
    )
}

fn draw_up_to_date(
    display: &mut OrientedFrameBuffer<'_>,
    state: &AppState,
) -> Result<(), Infallible> {
    draw_check_icon(display, 150)?;
    centered(
        display,
        &state.update.installed().to_string(),
        262,
        state.display.heading_style(),
    )?;
    centered(
        display,
        "You have the latest version",
        304,
        state.display.heading_style(),
    )?;
    let checked = state
        .update
        .checked_at()
        .map_or_else(String::new, |at| format!("Checked today at {at}"));
    centered(display, &checked, 338, state.display.body_style())?;
    let bottom = draw_rows(display, state, 400, &[])?;
    draw_info_box(
        display,
        state.display,
        "Wi-Fi is turned on only for the check and goes off again when it ends.",
        bottom + 12,
    )
}

fn draw_working(
    display: &mut OrientedFrameBuffer<'_>,
    state: &AppState,
    step: Step,
    done: u64,
    total: u64,
) -> Result<(), Infallible> {
    draw_steps(display, state, step)?;
    let ink = BinaryColor::On;
    let percent = state.update.percent();
    match step {
        Step::Download => {
            draw_numerals(
                display,
                &format!("{percent}%"),
                Numerals::Temperature,
                (16, 464),
                250,
                ink,
            )?;
            // A 3 px frame with the ink growing inside it, as `.big`.
            let (left, top, width, height) = (16, 282, 448, 26);
            Rectangle::new(
                Point::new(left, top),
                Size::new(width as u32, height as u32),
            )
            .into_styled(PrimitiveStyle::with_stroke(ink, 3))
            .draw(display)?;
            let filled = (width - 6) * i32::from(percent) / 100;
            Rectangle::new(
                Point::new(left + 3, top + 3),
                Size::new(filled.max(0) as u32, (height - 6) as u32),
            )
            .into_styled(PrimitiveStyle::with_fill(ink))
            .draw(display)?;
            let amount = if total > 0 {
                format!(
                    "{} of {}",
                    megabytes(done).trim_end_matches(" MB"),
                    megabytes(total)
                )
            } else {
                megabytes(done)
            };
            centered(display, &amount, 360, state.display.heading_style())?;
        }
        Step::Verify => {
            centered(display, "Verifying", 250, state.display.heading_style())?;
            centered(
                display,
                "Size, header and SHA-256",
                292,
                state.display.body_style(),
            )?;
        }
        Step::Check | Step::Restart => {
            centered(
                display,
                "Working\u{2026}",
                250,
                state.display.heading_style(),
            )?;
        }
    }
    draw_info_box(
        display,
        state.display,
        "Do not turn the device off. It writes to the free slot; if the power fails, the current version still starts. The screen refreshes every 10% to save battery.",
        420,
    )
}

fn draw_installed(
    display: &mut OrientedFrameBuffer<'_>,
    state: &AppState,
    version: &str,
) -> Result<(), Infallible> {
    draw_steps(display, state, Step::Restart)?;
    draw_check_icon(display, 150)?;
    centered(display, version, 262, state.display.heading_style())?;
    centered(
        display,
        "Installed and verified",
        304,
        state.display.heading_style(),
    )?;
    centered(
        display,
        "Restart to start it",
        338,
        state.display.body_style(),
    )?;
    let bottom = draw_rows(display, state, 390, &["", "next start"])?;
    draw_info_box(
        display,
        state.display,
        &format!(
            "If the new version does not start, the device goes back to {} by itself. It confirms itself after 30 seconds of running.",
            state.update.installed()
        ),
        bottom + 12,
    )
}

fn draw_failed(
    display: &mut OrientedFrameBuffer<'_>,
    state: &AppState,
    title: &str,
    detail: &str,
) -> Result<(), Infallible> {
    draw_icon_ring(display, 140)?;
    // The mark inside the ring: a bar and a dot.
    let ink = PrimitiveStyle::with_stroke(BinaryColor::On, 5);
    Line::new(Point::new(CENTER, 118), Point::new(CENTER, 146))
        .into_styled(ink)
        .draw(display)?;
    Circle::new(Point::new(CENTER - 3, 158), 7)
        .into_styled(PrimitiveStyle::with_fill(BinaryColor::On))
        .draw(display)?;
    centered(display, title, 262, state.display.heading_style())?;
    let body = state.display.body_style();
    let mut baseline = 300;
    for line in body.wrap(detail, 420) {
        centered(display, &line, baseline, body)?;
        baseline += i32::from(body.line_height());
    }
    let bottom = draw_rows(display, state, baseline.max(400), &[])?;
    draw_info_box(
        display,
        state.display,
        "Every failure leaves the installed version as it was: nothing is written to the other slot until the file's header, version and checksum agree.",
        bottom + 12,
    )
}

/// A ring with its centre `cy` below the status bar, the mockup's 96 px icon.
fn draw_icon_ring(display: &mut OrientedFrameBuffer<'_>, cy: i32) -> Result<(), Infallible> {
    let diameter = 92;
    Circle::new(
        Point::new(CENTER - diameter / 2, cy - diameter / 2),
        diameter as u32,
    )
    .into_styled(PrimitiveStyle::with_stroke(BinaryColor::On, 5))
    .draw(display)
}

/// The ring with a check mark in it.
fn draw_check_icon(display: &mut OrientedFrameBuffer<'_>, cy: i32) -> Result<(), Infallible> {
    draw_icon_ring(display, cy)?;
    let ink = PrimitiveStyle::with_stroke(BinaryColor::On, 6);
    let points = [
        Point::new(CENTER - 26, cy + 2),
        Point::new(CENTER - 8, cy + 20),
        Point::new(CENTER + 28, cy - 18),
    ];
    Line::new(points[0], points[1])
        .into_styled(ink)
        .draw(display)?;
    Line::new(points[1], points[2])
        .into_styled(ink)
        .draw(display)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::megabytes;

    #[test]
    fn sizes_are_whole_tenths_of_a_megabyte() {
        assert_eq!(megabytes(0), "0.0 MB");
        assert_eq!(megabytes(3_200_000), "3.2 MB");
        assert_eq!(megabytes(1_349_999), "1.3 MB");
        assert_eq!(megabytes(12_000_000), "12.0 MB");
    }
}
