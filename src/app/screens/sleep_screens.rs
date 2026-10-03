//! Drawing-only clock and weather cards for future sleep-mode integration.

use core::convert::Infallible;

use embedded_graphics::{
    pixelcolor::BinaryColor,
    prelude::{Drawable, Point, Primitive, Size},
    primitives::{Line, PrimitiveStyle, Rectangle},
};

use crate::{
    app::{
        display::DisplayPreferences,
        typography::{Text, TextBounds, UiTextStyle},
        widgets::{
            big_digits::{big_text_width, draw_big_text},
            icons::weather_icon,
        },
    },
    orientation::OrientedFrameBuffer,
};

pub struct SleepClock<'a> {
    pub time: &'a str,
    pub date: &'a str,
    pub weather: Option<SleepWeatherLine<'a>>,
    pub battery_percent: Option<u8>,
    pub wake_hint: &'a str,
}

pub struct SleepWeatherLine<'a> {
    pub weather_code: u16,
    pub summary: &'a str,
    pub details: &'a str,
}

pub struct SleepWeather<'a> {
    pub place: &'a str,
    pub updated: &'a str,
    pub weather_code: u16,
    pub temperature: &'a str,
    pub condition: &'a str,
    pub details: &'a str,
    pub days: &'a [SleepWeatherDay<'a>],
    pub battery_percent: Option<u8>,
    pub wake_hint: &'a str,
}

pub struct SleepWeatherDay<'a> {
    pub name: &'a str,
    pub weather_code: u16,
    pub range: &'a str,
    pub rain: &'a str,
}

pub fn render_sleep_clock(
    display: &mut OrientedFrameBuffer<'_>,
    preferences: DisplayPreferences,
    clock: &SleepClock<'_>,
) -> Result<(), Infallible> {
    draw_frame(display)?;
    draw_big_text(display, clock.time, 240, 180, 150)?;
    let date_height = text_height(clock.date, preferences.large_style(), 408, 2);
    text_block(
        display,
        clock.date,
        preferences.large_style(),
        TextBounds::new(36, 340, 444, 340 + date_height),
        2,
        true,
    )?;
    let rule_y = 340 + date_height + 40;
    Line::new(Point::new(70, rule_y), Point::new(410, rule_y))
        .into_styled(PrimitiveStyle::with_stroke(BinaryColor::On, 3))
        .draw(display)?;
    if let Some(weather) = &clock.weather {
        let (icon, summary) = centered_group(
            weather.summary,
            preferences.large_style(),
            52,
            12,
            rule_y + 28,
        );
        weather_icon(weather.weather_code).draw_scaled(
            display,
            Point::new(icon.left, icon.top),
            2,
            BinaryColor::On,
        )?;
        text_block(
            display,
            weather.summary,
            preferences.large_style(),
            summary,
            2,
            true,
        )?;
        let details_top = icon.bottom.max(summary.bottom) + 6;
        text_block(
            display,
            weather.details,
            preferences.body_style(),
            TextBounds::new(36, details_top, 444, details_top + 64),
            2,
            true,
        )?;
    }
    draw_footer(display, preferences, clock.wake_hint, clock.battery_percent)
}

pub fn render_sleep_weather(
    display: &mut OrientedFrameBuffer<'_>,
    preferences: DisplayPreferences,
    weather: &SleepWeather<'_>,
) -> Result<(), Infallible> {
    draw_frame(display)?;
    let layout = weather_layout(preferences, weather);
    text_block(
        display,
        weather.place,
        preferences.detail_style(),
        TextBounds::new(46, 48, 434, 70),
        1,
        false,
    )?;
    text_block(
        display,
        weather.updated,
        preferences.body_style(),
        TextBounds::new(46, 70, 434, layout.hero_top - 22),
        2,
        false,
    )?;
    weather_icon(weather.weather_code).draw_scaled(
        display,
        Point::new(layout.icon.left, layout.icon.top),
        5,
        BinaryColor::On,
    )?;
    let height = temperature_height(weather.temperature);
    draw_big_text(
        display,
        weather.temperature,
        layout.condition.left + layout.condition.width() / 2,
        layout.temperature_top,
        height,
    )?;
    text_block(
        display,
        weather.condition,
        preferences.heading_style(),
        layout.condition,
        2,
        true,
    )?;
    text_block(
        display,
        weather.details,
        preferences.body_style(),
        layout.details,
        3,
        true,
    )?;
    Line::new(
        Point::new(42, layout.rule_y),
        Point::new(438, layout.rule_y),
    )
    .into_styled(PrimitiveStyle::with_stroke(BinaryColor::On, 3))
    .draw(display)?;
    for (index, day) in weather.days.iter().take(3).enumerate() {
        let left = 36 + index as i32 * 136;
        text_block(
            display,
            day.name,
            preferences.heading_style(),
            TextBounds::new(left + 4, layout.rule_y + 20, left + 132, layout.icon_y - 8),
            1,
            true,
        )?;
        weather_icon(day.weather_code).draw_scaled(
            display,
            Point::new(left + 42, layout.icon_y),
            2,
            BinaryColor::On,
        )?;
        text_block(
            display,
            day.range,
            preferences.body_style(),
            TextBounds::new(left + 4, layout.range_y, left + 132, layout.rain_y - 6),
            1,
            true,
        )?;
        text_block(
            display,
            day.rain,
            preferences.body_style(),
            TextBounds::new(left + 4, layout.rain_y, left + 132, layout.rain_y + 34),
            1,
            true,
        )?;
    }
    draw_footer(
        display,
        preferences,
        weather.wake_hint,
        weather.battery_percent,
    )
}

fn text_height(text: &str, style: UiTextStyle, width: i32, limit: usize) -> i32 {
    fitted_lines(text, style, width, limit).len() as i32 * i32::from(style.line_height())
}

fn text_width(text: &str, style: UiTextStyle, width: i32) -> i32 {
    fitted_lines(text, style, width, 2)
        .iter()
        .map(|line| style.text_width(line))
        .max()
        .unwrap_or(0)
}

fn centered_group(
    text: &str,
    style: UiTextStyle,
    icon_size: i32,
    gap: i32,
    top: i32,
) -> (TextBounds, TextBounds) {
    let width = text_width(text, style, 408 - icon_size - gap);
    let height = text_height(text, style, width, 2);
    let group_height = icon_size.max(height);
    let left = 240 - (icon_size + gap + width) / 2;
    let icon_top = top + (group_height - icon_size) / 2;
    let text_top = top + (group_height - height) / 2;
    (
        TextBounds::new(left, icon_top, left + icon_size, icon_top + icon_size),
        TextBounds::new(
            left + icon_size + gap,
            text_top,
            left + icon_size + gap + width,
            text_top + height,
        ),
    )
}

struct WeatherLayout {
    hero_top: i32,
    icon: TextBounds,
    temperature_top: i32,
    condition: TextBounds,
    details: TextBounds,
    rule_y: i32,
    icon_y: i32,
    range_y: i32,
    rain_y: i32,
}

fn weather_layout(preferences: DisplayPreferences, weather: &SleepWeather<'_>) -> WeatherLayout {
    let hero_top = 70 + text_height(weather.updated, preferences.body_style(), 388, 2) + 22;
    let temperature_height = temperature_height(weather.temperature);
    let width = big_text_width(weather.temperature, temperature_height).max(text_width(
        weather.condition,
        preferences.heading_style(),
        240,
    ));
    let condition_height = text_height(weather.condition, preferences.heading_style(), width, 2);
    let column_height = temperature_height + 8 + condition_height;
    let hero_height = 130.max(column_height);
    let left = 240 - (130 + 8 + width) / 2;
    let icon_top = hero_top + (hero_height - 130) / 2;
    let temperature_top = hero_top + (hero_height - column_height) / 2;
    let condition_top = temperature_top + temperature_height + 8;
    let details_top = hero_top + hero_height + 12;
    let details_bottom =
        details_top + text_height(weather.details, preferences.body_style(), 408, 3);
    let rule_y = details_bottom + 34;
    let icon_y = rule_y + 20 + i32::from(preferences.heading_style().line_height()) + 8;
    let range_y = icon_y + 52 + 12;
    let rain_y = range_y + i32::from(preferences.body_style().line_height()) + 6;
    WeatherLayout {
        hero_top,
        icon: TextBounds::new(left, icon_top, left + 130, icon_top + 130),
        temperature_top,
        condition: TextBounds::new(
            left + 138,
            condition_top,
            left + 138 + width,
            condition_top + condition_height,
        ),
        details: TextBounds::new(36, details_top, 444, details_bottom),
        rule_y,
        icon_y,
        range_y,
        rain_y,
    }
}

fn temperature_height(text: &str) -> i32 {
    let mut height = 110;
    while height > 0 && big_text_width(text, height) > 240 {
        height -= 1;
    }
    height
}

fn draw_frame(display: &mut OrientedFrameBuffer<'_>) -> Result<(), Infallible> {
    Rectangle::new(Point::new(16, 16), Size::new(448, 768))
        .into_styled(PrimitiveStyle::with_stroke(BinaryColor::On, 3))
        .draw(display)?;
    Ok(())
}

fn fitted_lines(text: &str, style: UiTextStyle, width: i32, limit: usize) -> Vec<String> {
    let wrapped = style.wrap(text, width);
    let mut lines = Vec::new();
    for (index, line) in wrapped.iter().take(limit).enumerate() {
        let line = if index + 1 == limit && wrapped.len() > limit {
            format!("{line}...")
        } else {
            line.clone()
        };
        lines.push(style.fit(&line, width));
    }
    lines
}

fn text_block(
    display: &mut OrientedFrameBuffer<'_>,
    text: &str,
    style: UiTextStyle,
    bounds: TextBounds,
    limit: usize,
    centered: bool,
) -> Result<(), Infallible> {
    let line_height = i32::from(style.line_height());
    let limit = limit.min(((bounds.bottom - bounds.top) / line_height) as usize);
    for (index, line) in fitted_lines(text, style, bounds.width(), limit)
        .iter()
        .enumerate()
    {
        let left = if centered {
            bounds.left + (bounds.width() - style.text_width(line)) / 2
        } else {
            bounds.left
        };
        let baseline = bounds.top + style.cap_height() + index as i32 * line_height;
        Text::new(line, Point::new(left, baseline), style).draw_clipped(display, bounds)?;
    }
    Ok(())
}

fn footer_widths(style: UiTextStyle, percent: Option<u8>) -> (String, i32) {
    let battery = percent.map_or_else(String::new, |percent| format!("{percent}%"));
    let width = style.text_width(&battery);
    let hint_width = 408 - if battery.is_empty() { 0 } else { width + 16 };
    (battery, hint_width)
}

fn draw_footer(
    display: &mut OrientedFrameBuffer<'_>,
    preferences: DisplayPreferences,
    hint: &str,
    percent: Option<u8>,
) -> Result<(), Infallible> {
    let style = preferences.body_style();
    let (battery, hint_width) = footer_widths(style, percent);
    let hint = style.fit(hint, hint_width);
    let bounds = TextBounds::new(36, 746, 444, 774);
    Text::new(&hint, Point::new(36, 768), style).draw_clipped(display, bounds)?;
    Text::new(
        &battery,
        Point::new(444 - style.text_width(&battery), 768),
        style,
    )
    .draw_clipped(display, bounds)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        app::{
            display::{UiFontFamily, UiFontSize},
            render_sleep_clock, render_sleep_weather,
        },
        framebuffer::FrameBuffer,
    };

    fn clock(with_weather: bool) -> SleepClock<'static> {
        SleepClock {
            time: "13:42",
            date: "Friday, October 2",
            weather: with_weather.then_some(SleepWeatherLine {
                weather_code: 2,
                summary: "18° · Partly cloudy",
                details: "H 21° · L 11° · Rain 10%",
            }),
            battery_percent: Some(100),
            wake_hint: "Press any key to wake",
        }
    }

    const DAYS: [SleepWeatherDay<'static>; 4] = [
        SleepWeatherDay {
            name: "Sat",
            weather_code: 0,
            range: "23° / 12°",
            rain: "0%",
        },
        SleepWeatherDay {
            name: "Sun",
            weather_code: 61,
            range: "17° / 10°",
            rain: "80%",
        },
        SleepWeatherDay {
            name: "Mon",
            weather_code: 95,
            range: "15° / 9°",
            rain: "60%",
        },
        SleepWeatherDay {
            name: "IGNORED",
            weather_code: 71,
            range: "-40° / -50°",
            rain: "100%",
        },
    ];

    fn weather() -> SleepWeather<'static> {
        SleepWeather {
            place: "Madrid",
            updated: "Fri, Oct 2 · updated 13:30",
            weather_code: 2,
            temperature: "18°",
            condition: "Partly cloudy",
            details: "H 21° · L 11° · Wind 12 km/h · Rain 10%",
            days: &DAYS[..3],
            battery_percent: Some(100),
            wake_hint: "Press any key to wake",
        }
    }

    fn black(frame: &FrameBuffer, x: i32, y: i32) -> bool {
        frame.is_black(Point::new(y, 479 - x)).unwrap()
    }

    fn check_frame(frame: &FrameBuffer) {
        for x in [15, 16, 17, 462, 463, 464] {
            assert!(black(frame, x, 400), "frame column {x}");
        }
        for y in [15, 16, 17, 782, 783, 784] {
            assert!(black(frame, 240, y), "frame row {y}");
        }
        assert!(!black(frame, 14, 400));
        assert!(!black(frame, 465, 400));
        assert!(!black(frame, 240, 14));
        assert!(!black(frame, 240, 785));
        assert!(!black(frame, 0, 0));
    }

    #[test]
    fn clock_pixels_cover_frame_digit_and_empty_weather_area() {
        let mut frame = FrameBuffer::from_native_bytes(vec![0; 48_000]).unwrap();
        render_sleep_clock(&mut frame, DisplayPreferences::default(), &clock(false)).unwrap();
        check_frame(&frame);
        assert!(black(&frame, 169, 189));
        assert!(!black(&frame, 169, 222));
        let rule_y = 380
            + text_height(
                clock(false).date,
                DisplayPreferences::default().large_style(),
                408,
                2,
            );
        assert!(black(&frame, 240, rule_y));
        assert!(!black(&frame, 240, rule_y + 28));
    }

    #[test]
    fn clock_weather_pixels_cover_digit_and_scaled_icon() {
        let mut frame = FrameBuffer::new_white();
        render_sleep_clock(&mut frame, DisplayPreferences::default(), &clock(true)).unwrap();
        check_frame(&frame);
        assert!(black(&frame, 169, 189));
        let preferences = DisplayPreferences::default();
        let rule_y = 380 + text_height(clock(true).date, preferences.large_style(), 408, 2);
        let (icon, _) = centered_group(
            "18° · Partly cloudy",
            preferences.large_style(),
            52,
            12,
            rule_y + 28,
        );
        assert!(black(&frame, icon.left + 16, icon.top + 2));
        assert!(black(&frame, icon.left + 17, icon.top + 3));
        assert!(!black(&frame, icon.left, icon.top));
    }

    #[test]
    fn weather_pixels_cover_frame_temperature_and_three_forecast_icons() {
        let mut frame = FrameBuffer::from_native_bytes(vec![0; 48_000]).unwrap();
        render_sleep_weather(&mut frame, DisplayPreferences::default(), &weather()).unwrap();
        check_frame(&frame);
        let layout = weather_layout(DisplayPreferences::default(), &weather());
        let digit_left =
            layout.condition.left + layout.condition.width() / 2 - big_text_width("18°", 110) / 2;
        assert!(black(&frame, digit_left + 48, layout.temperature_top + 14));
        assert!(!black(&frame, digit_left + 5, layout.temperature_top + 35));
        assert!(black(&frame, layout.icon.left + 40, layout.icon.top + 5));
        assert!(black(&frame, 102, layout.icon_y + 4));
        assert!(black(&frame, 242, layout.icon_y + 2));
        assert!(black(&frame, 378, layout.icon_y + 2));
        for y in layout.rule_y - 1..=layout.rule_y + 1 {
            assert!(black(&frame, 42, y));
            assert!(black(&frame, 438, y));
        }
    }

    #[test]
    fn sample_text_fits_every_size_and_family_without_truncation() {
        for font_family in UiFontFamily::ALL {
            for font_size in UiFontSize::ALL {
                let preferences = DisplayPreferences {
                    font_family,
                    font_size,
                };
                for (text, style, width, lines, height) in [
                    ("Friday, October 2", preferences.large_style(), 408, 2, 88),
                    (
                        "18° · Partly cloudy",
                        preferences.large_style(),
                        344,
                        2,
                        100,
                    ),
                    (
                        "H 21° · L 11° · Rain 10%",
                        preferences.body_style(),
                        408,
                        2,
                        64,
                    ),
                    (
                        "Fri, Oct 2 · updated 13:30",
                        preferences.body_style(),
                        388,
                        2,
                        70,
                    ),
                    ("Partly cloudy", preferences.heading_style(), 240, 2, 58),
                    (
                        "H 21° · L 11° · Wind 12 km/h · Rain 10%",
                        preferences.body_style(),
                        408,
                        3,
                        69,
                    ),
                    ("23° / 12°", preferences.body_style(), 128, 1, 34),
                    ("Sat", preferences.heading_style(), 128, 1, 36),
                    ("100%", preferences.body_style(), 128, 1, 34),
                ] {
                    let wrapped = style.wrap(text, width);
                    assert!(wrapped.len() <= lines);
                    assert!(wrapped.len() as i32 * i32::from(style.line_height()) <= height);
                    assert!(wrapped.iter().all(|line| style.text_width(line) <= width));
                    assert_eq!(fitted_lines(text, style, width, lines), wrapped);
                }
                let (battery, width) = footer_widths(preferences.body_style(), Some(100));
                assert!(preferences.body_style().text_width("Press any key to wake") <= width);
                assert_eq!(
                    width + preferences.body_style().text_width(&battery) + 16,
                    408
                );
                for with_weather in [false, true] {
                    let mut frame = FrameBuffer::new_white();
                    render_sleep_clock(&mut frame, preferences, &clock(with_weather)).unwrap();
                    check_frame(&frame);
                }
                let mut frame = FrameBuffer::new_white();
                render_sleep_weather(&mut frame, preferences, &weather()).unwrap();
                check_frame(&frame);
            }
        }
    }

    #[test]
    fn extreme_strings_stay_in_their_regions_for_every_typography() {
        let long = "A very long weather description with extreme conditions and an exceptionallylongunbrokenlocationname";
        for font_family in UiFontFamily::ALL {
            for font_size in UiFontSize::ALL {
                let preferences = DisplayPreferences {
                    font_family,
                    font_size,
                };
                let mut sample = weather();
                sample.place = long;
                sample.updated = long;
                sample.temperature = "-12345°";
                sample.condition = long;
                sample.details = long;
                sample.wake_hint = long;
                let days = [SleepWeatherDay {
                    name: long,
                    weather_code: 95,
                    range: "-12345° / -12345°",
                    rain: "Rain 100% with an exceptionallylongunbrokenlabel",
                }];
                sample.days = &days;
                let layout = weather_layout(preferences, &sample);
                let height = temperature_height(sample.temperature);
                assert!(height < 110 && height > 0);
                assert!(big_text_width(sample.temperature, height) <= 240);
                let mut frame = FrameBuffer::new_white();
                render_sleep_weather(&mut frame, preferences, &sample).unwrap();
                let temperature_left = layout.condition.left + layout.condition.width() / 2
                    - big_text_width(sample.temperature, height) / 2;
                assert!(
                    black(
                        &frame,
                        temperature_left + height / 4,
                        layout.temperature_top + (height - height / 8) / 2 + height / 16,
                    ),
                    "negative temperature must retain its minus sign"
                );
                for y in 19..782 {
                    for x in 19..462 {
                        if !black(&frame, x, y) {
                            continue;
                        }
                        assert!(
                            [
                                TextBounds::new(46, 48, 434, 70),
                                TextBounds::new(46, 70, 434, layout.hero_top - 22),
                                layout.icon,
                                TextBounds::new(
                                    layout.condition.left,
                                    layout.temperature_top,
                                    layout.condition.right,
                                    layout.temperature_top + height
                                ),
                                layout.condition,
                                layout.details,
                                TextBounds::new(42, layout.rule_y - 1, 439, layout.rule_y + 2),
                                TextBounds::new(36, layout.rule_y + 20, 444, layout.rain_y + 34),
                                TextBounds::new(36, 746, 444, 774),
                            ]
                            .iter()
                            .any(|b| x >= b.left && x < b.right && y >= b.top && y < b.bottom),
                            "escaped pixel {x},{y}"
                        );
                    }
                }
                let (battery, width) = footer_widths(preferences.body_style(), Some(100));
                assert!(
                    preferences
                        .body_style()
                        .text_width(&preferences.body_style().fit(long, width))
                        <= width
                );
                assert_eq!(battery, "100%");
            }
        }
    }

    #[test]
    fn groups_are_centered_and_rows_fit_every_typography() {
        for font_family in UiFontFamily::ALL {
            for font_size in UiFontSize::ALL {
                let preferences = DisplayPreferences {
                    font_family,
                    font_size,
                };
                for summary in [
                    "18° · Partly cloudy",
                    "-123° · Freezing rain",
                    "",
                    "An exceptionallylongunbrokencondition followed by more words than fit",
                ] {
                    let rule_y =
                        380 + text_height("Friday, October 2", preferences.large_style(), 408, 2);
                    let (icon, text) =
                        centered_group(summary, preferences.large_style(), 52, 12, rule_y + 28);
                    assert!((icon.left + text.right - 480).abs() <= 1);
                    assert!((icon.top + icon.bottom - text.top - text.bottom).abs() <= 1);
                    assert_eq!(text.left - icon.right, 12);
                    assert!(icon.left >= 36 && text.right <= 444);
                    assert_eq!(icon.top.min(text.top), rule_y + 28);
                    assert!(icon.bottom.max(text.bottom) + 6 + 64 < 746);
                }
                for temperature in ["18°", "-40°", "-12345°", "", "-12345678901234567890°"] {
                    for condition in [
                        "Partly cloudy",
                        "Freezing rain and exceptionallylongunbrokenconditions",
                        "",
                    ] {
                        let mut sample = weather();
                        sample.temperature = temperature;
                        sample.condition = condition;
                        sample.updated =
                            "Updated at an unusually long time with a very long location label";
                        sample.details = "H -40° · L -50° · Wind 999 km/h · Rain 100% with unusually long extra details";
                        let layout = weather_layout(preferences, &sample);
                        let height = temperature_height(temperature);
                        assert!((layout.icon.left + layout.condition.right - 480).abs() <= 1);
                        assert!(
                            (layout.icon.top + layout.icon.bottom
                                - layout.temperature_top
                                - layout.condition.bottom)
                                .abs()
                                <= 1
                        );
                        assert!(layout.icon.left >= 36 && layout.condition.right <= 444);
                        assert!(big_text_width(temperature, height) <= layout.condition.width());
                        assert_eq!(layout.condition.top, layout.temperature_top + height + 8);
                        assert!(layout.details.top >= layout.icon.bottom + 12);
                        assert_eq!(layout.rule_y - layout.details.bottom, 34);
                        assert!(layout.rain_y + 34 < 746);
                        assert!(i32::from(preferences.body_style().line_height()) <= 34);
                    }
                }
            }
        }
    }

    #[test]
    fn clock_extremes_stay_below_time_and_above_footer() {
        let long = "An exceptionallylongunbrokenlocationname and a very long weather description with more words";
        for font_family in UiFontFamily::ALL {
            for font_size in UiFontSize::ALL {
                let preferences = DisplayPreferences {
                    font_family,
                    font_size,
                };
                let mut sample = clock(true);
                sample.date = long;
                sample.weather = Some(SleepWeatherLine {
                    weather_code: 95,
                    summary: long,
                    details: long,
                });
                let mut frame = FrameBuffer::new_white();
                render_sleep_clock(&mut frame, preferences, &sample).unwrap();
                let date_bottom = 340 + text_height(long, preferences.large_style(), 408, 2);
                let rule_y = date_bottom + 40;
                let (icon, summary) =
                    centered_group(long, preferences.large_style(), 52, 12, rule_y + 28);
                let details_top = icon.bottom.max(summary.bottom) + 6;
                for y in 331..746 {
                    for x in 19..462 {
                        if black(&frame, x, y) {
                            assert!(
                                [
                                    TextBounds::new(36, 340, 444, date_bottom),
                                    TextBounds::new(70, rule_y - 1, 411, rule_y + 2),
                                    icon,
                                    summary,
                                    TextBounds::new(36, details_top, 444, details_top + 64),
                                ]
                                .iter()
                                .any(|b| x >= b.left && x < b.right && y >= b.top && y < b.bottom),
                                "escaped clock pixel {x},{y}"
                            );
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn forecast_ignores_extra_days_and_accepts_no_days_or_battery() {
        let mut three = FrameBuffer::new_white();
        let mut four = FrameBuffer::new_white();
        let mut sample = weather();
        render_sleep_weather(&mut three, DisplayPreferences::default(), &sample).unwrap();
        sample.days = &DAYS;
        render_sleep_weather(&mut four, DisplayPreferences::default(), &sample).unwrap();
        assert_eq!(three, four);
        sample.days = &[];
        sample.battery_percent = None;
        render_sleep_weather(&mut four, DisplayPreferences::default(), &sample).unwrap();
        assert!(!black(
            &four,
            102,
            weather_layout(DisplayPreferences::default(), &sample).icon_y + 4
        ));
        assert_eq!(
            footer_widths(DisplayPreferences::default().body_style(), None),
            (String::new(), 408)
        );
        assert_eq!(temperature_height("18°"), 110);
        assert!(temperature_height("-123°") < 110);
    }
}
