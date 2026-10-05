//! Persistent global user-interface typography settings.

use core::convert::Infallible;

use embedded_graphics::{
    pixelcolor::BinaryColor,
    prelude::{Drawable, Point, Primitive, Size},
    primitives::{PrimitiveStyle, Rectangle},
};

use crate::app::typography::{Text, UiTextStyle};

use crate::{
    app::{
        display::DisplaySetting,
        state::AppState,
        widgets::{
            bottom_bar::{draw_bottom_bar, CHANGE_HINTS, CHOOSE_HINTS},
            header::draw_header,
            option_list::draw_option_list,
            status_row::{draw_status_row, StatusRow},
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

    draw_header(display, state.display, "DISPLAY", "FONT AND SIZE")?;
    draw_status_row(
        display,
        state.display,
        StatusRow {
            left: prefs.font_family.compact_label(),
            middle: prefs.font_size.label(),
            right: prefs.persistence_label(),
        },
    )?;
    if let Some(highlighted) = state.display_picker {
        let Some(&setting) = DisplaySetting::ALL.get(state.display_action_selected) else {
            return Ok(());
        };
        let (options, current) = prefs.options(setting);
        Text::new(setting.label(), Point::new(22, 160), heading).draw(display)?;
        draw_option_list(display, state.display, 184, &options, current, highlighted)?;
        draw_bottom_bar(display, state.display, &CHOOSE_HINTS)?;
        return Ok(());
    }
    Text::new("Display preferences", Point::new(22, 160), heading).draw(display)?;

    draw_setting_row(
        display,
        202,
        DisplaySetting::Font.label(),
        prefs.font_family.compact_label(),
        state.display_action_selected == 0,
        body,
    )?;
    draw_setting_row(
        display,
        292,
        DisplaySetting::Size.label(),
        prefs.font_size.label(),
        state.display_action_selected == 1,
        body,
    )?;

    Text::new("Live preview", Point::new(22, 410), heading).draw(display)?;
    Rectangle::new(Point::new(22, 438), Size::new(436, 160))
        .into_styled(PrimitiveStyle::with_stroke(BinaryColor::On, 1))
        .draw(display)?;
    Text::new("Reader", Point::new(44, 500), prefs.navigation_style()).draw(display)?;
    Text::new("Books, progress and bookmarks", Point::new(44, 548), body).draw(display)?;
    Text::new(
        "Hold BOOT to return to Settings.",
        Point::new(22, 666),
        body,
    )
    .draw(display)?;

    draw_bottom_bar(display, state.display, &CHANGE_HINTS)?;
    Ok(())
}

fn draw_setting_row(
    display: &mut OrientedFrameBuffer<'_>,
    top: i32,
    label: &str,
    value: &str,
    selected: bool,
    style: UiTextStyle,
) -> Result<(), Infallible> {
    let border = if selected {
        PrimitiveStyle::with_stroke(BinaryColor::On, 4)
    } else {
        PrimitiveStyle::with_stroke(BinaryColor::On, 1)
    };
    Rectangle::new(Point::new(22, top), Size::new(436, 70))
        .into_styled(border)
        .draw(display)?;
    Text::new(
        if selected { ">" } else { " " },
        Point::new(38, top + 43),
        style,
    )
    .draw(display)?;
    Text::new(label, Point::new(68, top + 43), style).draw(display)?;
    Text::new(value, Point::new(258, top + 43), style).draw(display)?;
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
