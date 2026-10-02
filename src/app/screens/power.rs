//! Settings › Power: battery history, auto-sleep, wake keys and sleep use.

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
            footer::draw_footer,
            header::draw_header,
            list_row::{draw_list_row, ListRow, LIST_ROW_HEIGHT},
            option_list::draw_option_list,
            status_row::{draw_status_row, StatusRow},
        },
    },
    orientation::OrientedFrameBuffer,
    power_settings::PowerSetting,
};

const CHART_TOP: i32 = 180;
const CHART_SIZE: Size = Size::new(436, 140);
/// Height of a 100% bar.
const BAR_MAX: u32 = 120;
const ROWS_TOP: i32 = 372;

pub fn render_power(
    display: &mut OrientedFrameBuffer<'_>,
    state: &AppState,
) -> Result<(), Infallible> {
    let power = state.board.power;
    let percent = match power.and_then(|snapshot| snapshot.battery_percent) {
        Some(percent) => format!("{percent}%"),
        None => "--%".into(),
    };
    let voltage = match power.and_then(|snapshot| snapshot.battery_voltage_mv) {
        Some(mv) => format!("{}.{:02} V", mv / 1000, mv % 1000 / 10),
        None => "-- V".into(),
    };
    let source = match power {
        Some(snapshot) if snapshot.charging => "CHARGING",
        Some(snapshot) if snapshot.vbus_present => "USB",
        Some(_) => "BATTERY",
        None => "--",
    };

    draw_header(display, state.display, "POWER", "BATTERY AND SLEEP")?;
    draw_status_row(
        display,
        state.display,
        StatusRow {
            left: &percent,
            middle: &voltage,
            right: source,
        },
    )?;
    match state.power_ui.picker {
        Some(highlighted) => draw_picker(display, state, highlighted),
        None => draw_overview(display, state),
    }
}

fn draw_overview(
    display: &mut OrientedFrameBuffer<'_>,
    state: &AppState,
) -> Result<(), Infallible> {
    let heading = state.display.heading_style();
    let body = state.display.body_style();

    Text::new("Battery, last 24 hours", Point::new(22, 160), heading).draw(display)?;
    draw_battery_chart(display, state)?;

    for (index, setting) in PowerSetting::ALL.into_iter().enumerate() {
        let row = ListRow {
            title: setting.label(),
            subtitle: match setting {
                PowerSetting::AutoSleep => "Sleep after this long without a key press",
                PowerSetting::WakeKeys => "Keys that wake Wave from sleep",
            },
            value: state.power.value_label(setting),
            selected: state.power_ui.selected == index,
        };
        let top = ROWS_TOP + index as i32 * LIST_ROW_HEIGHT;
        draw_list_row(display, state.display, top, row)?;
    }

    let last_sleep = state
        .last_sleep
        .map_or_else(|| "None yet".into(), |report| report.label());
    let light_sleep = state.light_sleep.label();
    Text::new("Sleep", Point::new(22, 566), heading).draw(display)?;
    info_line(display, 608, "Last sleep", &last_sleep, body)?;
    info_line(display, 648, "Light sleep", &light_sleep, body)?;
    draw_footer(
        display,
        state.display,
        "MOVE  SELECT CHANGE  HOLD BOOT BACK",
    )?;
    Ok(())
}

/// Bars for the battery level at the end of each of the last 24 hours.
fn draw_battery_chart(
    display: &mut OrientedFrameBuffer<'_>,
    state: &AppState,
) -> Result<(), Infallible> {
    let detail = state.display.detail_style();
    Rectangle::new(Point::new(22, CHART_TOP), CHART_SIZE)
        .into_styled(PrimitiveStyle::with_stroke(BinaryColor::On, 1))
        .draw(display)?;
    let hours = match state.board.rtc {
        Some(now) => state.battery_log.last_day(now.epoch_minutes()),
        None => [None; 24],
    };
    let base = CHART_TOP + 10 + BAR_MAX as i32;
    if hours.iter().all(Option::is_none) {
        let note = "No readings yet";
        let left = 240 - detail.text_width(note) / 2;
        Text::new(note, Point::new(left, CHART_TOP + 76), detail).draw(display)?;
    }
    for (hour, level) in hours.into_iter().enumerate() {
        if let Some(level) = level {
            let height = u32::from(level) * BAR_MAX / 100;
            let top_left = Point::new(36 + hour as i32 * 17, base - height as i32);
            Rectangle::new(top_left, Size::new(13, height))
                .into_styled(PrimitiveStyle::with_fill(BinaryColor::On))
                .draw(display)?;
        }
    }
    Text::new("24 h ago", Point::new(22, base + 40), detail).draw(display)?;
    let left = 458 - detail.text_width("Now");
    Text::new("Now", Point::new(left, base + 40), detail).draw(display)?;
    Ok(())
}

fn draw_picker(
    display: &mut OrientedFrameBuffer<'_>,
    state: &AppState,
    highlighted: usize,
) -> Result<(), Infallible> {
    let setting = state.power_ui.setting();
    let (options, current) = state.power.options(setting);
    let heading = state.display.heading_style();
    Text::new(setting.label(), Point::new(22, 160), heading).draw(display)?;
    draw_option_list(display, state.display, 184, &options, current, highlighted)?;
    draw_footer(
        display,
        state.display,
        "MOVE  SELECT CHOOSE  HOLD BOOT CANCEL",
    )?;
    Ok(())
}

/// Label on the left, value flush right.
fn info_line(
    display: &mut OrientedFrameBuffer<'_>,
    baseline: i32,
    label: &str,
    value: &str,
    style: UiTextStyle,
) -> Result<(), Infallible> {
    Text::new(label, Point::new(22, baseline), style).draw(display)?;
    let left = 458 - style.text_width(value);
    Text::new(value, Point::new(left, baseline), style).draw(display)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use embedded_graphics::prelude::Point;

    use crate::{
        app::{render_current_screen, AppState, ScreenRoute},
        buttons::ButtonEvent,
        framebuffer::FrameBuffer,
        power_settings::AutoSleep,
        rtc::RtcDateTime,
    };

    #[test]
    fn picker_changes_auto_sleep_and_draws_the_battery_bars() {
        let mut state = AppState::default();
        state.router.navigate_to(ScreenRoute::Power);
        let now = RtcDateTime {
            year: 2026,
            month: 10,
            day: 2,
            weekday: 5,
            hour: 13,
            minute: 42,
            second: 0,
        };
        state.board.rtc = Some(now);
        state.battery_log.record(now.epoch_minutes(), 100);

        state.apply(ButtonEvent::Select);
        assert_eq!(state.power_ui.picker, Some(2));
        let mut frame = FrameBuffer::new_white();
        render_current_screen(&mut frame, &state).unwrap();

        state.apply(ButtonEvent::Down);
        state.apply(ButtonEvent::Select);
        assert_eq!(state.power.auto_sleep, AutoSleep::Minutes15);
        assert_eq!(state.power_ui.picker, None);

        let mut frame = FrameBuffer::new_white();
        render_current_screen(&mut frame, &state).unwrap();
        // The newest bar (logical x 427, full height) maps to native (y, 479 - x).
        assert_eq!(frame.is_black(Point::new(250, 479 - 427)), Some(true));
    }

    #[test]
    fn back_closes_the_picker_before_leaving_the_screen() {
        let mut state = AppState::default();
        state.router.navigate_to(ScreenRoute::Power);
        state.apply(ButtonEvent::Down);
        state.apply(ButtonEvent::Select);
        state.back();
        assert_eq!(state.active_route(), ScreenRoute::Power);
        assert_eq!(state.power_ui.picker, None);
        state.back();
        assert_eq!(state.active_route(), ScreenRoute::Settings);
    }
}
