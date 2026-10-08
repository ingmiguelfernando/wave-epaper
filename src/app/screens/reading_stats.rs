//! Reading statistics screen drawn from `ReadingStats`. Drawing only: the
//! Reader wiring, ranges and routes stay with the main line.

use core::convert::Infallible;

use embedded_graphics::{
    draw_target::DrawTarget,
    pixelcolor::BinaryColor,
    prelude::{Drawable, Pixel, Point, Primitive, Size},
    primitives::{PrimitiveStyle, Rectangle},
};

use crate::{
    app::{
        display::DisplayPreferences,
        typography::{Text, TextBounds, UiTextStyle},
        widgets::{
            bottom_bar::{draw_bottom_bar, BACK_HINTS},
            header::draw_header,
        },
    },
    civil_date,
    orientation::OrientedFrameBuffer,
    reading_stats::ReadingStats,
    regional::grouped,
};

/// The book open in the Reader, drawn as the last summary block.
pub struct CurrentBook<'a> {
    pub title: &'a str,
    pub path: &'a str,
    pub percent: Option<u8>,
}

/// Weekday letters under the bars and heatmap, Monday first as in the mockup.
const WEEK_DAYS: [&str; 7] = ["M", "T", "W", "T", "F", "S", "S"];

pub fn render_reading_stats(
    display: &mut OrientedFrameBuffer<'_>,
    preferences: DisplayPreferences,
    stats: &ReadingStats,
    today: u32,
    current: Option<&CurrentBook<'_>>,
) -> Result<(), Infallible> {
    draw_header(display, preferences, "READING STATS", "THIS WEEK")?;
    draw_tiles(display, preferences, stats, today)?;
    draw_minutes_chart(display, preferences, stats, today)?;
    draw_heatmap_and_year(display, preferences, stats, today)?;
    if let Some(current) = current {
        draw_current_book(display, preferences, stats, current)?;
    }
    draw_bottom_bar(display, preferences, &BACK_HINTS)
}

/// Nothing is recorded until the clock is set, so the screen says how.
pub fn render_without_clock(
    display: &mut OrientedFrameBuffer<'_>,
    preferences: DisplayPreferences,
) -> Result<(), Infallible> {
    draw_header(display, preferences, "READING STATS", "NO CLOCK")?;
    let body = preferences.body_style();
    let text = "Reading time is recorded once the clock is set: \
                Settings › Clock & Alarms › Clock, or Wi-Fi time sync.";
    let mut baseline = 140;
    for line in body.wrap(text, 432) {
        Text::new(&line, Point::new(24, baseline), body).draw(display)?;
        baseline += i32::from(body.line_height()) + 4;
    }
    draw_bottom_bar(display, preferences, &BACK_HINTS)
}

fn draw_tiles(
    display: &mut OrientedFrameBuffer<'_>,
    preferences: DisplayPreferences,
    stats: &ReadingStats,
    today: u32,
) -> Result<(), Infallible> {
    let monday = monday_of(today);
    let elapsed = today - monday;
    let this_week = stats.total(monday, today).seconds / 60;
    let last_week = stats
        .total(monday.saturating_sub(7), monday.saturating_sub(7) + elapsed)
        .seconds
        / 60;
    let tiles = [
        (
            "TODAY",
            minutes_label(stats.day(today).seconds / 60),
            format!("{} pages", stats.day(today).pages),
        ),
        (
            "STREAK",
            format!("{} days", stats.streak(today)),
            format!("best {}", stats.best_streak()),
        ),
        (
            "THIS WEEK",
            minutes_label(this_week),
            change_label(this_week, last_week),
        ),
    ];
    for (index, (label, value, sub)) in tiles.iter().enumerate() {
        let left = 16 + index as i32 * 152;
        let bounds = TextBounds::new(left + 10, 88, left + 138, 174);
        Rectangle::new(Point::new(left, 84), Size::new(144, 92))
            .into_styled(PrimitiveStyle::with_stroke(BinaryColor::On, 2))
            .draw(display)?;
        fit_line(display, label, preferences.detail_style(), bounds, 108)?;
        fit_line(display, value, preferences.heading_style(), bounds, 142)?;
        // The change label contains '%', which is broken at the Detail size.
        fit_line(display, sub, preferences.body_style(), bounds, 170)?;
    }
    Ok(())
}

fn draw_minutes_chart(
    display: &mut OrientedFrameBuffer<'_>,
    preferences: DisplayPreferences,
    stats: &ReadingStats,
    today: u32,
) -> Result<(), Infallible> {
    fit_line(
        display,
        "MINUTES PER DAY",
        preferences.detail_style(),
        TextBounds::new(16, 192, 464, 216),
        210,
    )?;
    const BAR_WIDTH: i32 = 40;
    const BAR_PITCH: i32 = 64;
    const CHART_BOTTOM: i32 = 362;
    const CHART_HEIGHT: i32 = 130;
    let monday = monday_of(today);
    let mut minutes = [0_u32; 7];
    let mut max = 1_u32;
    for (index, slot) in minutes.iter_mut().enumerate() {
        let day = monday + index as u32;
        if day <= today {
            *slot = stats.day(day).seconds / 60;
            max = max.max(*slot);
        }
    }
    Rectangle::new(Point::new(28, CHART_BOTTOM), Size::new(424, 3))
        .into_styled(PrimitiveStyle::with_fill(BinaryColor::On))
        .draw(display)?;
    for (index, slot) in minutes.iter().copied().enumerate() {
        let left = 28 + index as i32 * BAR_PITCH;
        let day = monday + index as u32;
        let style = preferences.detail_style();
        let label = WEEK_DAYS[index];
        let x = left + (BAR_WIDTH - style.text_width(label)) / 2;
        Text::new(label, Point::new(x, CHART_BOTTOM + 28), style).draw(display)?;
        if day > today {
            continue;
        }
        // Elapsed days show the mockup's 2 px stub and "0" when idle.
        let height = ((i32::try_from(slot).unwrap_or(i32::MAX) * CHART_HEIGHT) / max as i32).max(2);
        let bar = Rectangle::new(
            Point::new(left, CHART_BOTTOM - height),
            Size::new(BAR_WIDTH as u32, height as u32),
        );
        if day == today {
            bar.into_styled(PrimitiveStyle::with_stroke(BinaryColor::On, 2))
                .draw(display)?;
            hatch(display, left, CHART_BOTTOM - height, BAR_WIDTH, height)?;
        } else {
            bar.into_styled(PrimitiveStyle::with_fill(BinaryColor::On))
                .draw(display)?;
        }
        let value = slot.to_string();
        let x = left + (BAR_WIDTH - style.text_width(&value)) / 2;
        Text::new(&value, Point::new(x, CHART_BOTTOM - height - 8), style).draw(display)?;
    }
    Ok(())
}

/// Diagonal stripes, the mockup's fill for today's bar.
fn hatch(
    display: &mut OrientedFrameBuffer<'_>,
    left: i32,
    top: i32,
    width: i32,
    height: i32,
) -> Result<(), Infallible> {
    for y in 0..height {
        for x in 0..width {
            if (x + y) % 7 < 3 {
                display.draw_iter(core::iter::once(Pixel(
                    Point::new(left + x, top + y),
                    BinaryColor::On,
                )))?;
            }
        }
    }
    Ok(())
}

fn draw_heatmap_and_year(
    display: &mut OrientedFrameBuffer<'_>,
    preferences: DisplayPreferences,
    stats: &ReadingStats,
    today: u32,
) -> Result<(), Infallible> {
    let (year, _, _) = civil_date::civil_from_days(i64::from(today));
    fit_line(
        display,
        "LAST 12 WEEKS",
        preferences.detail_style(),
        TextBounds::new(16, 404, 250, 428),
        422,
    )?;
    fit_line(
        display,
        &format!("{year} SO FAR"),
        preferences.detail_style(),
        TextBounds::new(276, 404, 464, 428),
        422,
    )?;
    const CELL: i32 = 16;
    const GAP: i32 = 3;
    let monday = monday_of(today);
    let first_week = monday.saturating_sub(11 * 7);
    for column in 0..12_i32 {
        for row in 0..7_i32 {
            let day = first_week + (column * 7 + row) as u32;
            let top = 436 + row * (CELL + GAP);
            let left = 16 + column * (CELL + GAP);
            let cell = Rectangle::new(Point::new(left, top), Size::new(CELL as u32, CELL as u32));
            if day > today {
                dotted_cell(display, left, top)?;
                continue;
            }
            let minutes = stats.day(day).seconds / 60;
            if minutes >= 30 {
                cell.into_styled(PrimitiveStyle::with_fill(BinaryColor::On))
                    .draw(display)?;
                continue;
            }
            cell.into_styled(PrimitiveStyle::with_stroke(BinaryColor::On, 2))
                .draw(display)?;
            if minutes >= 15 {
                Rectangle::new(Point::new(left + 5, top + 5), Size::new(7, 7))
                    .into_styled(PrimitiveStyle::with_fill(BinaryColor::On))
                    .draw(display)?;
            } else if minutes >= 5 {
                Rectangle::new(Point::new(left + 6, top + 6), Size::new(5, 5))
                    .into_styled(PrimitiveStyle::with_fill(BinaryColor::On))
                    .draw(display)?;
            }
        }
    }
    let year_total = stats.total(first_of_year(today), today);
    let hours = year_total.seconds / 3600;
    let rate = if year_total.seconds == 0 {
        0
    } else {
        u64::from(year_total.pages) * 3600 / u64::from(year_total.seconds)
    };
    // Mockup order: books first, then pages, reading and pace.
    let finished = stats.books_finished();
    let books_label = if finished == 1 {
        "book finished"
    } else {
        "books finished"
    };
    let rows = [
        (finished.to_string(), books_label),
        (grouped(year_total.pages), "pages"),
        (format!("{hours} h"), "reading"),
        (rate.to_string(), "pages / hour"),
    ];
    let body = preferences.body_style();
    for (index, (value, label)) in rows.iter().enumerate() {
        stat_line(
            display,
            preferences,
            value,
            label,
            276,
            452 + index as i32 * i32::from(body.line_height()),
        )?;
    }
    Ok(())
}

/// Dotted outline for days that have not happened yet.
fn dotted_cell(
    display: &mut OrientedFrameBuffer<'_>,
    left: i32,
    top: i32,
) -> Result<(), Infallible> {
    const CELL: i32 = 16;
    for step in (0..CELL).step_by(4) {
        for (x, y) in [
            (left + step, top),
            (left + step, top + CELL - 2),
            (left, top + step),
            (left + CELL - 2, top + step),
        ] {
            Rectangle::new(Point::new(x, y), Size::new(2, 2))
                .into_styled(PrimitiveStyle::with_fill(BinaryColor::On))
                .draw(display)?;
        }
    }
    Ok(())
}

/// Bold value followed by the plain label, as in the mockup's year column.
fn stat_line(
    display: &mut OrientedFrameBuffer<'_>,
    preferences: DisplayPreferences,
    value: &str,
    label: &str,
    left: i32,
    baseline: i32,
) -> Result<(), Infallible> {
    let value_style = preferences.heading_style();
    let label_style = preferences.body_style();
    Text::new(value, Point::new(left, baseline), value_style).draw(display)?;
    let label_left = left + value_style.text_width(value) + 8;
    Text::new(label, Point::new(label_left, baseline), label_style).draw(display)?;
    Ok(())
}

fn draw_current_book(
    display: &mut OrientedFrameBuffer<'_>,
    preferences: DisplayPreferences,
    stats: &ReadingStats,
    current: &CurrentBook<'_>,
) -> Result<(), Infallible> {
    let top = 582;
    Rectangle::new(Point::new(16, top), Size::new(448, 122))
        .into_styled(PrimitiveStyle::with_stroke(BinaryColor::On, 2))
        .draw(display)?;
    let bounds = TextBounds::new(30, top + 10, 450, top + 112);
    fit_line(
        display,
        "CURRENT BOOK",
        preferences.detail_style(),
        bounds,
        top + 30,
    )?;
    fit_line(
        display,
        current.title,
        preferences.large_style(),
        bounds,
        top + 66,
    )?;
    let read = stats.book(current.path).map_or(0, |totals| totals.seconds);
    let summary = book_summary(read, current.percent);
    // The percentage belongs to this Body line, never the Detail size.
    fit_line(
        display,
        &summary,
        preferences.body_style(),
        bounds,
        top + 102,
    )?;
    Ok(())
}

/// Summary like "4 h 12 m read · 12% · ~31 h left at your pace".
fn book_summary(read_seconds: u32, percent: Option<u8>) -> String {
    let mut text = format!("{} read", read_time_label(read_seconds));
    match percent {
        Some(0) => text.push_str(" · 0%"),
        Some(percent) => {
            text.push_str(&format!(" · {percent}%"));
            let left = u64::from(read_seconds) * u64::from(100 - percent) / u64::from(percent);
            text.push_str(&format!(
                " · {} left at your pace",
                short_time_label(left as u32)
            ));
        }
        None => {}
    }
    text
}

/// "25 min" or "2 h 25".
fn minutes_label(minutes: u32) -> String {
    if minutes >= 60 {
        format!("{} h {:02}", minutes / 60, minutes % 60)
    } else {
        format!("{minutes} min")
    }
}

/// "37 m" or "4 h 12 m".
fn read_time_label(seconds: u32) -> String {
    let minutes = seconds / 60;
    if minutes >= 60 {
        format!("{} h {} m", minutes / 60, minutes % 60)
    } else {
        format!("{minutes} m")
    }
}

/// "~31 h" or "~45 m", rounded like the mockup's estimate.
fn short_time_label(seconds: u32) -> String {
    let minutes = (seconds + 30) / 60;
    if minutes >= 60 {
        format!("~{} h", (seconds + 1800) / 3600)
    } else {
        format!("~{minutes} m")
    }
}

/// "+18% vs last"; blank until last week has some reading.
fn change_label(this_week: u32, last_week: u32) -> String {
    if last_week == 0 {
        return String::new();
    }
    let change = (i64::from(this_week) - i64::from(last_week)) * 100 / i64::from(last_week);
    format!("{change:+}% vs last")
}

/// First day of `today`'s calendar year.
fn first_of_year(today: u32) -> u32 {
    let (year, _, _) = civil_date::civil_from_days(i64::from(today));
    u32::try_from(civil_date::days_from_civil(year, 1, 1)).unwrap_or(0)
}

/// Monday of `epoch_day`'s week, the mockup's first bar and heatmap column.
fn monday_of(epoch_day: u32) -> u32 {
    let since_monday = (u32::from(civil_date::weekday(i64::from(epoch_day))) + 6) % 7;
    epoch_day.saturating_sub(since_monday)
}

fn fit_line(
    display: &mut OrientedFrameBuffer<'_>,
    text: &str,
    style: UiTextStyle,
    bounds: TextBounds,
    baseline: i32,
) -> Result<(), Infallible> {
    let line = style.fit(text, bounds.width());
    // Left-aligned like the mockup's labels and rows.
    Text::new(&line, Point::new(bounds.left, baseline), style).draw_clipped(display, bounds)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        app::display::{UiFontFamily, UiFontSize},
        framebuffer::FrameBuffer,
        orientation::{DisplayOrientation, OrientedFrameBuffer},
        reading_stats::ReadingStats,
    };

    const TODAY: u32 = 20_729; // 2026-10-03

    fn sample_stats() -> ReadingStats {
        let mut stats = ReadingStats::default();
        for day in 0..30 {
            stats.record(
                TODAY - 29 + day,
                1200 + day * 37,
                12 + day % 5,
                Some("/a/Don.txt"),
            );
        }
        stats.record(TODAY - 4, 1500, 18, Some("/a/Don.txt"));
        stats.mark_finished("/a/Other.txt");
        stats
    }

    fn render(
        stats: &ReadingStats,
        current: Option<&CurrentBook<'_>>,
        size: UiFontSize,
    ) -> FrameBuffer {
        let mut frame = FrameBuffer::new_white();
        let preferences = DisplayPreferences {
            font_family: UiFontFamily::Inter,
            font_size: size,
        };
        {
            let mut display = OrientedFrameBuffer::new(&mut frame, DisplayOrientation::Portrait);
            render_reading_stats(&mut display, preferences, stats, TODAY, current).unwrap();
        }
        frame
    }

    #[test]
    fn renders_sample_and_empty_at_every_typography_size() {
        let stats = sample_stats();
        let current = CurrentBook {
            title: "Don Quijote de la Mancha",
            path: "/a/Don.txt",
            percent: Some(12),
        };
        for family in [UiFontFamily::Inter, UiFontFamily::AtkinsonHyperlegible] {
            for size in [UiFontSize::Compact, UiFontSize::Standard, UiFontSize::Large] {
                let mut frame = FrameBuffer::new_white();
                let preferences = DisplayPreferences {
                    font_family: family,
                    font_size: size,
                };
                {
                    let mut display =
                        OrientedFrameBuffer::new(&mut frame, DisplayOrientation::Portrait);
                    render_reading_stats(&mut display, preferences, &stats, TODAY, Some(&current))
                        .unwrap();
                    render_reading_stats(
                        &mut display,
                        preferences,
                        &ReadingStats::default(),
                        TODAY,
                        None,
                    )
                    .unwrap();
                }
            }
        }
    }

    #[test]
    fn sample_data_draws_more_ink_than_empty() {
        let sample = render(&sample_stats(), None, UiFontSize::Standard);
        let empty = render(&ReadingStats::default(), None, UiFontSize::Standard);
        let ink = |frame: &FrameBuffer| {
            frame
                .as_bytes()
                .iter()
                .map(|byte| u32::from(byte.count_zeros()))
                .sum::<u32>()
        };
        assert!(ink(&sample) > ink(&empty));
    }

    #[test]
    fn header_and_footer_pixels_are_drawn() {
        let frame = render(&ReadingStats::default(), None, UiFontSize::Standard);
        // Header band (logical y 20) maps to native (y, 479 - x).
        assert_eq!(frame.is_black(Point::new(20, 479 - 240)), Some(true));
        // Bottom bar rule at logical y 752.
        assert_eq!(frame.is_black(Point::new(752, 479 - 240)), Some(true));
    }

    #[test]
    fn long_titles_and_extreme_percentages_stay_inside_the_frame() {
        let stats = sample_stats();
        for percent in [Some(0), Some(1), Some(99), None] {
            let current = CurrentBook {
                title: "A very long book title that must be clamped to the summary box width",
                path: "/a/Don.txt",
                percent,
            };
            let frame = render(&stats, Some(&current), UiFontSize::Large);
            // Just inside the right border of the summary box stays paper.
            assert_eq!(frame.is_black(Point::new(600, 479 - 455)), Some(false));
            // The box border itself is ink at logical x 464.
            assert_eq!(frame.is_black(Point::new(600, 479 - 464)), Some(true));
        }
    }

    #[test]
    fn todays_bar_is_hatched_and_future_days_stay_blank() {
        let stats = sample_stats();
        let frame = render(&stats, None, UiFontSize::Standard);
        // Today is the last elapsed bar: hatching mixes ink and paper inside.
        let today_x = 479 - 360;
        let mut ink = 0;
        for y in 260..350 {
            ink += u32::from(frame.is_black(Point::new(y, today_x)) == Some(true));
        }
        assert!(ink > 5 && ink < 85);
        // Tomorrow's bar area (Sunday, logical x 420) is completely blank.
        for y in 240..355 {
            assert_eq!(frame.is_black(Point::new(y, 479 - 420)), Some(false));
        }
    }

    #[test]
    fn summary_follows_the_mockup_pace_line() {
        assert_eq!(
            book_summary(4 * 3600 + 12 * 60, Some(12)),
            "4 h 12 m read · 12% · ~31 h left at your pace"
        );
        assert_eq!(book_summary(37 * 60, None), "37 m read");
        assert_eq!(book_summary(60, Some(0)), "1 m read · 0%");
    }

    #[test]
    fn grouped_values_and_change_labels_read_like_the_mockup() {
        assert_eq!(grouped(2318), "2,318");
        assert_eq!(grouped(7), "7");
        assert_eq!(grouped(1_234_567), "1,234,567");
        assert_eq!(change_label(118, 100), "+18% vs last");
        assert_eq!(change_label(25, 0), "");
    }

    #[test]
    fn weeks_start_on_monday_and_years_on_january_first() {
        // 2026-10-03 is a Saturday; its Monday is 2026-09-28.
        assert_eq!(monday_of(TODAY), TODAY - 5);
        assert_eq!(monday_of(TODAY - 5), TODAY - 5);
        assert_eq!(first_of_year(TODAY), 20_454);
        // An unset clock near the epoch saturates instead of underflowing.
        assert_eq!(monday_of(0), 0);
    }
}
