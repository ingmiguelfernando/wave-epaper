//! Reusable application modules for the Waveshare ESP32-S3 e-Paper 3.97 board.
//!
//! Hardware-independent code stays in this library so framebuffer, routing,
//! widgets and protocol helpers can be unit-tested on the host. ESP-IDF wiring
//! remains isolated in `main.rs`.

pub mod ai_client;
pub mod alarm;
pub mod app;
pub mod audio;
pub mod battery_log;
pub mod bible;
pub mod bible_nav;
pub mod board_services;
pub mod build_info;
pub mod buttons;
pub mod calendar;
pub mod charset;
pub mod civil_date;
pub mod dictionary;
pub mod dither;
pub mod environment;
pub mod epaper;
pub mod epub;
pub mod framebuffer;
pub mod games;
pub mod hyphenation;
pub mod imu;
pub mod imu_events;
pub mod json_lite;
pub mod keyboard_navigation;
pub mod lua_runtime;
pub mod network;
pub mod network_config;
pub mod ntp;
pub mod orientation;
pub mod panel_refresh;
pub mod photos;
pub mod power;
pub mod power_key;
pub mod power_key_menu;
pub mod power_settings;
pub mod radio_burst;
pub mod reader;
pub mod reading_stats;
pub mod regional;
pub mod rtc;
pub mod rtc_alarm_interrupt;
pub mod runtime_memory;
pub mod runtime_worker;
pub mod sd_file;
pub mod shared_i2c;
pub mod sleep_images;
pub mod sleep_mode;
pub mod sleep_screen;
pub mod storage;
pub mod unit_converter;
pub mod voice_note_metadata;
pub mod voice_notes;
pub mod watchdog;
pub mod weather;
pub mod weather_config;
pub mod wifi_transfer;
