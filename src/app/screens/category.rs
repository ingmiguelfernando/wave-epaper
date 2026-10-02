//! Generic category list screen with bounded paging.

use core::convert::Infallible;

use crate::{
    app::{
        menu::{category_entries, CATEGORY_PAGE_SIZE},
        state::AppState,
        widgets::{
            key_hints::{draw_key_hints, KeyCap},
            list_row::{draw_list_row, ListRow, LIST_ROW_HEIGHT},
            status_bar::{draw_status_bar, draw_status_text, STATUS_BAR_HEIGHT, STATUS_BAR_RIGHT},
        },
    },
    orientation::OrientedFrameBuffer,
};

const CATEGORY_HINTS: [(KeyCap, &str); 3] = [
    (KeyCap::UpDown, "move"),
    (KeyCap::Select, "open"),
    (KeyCap::Boot, "hold: back"),
];

pub fn render_category(
    display: &mut OrientedFrameBuffer<'_>,
    state: &AppState,
) -> Result<(), Infallible> {
    let route = state.active_route();
    let entries = category_entries(route);
    let selected = state.category_selection(route);
    let page_start = (selected / CATEGORY_PAGE_SIZE) * CATEGORY_PAGE_SIZE;
    let pages = entries.len().max(1).div_ceil(CATEGORY_PAGE_SIZE);
    let status = if pages > 1 {
        format!("{}/{}", page_start / CATEGORY_PAGE_SIZE + 1, pages)
    } else {
        format!("{} items", entries.len())
    };

    draw_status_bar(display, state.display, route.label())?;
    draw_status_text(display, state.display, &status, STATUS_BAR_RIGHT)?;
    let visible = entries.iter().skip(page_start).take(CATEGORY_PAGE_SIZE);
    for (offset, entry) in visible.enumerate() {
        let row = ListRow {
            title: entry.label,
            subtitle: entry.subtitle,
            value: entry.badge,
            selected: page_start + offset == selected,
        };
        let top = STATUS_BAR_HEIGHT + offset as i32 * LIST_ROW_HEIGHT;
        draw_list_row(display, state.display, top, row)?;
    }
    draw_key_hints(display, state.display, &CATEGORY_HINTS)
}
