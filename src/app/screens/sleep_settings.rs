//! Settings › Sleep screen: the five modes with their battery cost, and the
//! options of the mode in use.

use core::convert::Infallible;

use embedded_graphics::{
    pixelcolor::BinaryColor,
    prelude::{Drawable, Point, Primitive, Size},
    primitives::{Circle, PrimitiveStyle, Rectangle},
};

use crate::{
    app::{
        display::DisplayPreferences,
        state::AppState,
        typography::{Text, UiTextRole},
        widgets::{
            bottom_bar::{draw_bottom_bar, KeyCap, BOTTOM_BAR_TOP, CHOOSE_HINTS},
            list_row::{draw_list_row, ListRow, LIST_ROW_HEIGHT},
            option_list::draw_option_list,
            status_bar::{draw_status_bar, STATUS_BAR_HEIGHT},
        },
    },
    orientation::OrientedFrameBuffer,
    sleep_screen::{daily_cost_label, daily_cost_tenths, SleepMode, SleepScreenSetting, SleepSource},
};

use super::{games::draw_info_box, settings::hours_text};

const LEFT: i32 = 16;
const RIGHT: i32 = 464;
/// Title and description start right of the radio button.
const TEXT_LEFT: i32 = 48;
const INFO: &str = "Costs are estimates of the extra battery a day; Settings › Power shows \
                    what is really used.";

const LIST_HINTS: [(KeyCap, &str); 3] = [
    (KeyCap::UpDown, "move"),
    (KeyCap::Select, "select"),
    (KeyCap::Boot, "preview"),
];

pub fn render_sleep_settings(
    display: &mut OrientedFrameBuffer<'_>,
    state: &AppState,
) -> Result<(), Infallible> {
    if let Some(picture) = &state.sleep_preview {
        display.copy_native_frame(picture);
        return Ok(());
    }
    let preferences = state.display;
    let settings = state.sleep_screen;
    draw_status_bar(display, preferences, "Settings › Sleep screen")?;
    let open = state.sleep_screen_ui.picker.zip(state.sleep_screen_ui.setting(settings));
    if let Some((highlighted, setting)) = open {
        let (options, current) = settings.options(setting);
        let heading = preferences.heading_style();
        let label = Point::new(22, STATUS_BAR_HEIGHT + 40);
        Text::new(setting.label(), label, heading).draw(display)?;
        let top = STATUS_BAR_HEIGHT + 64;
        draw_option_list(display, preferences, top, &options, current, highlighted)?;
        return draw_bottom_bar(display, preferences, &CHOOSE_HINTS);
    }

    let mut top = STATUS_BAR_HEIGHT;
    for (index, mode) in SleepMode::ALL.into_iter().enumerate() {
        let row = ModeRow {
            title: mode.label(),
            description: &mode_description(state, mode),
            status: &mode_status(state, mode),
            active: settings.mode == mode,
            selected: state.sleep_screen_ui.selected == index,
        };
        top = draw_mode_row(display, preferences, top, &row)?;
    }
    let rows = settings.option_rows();
    if !rows.is_empty() {
        let label = match settings.mode {
            SleepMode::Photo => "PHOTO OPTIONS",
            _ => "CLOCK OPTIONS",
        };
        let detail = preferences.detail_style();
        Text::new(label, Point::new(22, top + 30), detail).draw(display)?;
        top += 40;
        for (offset, setting) in rows.iter().copied().enumerate() {
            let row = ListRow {
                title: setting.label(),
                subtitle: option_subtitle(setting),
                value: settings.value_label(setting),
                selected: state.sleep_screen_ui.selected == SleepMode::ALL.len() + offset,
            };
            draw_list_row(display, preferences, top, row)?;
            top += LIST_ROW_HEIGHT;
        }
    }
    let body = preferences.body_style();
    let lines = body.wrap(INFO, RIGHT - LEFT - 24).len() as i32;
    let info_height = lines * (i32::from(body.line_height()) + 2) + 18;
    if top + 16 + info_height <= BOTTOM_BAR_TOP - 8 {
        draw_info_box(display, preferences, INFO, top + 16)?;
    }
    draw_bottom_bar(display, preferences, &LIST_HINTS)
}

/// One mode: radio button, title and description, and its cost on the right.
struct ModeRow<'a> {
    title: &'a str,
    description: &'a str,
    status: &'a str,
    active: bool,
    selected: bool,
}

/// Draw a mode row from `top`; returns where the next row starts.
fn draw_mode_row(
    display: &mut OrientedFrameBuffer<'_>,
    preferences: DisplayPreferences,
    top: i32,
    row: &ModeRow<'_>,
) -> Result<i32, Infallible> {
    let ink = if row.selected {
        BinaryColor::Off
    } else {
        BinaryColor::On
    };
    let title = preferences.text_style(UiTextRole::Heading, ink);
    let detail = preferences.text_style(UiTextRole::Detail, ink);
    let height = (i32::from(title.line_height()) + i32::from(detail.line_height()) + 22).max(64);
    let size = Size::new(480, height as u32);
    if row.selected {
        Rectangle::new(Point::new(0, top), size)
            .into_styled(PrimitiveStyle::with_fill(BinaryColor::On))
            .draw(display)?;
    } else {
        Rectangle::new(Point::new(0, top + height - 1), Size::new(480, 1))
            .into_styled(PrimitiveStyle::with_fill(BinaryColor::On))
            .draw(display)?;
    }
    let middle = top + height / 2;
    Circle::new(Point::new(LEFT, middle - 9), 19)
        .into_styled(PrimitiveStyle::with_stroke(ink, 2))
        .draw(display)?;
    if row.active {
        Circle::new(Point::new(LEFT + 5, middle - 4), 9)
            .into_styled(PrimitiveStyle::with_fill(ink))
            .draw(display)?;
    }
    let status_left = RIGHT - detail.text_width(row.status);
    let status_baseline = middle + detail.cap_height() / 2;
    Text::new(row.status, Point::new(status_left, status_baseline), detail).draw(display)?;
    let width = status_left - 12 - TEXT_LEFT;
    let title_baseline = top + 11 + title.cap_height();
    let text = title.fit(row.title, width);
    Text::new(&text, Point::new(TEXT_LEFT, title_baseline), title).draw(display)?;
    let description_baseline = title_baseline + 8 + detail.cap_height();
    let text = detail.fit(row.description, width);
    Text::new(&text, Point::new(TEXT_LEFT, description_baseline), detail).draw(display)?;
    Ok(top + height)
}

/// What a mode shows, as the mockup words it.
fn mode_description(state: &AppState, mode: SleepMode) -> String {
    let settings = state.sleep_screen;
    let clock = match settings.clock_refresh.minutes() {
        1 => "1 min".to_string(),
        minutes => format!("{minutes} min"),
    };
    let weather = weather_interval(state);
    match mode {
        SleepMode::Photo => {
            let order = settings.order.label().to_lowercase();
            match settings.source {
                SleepSource::Starred => {
                    format!("{} starred · {order}", state.photos.starred.len())
                }
                SleepSource::Folder => format!("Sleep folder · {order}"),
            }
        }
        SleepMode::Clock => format!("Refresh every {clock}"),
        SleepMode::Weather => match weather {
            Ok(Some(minutes)) => format!("Now + 3 days · every {}", hours_text(minutes)),
            Ok(None) => "Now + 3 days · manual updates".into(),
            Err(reason) => reason.into(),
        },
        SleepMode::ClockWeather => match weather {
            Ok(Some(minutes)) => format!("Clock {clock} · weather {}", hours_text(minutes)),
            Ok(None) => format!("Clock {clock} · weather manual"),
            Err(_) => format!("Clock {clock} · weather off"),
        },
        SleepMode::Verse => "From your Bible on the SD".into(),
    }
}

/// The mode's battery cost a day, or why it has none.
fn mode_status(state: &AppState, mode: SleepMode) -> String {
    let weather = weather_interval(state);
    match mode {
        SleepMode::Photo => "NO WAKE-UPS".into(),
        SleepMode::Verse => "SOON".into(),
        SleepMode::Weather if weather.is_err() => "OFF".into(),
        _ => {
            let refresh = state.sleep_screen.clock_refresh;
            let tenths = daily_cost_tenths(mode, refresh, weather.unwrap_or(None));
            tenths.map_or_else(String::new, daily_cost_label)
        }
    }
}

/// The weather update interval in minutes, `None` for manual updates, or
/// why the weather cannot show.
fn weather_interval(state: &AppState) -> Result<Option<u64>, &'static str> {
    match state.weather_config.as_ref() {
        None => Err("Weather is not set up"),
        Some(config) if !config.enabled => Err("Weather is off"),
        Some(config) => Ok((config.refresh_minutes > 0).then_some(config.refresh_minutes)),
    }
}

fn option_subtitle(setting: SleepScreenSetting) -> &'static str {
    match setting {
        SleepScreenSetting::Source => "Starred photos or /RUSTMIX/SLEEP",
        SleepScreenSetting::Order => "Which picture comes next",
        SleepScreenSetting::Fit => "How photos cover the screen",
        SleepScreenSetting::ClockRefresh => "How often the time changes",
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        app::{render_current_screen, AppState, ScreenRoute},
        buttons::ButtonEvent,
        framebuffer::FrameBuffer,
        photos::image::PhotoFit,
        sleep_screen::{ClockRefresh, SleepMode},
    };

    #[test]
    fn rows_open_option_lists_and_apply_the_fit_to_photos() {
        let mut state = AppState::default();
        state.router.navigate_to(ScreenRoute::SleepScreen);
        let mut frame = FrameBuffer::new_white();
        render_current_screen(&mut frame, &state).unwrap();

        state.apply(ButtonEvent::Up);
        state.apply(ButtonEvent::Select);
        assert_eq!(state.sleep_screen_ui.picker, Some(0));
        render_current_screen(&mut frame, &state).unwrap();
        state.apply(ButtonEvent::Down);
        state.apply(ButtonEvent::Select);
        assert_eq!(state.sleep_screen.fit, PhotoFit::Whole);
        assert_eq!(state.photos.fit, PhotoFit::Whole);

        state.apply(ButtonEvent::Select);
        state.back();
        assert_eq!(state.active_route(), ScreenRoute::SleepScreen);
        state.back();
        assert_eq!(state.active_route(), ScreenRoute::Settings);
    }

    #[test]
    fn mode_rows_pick_the_mode_and_its_options_follow() {
        let mut state = AppState::default();
        state.router.navigate_to(ScreenRoute::SleepScreen);
        state.apply(ButtonEvent::Down);
        state.apply(ButtonEvent::Select);
        assert_eq!(state.sleep_screen.mode, SleepMode::Clock);
        let mut frame = FrameBuffer::new_white();
        render_current_screen(&mut frame, &state).unwrap();

        // Five modes and one clock option: Up from the top wraps to it.
        state.sleep_screen_ui.selected = 0;
        state.apply(ButtonEvent::Up);
        assert_eq!(state.sleep_screen_ui.selected, 5);
        state.apply(ButtonEvent::Select);
        state.apply(ButtonEvent::Down);
        state.apply(ButtonEvent::Select);
        assert_eq!(state.sleep_screen.clock_refresh, ClockRefresh::EveryFiveMinutes);

        // Verse is not ready yet.
        state.sleep_screen_ui.selected = 4;
        state.apply(ButtonEvent::Select);
        assert_eq!(state.sleep_screen.mode, SleepMode::Clock);
        render_current_screen(&mut frame, &state).unwrap();
    }

    #[test]
    fn boot_previews_the_next_sleep_picture_until_a_key() {
        let mut state = AppState::default();
        state.router.navigate_to(ScreenRoute::SleepScreen);
        assert!(state.apply_sleep_screen_boot_short_press());
        assert!(state.take_sleep_preview_request());

        let picture = FrameBuffer::new_white();
        state.show_sleep_preview(picture.clone());
        assert!(state.take_full_refresh());
        let mut frame = FrameBuffer::new_white();
        render_current_screen(&mut frame, &state).unwrap();
        assert!(frame == picture);
        state.apply(ButtonEvent::Select);
        assert!(state.sleep_preview.is_none());
        assert_eq!(state.sleep_screen_ui.picker, None);
        assert!(state.take_full_refresh());

        state.show_sleep_preview(picture);
        state.back();
        assert_eq!(state.active_route(), ScreenRoute::SleepScreen);
        assert!(state.sleep_preview.is_none());
    }
}
