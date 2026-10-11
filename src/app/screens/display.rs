//! Persistent global user-interface typography settings.

use core::convert::Infallible;

use embedded_graphics::{
    pixelcolor::BinaryColor,
    prelude::{Drawable, Point, Primitive, Size},
    primitives::{PrimitiveStyle, Rectangle},
};

use crate::app::typography::Text;

use crate::{
    app::{
        display::DisplaySetting,
        state::AppState,
        widgets::{
            bottom_bar::{draw_bottom_bar, CHANGE_HINTS, CHOOSE_HINTS},
            list_row::{draw_list_row, ListRow, LIST_ROW_HEIGHT},
            option_list::draw_option_list,
            status_bar::{draw_status_bar, draw_status_text, STATUS_BAR_HEIGHT, STATUS_BAR_RIGHT},
        },
    },
    orientation::OrientedFrameBuffer,
};

pub fn render_display(
    display: &mut OrientedFrameBuffer<'_>,
    state: &AppState,
) -> Result<(), Infallible> {
    let heading = state.display.heading_style();
    let body = state.display.body_style();
    let prefs = state.display;

    draw_status_bar(display, state.display, "Settings › Display")?;
    draw_status_text(
        display,
        state.display,
        prefs.persistence_label(),
        STATUS_BAR_RIGHT,
    )?;
    let top = STATUS_BAR_HEIGHT;
    if let Some(highlighted) = state.display_picker {
        let Some(&setting) = DisplaySetting::ALL.get(state.display_action_selected) else {
            return Ok(());
        };
        let (options, current) = prefs.options(setting);
        Text::new(setting.label(), Point::new(22, top + 40), heading).draw(display)?;
        draw_option_list(
            display,
            state.display,
            top + 64,
            &options,
            current,
            highlighted,
        )?;
        draw_bottom_bar(display, state.display, &CHOOSE_HINTS)?;
        return Ok(());
    }
    for (index, setting) in DisplaySetting::ALL.iter().copied().enumerate() {
        let (options, current) = prefs.options(setting);
        let value = options.get(current).copied().unwrap_or("");
        let row = ListRow {
            title: setting.label(),
            subtitle: "",
            value,
            selected: state.display_action_selected == index,
        };
        draw_list_row(
            display,
            state.display,
            top + index as i32 * LIST_ROW_HEIGHT,
            row,
        )?;
    }

    let rows_bottom = top + DisplaySetting::ALL.len() as i32 * LIST_ROW_HEIGHT;
    Text::new("Live preview", Point::new(22, rows_bottom + 44), heading).draw(display)?;
    let box_top = rows_bottom + 72;
    Rectangle::new(Point::new(22, box_top), Size::new(436, 160))
        .into_styled(PrimitiveStyle::with_stroke(BinaryColor::On, 1))
        .draw(display)?;
    Text::new(
        "Reader",
        Point::new(44, box_top + 62),
        prefs.navigation_style(),
    )
    .draw(display)?;
    Text::new(
        "Books, progress and bookmarks",
        Point::new(44, box_top + 110),
        body,
    )
    .draw(display)?;
    Text::new(
        "Hold BOOT to return to Settings.",
        Point::new(22, box_top + 204),
        body,
    )
    .draw(display)?;

    draw_bottom_bar(display, state.display, &CHANGE_HINTS)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::{
        app::{
            display::{DisplayPreferences, UiFontFamily},
            render_current_screen, AppState, ScreenRoute,
        },
        buttons::ButtonEvent,
        framebuffer::FrameBuffer,
    };

    #[test]
    fn picker_wraps_and_applies_the_highlighted_choice() {
        let mut state = AppState::default();
        state.router.navigate_to(ScreenRoute::Display);
        state.apply(ButtonEvent::Select);
        assert_eq!(state.display_picker, Some(0));
        let mut frame = FrameBuffer::new_white();
        render_current_screen(&mut frame, &state).unwrap();
        state.apply(ButtonEvent::Up);
        assert_eq!(state.display_picker, Some(1));
        state.apply(ButtonEvent::Select);
        assert_eq!(
            state.display.font_family,
            UiFontFamily::AtkinsonHyperlegible
        );
        assert_eq!(state.display_picker, None);
        let mut frame = FrameBuffer::new_white();
        render_current_screen(&mut frame, &state).unwrap();
    }

    #[test]
    fn current_choice_closes_picker_without_changing_display_preferences() {
        let mut state = AppState::default();
        state.router.navigate_to(ScreenRoute::Display);
        for _ in 0..2 {
            let original = state.display;
            state.apply(ButtonEvent::Select);
            assert!(state.display_picker.is_some());
            state.apply(ButtonEvent::Select);
            assert_eq!(state.display_picker, None);
            assert_eq!(state.display, original);
            assert_eq!(state.active_route(), ScreenRoute::Display);
            state.apply(ButtonEvent::Down);
        }
    }

    #[test]
    fn back_closes_the_picker_before_leaving_the_screen() {
        let mut state = AppState::default();
        state.router.navigate_to(ScreenRoute::Display);
        state.apply(ButtonEvent::Down);
        state.apply(ButtonEvent::Select);
        assert_eq!(state.display_picker, Some(1));
        state.back();
        assert_eq!(state.active_route(), ScreenRoute::Display);
        assert_eq!(state.display_picker, None);
        assert_eq!(state.display, DisplayPreferences::default());
        state.back();
        assert_eq!(state.active_route(), ScreenRoute::Settings);
    }
}
