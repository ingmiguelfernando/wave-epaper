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
    text_block(
        display,
        clock.date,
        preferences.large_style(),
        TextBounds::new(36, 356, 444, 450),
        2,
        true,
    )?;
    Line::new(Point::new(70, 470), Point::new(410, 470))
        .into_styled(PrimitiveStyle::with_stroke(BinaryColor::On, 3))
        .draw(display)?;
    if let Some(weather) = &clock.weather {
        weather_icon(weather.weather_code).draw_scaled(
            display,
            Point::new(40, 514),
            2,
            BinaryColor::On,
        )?;
        text_block(
            display,
            weather.summary,
            preferences.large_style(),
            TextBounds::new(106, 510, 440, 610),
            2,
            false,
        )?;
        text_block(
            display,
            weather.details,
            preferences.body_style(),
            TextBounds::new(36, 622, 444, 708),
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
    text_block(
        display,
        weather.place,
        preferences.detail_style(),
        TextBounds::new(46, 48, 434, 80),
        1,
        false,
    )?;
    text_block(
        display,
        weather.updated,
        preferences.body_style(),
        TextBounds::new(46, 88, 434, 158),
        2,
        false,
    )?;
    weather_icon(weather.weather_code).draw_scaled(
        display,
        Point::new(46, 185),
        5,
        BinaryColor::On,
    )?;
    let height = temperature_height(weather.temperature);
    draw_big_text(
        display,
        weather.temperature,
        318,
        195 + (110 - height) / 2,
        height,
    )?;
    text_block(
        display,
        weather.condition,
        preferences.heading_style(),
        TextBounds::new(192, 322, 444, 410),
        2,
        true,
    )?;
    text_block(
        display,
        weather.details,
        preferences.body_style(),
        TextBounds::new(36, 426, 444, 530),
        3,
        true,
    )?;
    for (index, day) in weather.days.iter().take(3).enumerate() {
        let left = 36 + index as i32 * 136;
        text_block(
            display,
            day.name,
            preferences.heading_style(),
            TextBounds::new(left + 4, 552, left + 132, 588),
            1,
            true,
        )?;
        weather_icon(day.weather_code).draw_scaled(
            display,
            Point::new(left + 42, 600),
            2,
            BinaryColor::On,
        )?;
        text_block(
            display,
            day.range,
            preferences.body_style(),
            TextBounds::new(left + 4, 666, left + 132, 700),
            1,
            true,
        )?;
        text_block(
            display,
            day.rain,
            preferences.detail_style(),
            TextBounds::new(left + 4, 708, left + 132, 738),
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
        assert!(black(&frame, 240, 470));
        assert!(!black(&frame, 56, 516));
    }

    #[test]
    fn clock_weather_pixels_cover_digit_and_scaled_icon() {
        let mut frame = FrameBuffer::new_white();
        render_sleep_clock(&mut frame, DisplayPreferences::default(), &clock(true)).unwrap();
        check_frame(&frame);
        assert!(black(&frame, 169, 189));
        assert!(black(&frame, 56, 516));
        assert!(black(&frame, 57, 517));
        assert!(!black(&frame, 40, 514));
    }

    #[test]
    fn weather_pixels_cover_frame_temperature_and_three_forecast_icons() {
        let mut frame = FrameBuffer::from_native_bytes(vec![0; 48_000]).unwrap();
        render_sleep_weather(&mut frame, DisplayPreferences::default(), &weather()).unwrap();
        check_frame(&frame);
        assert!(black(&frame, 333, 201));
        assert!(!black(&frame, 333, 230));
        assert!(black(&frame, 86, 190));
        assert!(black(&frame, 102, 604));
        assert!(black(&frame, 242, 602));
        assert!(black(&frame, 378, 602));
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
                    ("Friday, October 2", preferences.large_style(), 408, 2, 94),
                    (
                        "18° · Partly cloudy",
                        preferences.large_style(),
                        334,
                        2,
                        100,
                    ),
                    (
                        "H 21° · L 11° · Rain 10%",
                        preferences.body_style(),
                        408,
                        2,
                        86,
                    ),
                    (
                        "Fri, Oct 2 · updated 13:30",
                        preferences.body_style(),
                        388,
                        2,
                        70,
                    ),
                    ("Partly cloudy", preferences.heading_style(), 252, 2, 88),
                    (
                        "H 21° · L 11° · Wind 12 km/h · Rain 10%",
                        preferences.body_style(),
                        408,
                        3,
                        104,
                    ),
                    ("23° / 12°", preferences.body_style(), 128, 1, 34),
                    ("Sat", preferences.heading_style(), 128, 1, 36),
                    ("100%", preferences.detail_style(), 128, 1, 30),
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
                let height = temperature_height(sample.temperature);
                assert!(height < 110 && height > 0);
                assert!(big_text_width(sample.temperature, height) <= 240);
                let mut frame = FrameBuffer::new_white();
                render_sleep_weather(&mut frame, preferences, &sample).unwrap();
                for y in 19..782 {
                    for x in 19..462 {
                        if !black(&frame, x, y) {
                            continue;
                        }
                        assert!(
                            [
                                TextBounds::new(46, 48, 434, 80),
                                TextBounds::new(46, 88, 434, 158),
                                TextBounds::new(46, 185, 176, 315),
                                TextBounds::new(198, 195, 438, 305),
                                TextBounds::new(192, 322, 444, 410),
                                TextBounds::new(36, 426, 444, 530),
                                TextBounds::new(36, 552, 444, 738),
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
        assert!(!black(&four, 110, 604));
        assert_eq!(
            footer_widths(DisplayPreferences::default().body_style(), None),
            (String::new(), 408)
        );
        assert_eq!(temperature_height("18°"), 110);
        assert!(temperature_height("-123°") < 110);
    }
}
