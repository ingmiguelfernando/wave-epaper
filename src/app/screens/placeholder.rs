//! Placeholder for apps that are planned but not built yet.

use core::convert::Infallible;

use embedded_graphics::prelude::Point;

use crate::{
    app::{
        router::ScreenRoute,
        state::AppState,
        typography::Text,
        widgets::{
            bottom_bar::{draw_bottom_bar, BACK_HINTS},
            status_bar::{draw_status_bar, draw_status_text, STATUS_BAR_RIGHT},
        },
    },
    orientation::OrientedFrameBuffer,
};

const LEFT: i32 = 24;
const TEXT_WIDTH: i32 = 480 - 2 * LEFT;

pub fn render_placeholder(
    display: &mut OrientedFrameBuffer<'_>,
    state: &AppState,
) -> Result<(), Infallible> {
    let route = state.active_route();
    draw_status_bar(display, state.display, route.label())?;
    draw_status_text(display, state.display, "SOON", STATUS_BAR_RIGHT)?;

    let large = state.display.large_style();
    let body = state.display.body_style();
    Text::new("Coming soon", Point::new(LEFT, 150), large).draw(display)?;
    let mut baseline = 210;
    for line in body.wrap(description(route), TEXT_WIDTH) {
        Text::new(&line, Point::new(LEFT, baseline), body).draw(display)?;
        baseline += i32::from(body.line_height()) + 6;
    }
    draw_bottom_bar(display, state.display, &BACK_HINTS)
}

fn description(route: ScreenRoute) -> &'static str {
    match route {
        ScreenRoute::Bible => {
            "Read the Bible from the SD card, with a verse of the day that can also \
             appear on the sleep screen."
        }
        ScreenRoute::ReadingStats => {
            "Reading time per day, streaks and finished books, collected while you read."
        }
        ScreenRoute::XiaoZhi => "Talk with the XiaoZhi voice assistant from xiaozhi.me.",
        _ => "This app is planned for a later update.",
    }
}
