//! Settings › Sleep screen: where sleep pictures come from, their order and
//! how photos fit the screen.

use core::convert::Infallible;

use embedded_graphics::prelude::Point;

use crate::{
    app::{
        state::AppState,
        typography::Text,
        widgets::{
            footer::draw_footer,
            header::draw_header,
            list_row::{draw_list_row, ListRow, LIST_ROW_HEIGHT},
            option_list::draw_option_list,
            status_row::{draw_status_row, StatusRow},
        },
    },
    orientation::OrientedFrameBuffer,
    sleep_screen::{SleepScreenSetting, SleepSource},
};

const ROWS_TOP: i32 = 184;

pub fn render_sleep_settings(
    display: &mut OrientedFrameBuffer<'_>,
    state: &AppState,
) -> Result<(), Infallible> {
    if let Some(picture) = &state.sleep_preview {
        display.copy_native_frame(picture);
        return Ok(());
    }
    let settings = state.sleep_screen;
    let starred = format!("{} starred", state.photos.starred.len());
    draw_header(display, state.display, "SLEEP SCREEN", "WHILE WAVE SLEEPS")?;
    draw_status_row(
        display,
        state.display,
        StatusRow {
            left: &starred,
            middle: settings.order.label(),
            right: settings.fit.marker(),
        },
    )?;
    let heading = state.display.heading_style();
    if let Some(highlighted) = state.sleep_screen_ui.picker {
        let setting = state.sleep_screen_ui.setting();
        let (options, current) = settings.options(setting);
        Text::new(setting.label(), Point::new(22, 160), heading).draw(display)?;
        draw_option_list(display, state.display, 184, &options, current, highlighted)?;
        draw_footer(
            display,
            state.display,
            "MOVE  SELECT CHOOSE  HOLD BOOT CANCEL",
        )?;
        return Ok(());
    }

    Text::new("Sleep pictures", Point::new(22, 160), heading).draw(display)?;
    for (index, setting) in SleepScreenSetting::ALL.into_iter().enumerate() {
        let row = ListRow {
            title: setting.label(),
            subtitle: match setting {
                SleepScreenSetting::Source => "Where the pictures come from",
                SleepScreenSetting::Order => "Which picture comes next",
                SleepScreenSetting::Fit => "How photos cover the screen",
            },
            value: settings.value_label(setting),
            selected: state.sleep_screen_ui.selected == index,
        };
        let top = ROWS_TOP + index as i32 * LIST_ROW_HEIGHT;
        draw_list_row(display, state.display, top, row)?;
    }

    let body = state.display.body_style();
    let help = match settings.source {
        SleepSource::Starred => {
            "Star photos in Photos with a short BOOT press. Each sleep shows one of them, \
             or a picture from /RUSTMIX/SLEEP while none is ready."
        }
        SleepSource::Folder => {
            "BMP pictures in /RUSTMIX/SLEEP, 480\u{d7}800 or 800\u{d7}480. Fit only \
             applies to photos."
        }
    };
    let mut baseline = ROWS_TOP + 3 * LIST_ROW_HEIGHT + 50;
    for line in body.wrap(help, 436) {
        Text::new(&line, Point::new(22, baseline), body).draw(display)?;
        baseline += i32::from(body.line_height()) + 4;
    }
    draw_footer(display, state.display, "MOVE  SELECT CHANGE  BOOT PREVIEW")?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::{
        app::{render_current_screen, AppState, ScreenRoute},
        buttons::ButtonEvent,
        framebuffer::FrameBuffer,
        photos::image::PhotoFit,
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
