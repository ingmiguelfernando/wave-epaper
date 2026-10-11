//! Settings › System › Memory: the runtime memory snapshot stored by
//! `main.rs` at each status-bar refresh — internal RAM, its largest block,
//! the lowest free since boot, PSRAM and the main task's stack margin.

use core::convert::Infallible;

use embedded_graphics::prelude::Point;

use crate::{
    app::{
        state::AppState,
        typography::Text,
        widgets::{
            bottom_bar::{draw_bottom_bar, BACK_HINTS},
            list_row::{draw_list_row, ListRow, LIST_ROW_HEIGHT},
            status_bar::{draw_status_bar, draw_status_text, STATUS_BAR_HEIGHT, STATUS_BAR_RIGHT},
        },
    },
    orientation::OrientedFrameBuffer,
    regional::grouped,
    runtime_memory::RuntimeMemorySnapshot,
};

/// Sizes in KB, or in MB above 1 MB, with grouped digits.
#[must_use]
pub fn size_label(bytes: usize) -> String {
    const MB: usize = 1024 * 1024;
    if bytes >= MB {
        let whole = grouped((bytes / MB) as u32);
        let tenth = (bytes % MB) * 10 / MB;
        format!("{whole}.{tenth} MB")
    } else {
        format!("{} KB", grouped((bytes / 1024) as u32))
    }
}

/// The screen's rows, label left and value right, in reading order.
fn memory_rows(state: &AppState, snapshot: &RuntimeMemorySnapshot) -> [(&'static str, String); 6] {
    [
        (
            "Internal RAM free",
            size_label(snapshot.heap_free_internal_bytes),
        ),
        (
            "Largest free block",
            size_label(snapshot.heap_largest_internal_block_bytes),
        ),
        (
            "Lowest since boot",
            size_label(
                state
                    .memory
                    .lowest_internal_bytes
                    .unwrap_or(snapshot.heap_lowest_internal_bytes),
            ),
        ),
        ("PSRAM free", size_label(snapshot.heap_free_psram_bytes)),
        (
            "PSRAM lowest",
            size_label(
                state
                    .memory
                    .lowest_psram_bytes
                    .unwrap_or(snapshot.heap_lowest_psram_bytes),
            ),
        ),
        (
            "Main task stack margin",
            size_label(snapshot.main_stack_high_water_bytes),
        ),
    ]
}

pub fn render_settings_memory(
    display: &mut OrientedFrameBuffer<'_>,
    state: &AppState,
) -> Result<(), Infallible> {
    draw_status_bar(display, state.display, "Memory")?;
    let top = STATUS_BAR_HEIGHT;
    let Some(snapshot) = state.memory.snapshot else {
        draw_status_text(display, state.display, "not measured yet", STATUS_BAR_RIGHT)?;
        let heading = state.display.heading_style();
        let body = state.display.body_style();
        Text::new("No snapshot yet", Point::new(22, top + 56), heading).draw(display)?;
        Text::new(
            "The device measures at each status bar refresh.",
            Point::new(22, top + 112),
            body,
        )
        .draw(display)?;
        return draw_bottom_bar(display, state.display, &BACK_HINTS);
    };
    let summary = size_label(snapshot.heap_free_internal_bytes);
    draw_status_text(display, state.display, &summary, STATUS_BAR_RIGHT)?;
    for (index, (label, value)) in memory_rows(state, &snapshot).into_iter().enumerate() {
        let row = ListRow {
            title: label,
            subtitle: "",
            value: &value,
            selected: false,
        };
        draw_list_row(
            display,
            state.display,
            top + index as i32 * LIST_ROW_HEIGHT,
            row,
        )?;
    }
    draw_bottom_bar(display, state.display, &BACK_HINTS)
}

#[cfg(test)]
mod tests {
    use super::size_label;
    use crate::app::{menu::home_index, state::MemoryUiState, AppState};
    use crate::buttons::ButtonEvent;
    use crate::runtime_memory::RuntimeMemorySnapshot;

    #[test]
    fn sizes_use_kb_and_mb_above_one_megabyte() {
        const MB: usize = 1024 * 1024;
        assert_eq!(size_label(0), "0 KB");
        assert_eq!(size_label(512 * 1024), "512 KB");
        // Grouped digits at the top of the KB range.
        assert_eq!(size_label(1023 * 1024), "1,023 KB");
        assert_eq!(size_label(MB), "1.0 MB");
        assert_eq!(size_label(2 * MB + MB / 2), "2.5 MB");
        assert_eq!(size_label(8 * MB + MB / 4), "8.2 MB");
    }

    #[test]
    fn the_lowest_value_never_goes_up_across_snapshots() {
        let mut memory = MemoryUiState::default();
        let low = RuntimeMemorySnapshot {
            heap_lowest_internal_bytes: 512 * 1024,
            heap_lowest_psram_bytes: 2 * 1024 * 1024,
            ..RuntimeMemorySnapshot::default()
        };
        memory.note_snapshot(low);
        assert_eq!(memory.lowest_internal_bytes, Some(512 * 1024));
        // A later capture reporting a higher low does not raise the shown one.
        let higher = RuntimeMemorySnapshot {
            heap_lowest_internal_bytes: 768 * 1024,
            heap_lowest_psram_bytes: 3 * 1024 * 1024,
            ..RuntimeMemorySnapshot::default()
        };
        memory.note_snapshot(higher);
        assert_eq!(memory.lowest_internal_bytes, Some(512 * 1024));
        assert_eq!(memory.lowest_psram_bytes, Some(2 * 1024 * 1024));
        // A lower one is remembered.
        let lower = RuntimeMemorySnapshot {
            heap_lowest_internal_bytes: 256 * 1024,
            heap_lowest_psram_bytes: 1024 * 1024,
            ..RuntimeMemorySnapshot::default()
        };
        memory.note_snapshot(lower);
        assert_eq!(memory.lowest_internal_bytes, Some(256 * 1024));
        assert_eq!(memory.lowest_psram_bytes, Some(1024 * 1024));
    }

    #[test]
    fn home_settings_system_memory_opens_through_apply() {
        use crate::app::router::ScreenRoute;
        let mut state = AppState::default();
        state.home_selected = home_index(ScreenRoute::Settings).expect("Settings on Home");
        state.apply(ButtonEvent::Select);
        assert_eq!(state.active_route(), ScreenRoute::Settings);
        // System is the last group.
        for _ in 0..8 {
            state.apply(ButtonEvent::Down);
        }
        state.apply(ButtonEvent::Select);
        assert_eq!(state.active_route(), ScreenRoute::SettingsSystem);
        // Memory sits under Device Info.
        state.apply(ButtonEvent::Down);
        state.apply(ButtonEvent::Select);
        assert_eq!(state.active_route(), ScreenRoute::SettingsMemory);
        state.back();
        assert_eq!(state.active_route(), ScreenRoute::SettingsSystem);
    }
}
