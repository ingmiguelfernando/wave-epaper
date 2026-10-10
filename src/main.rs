#[cfg(target_os = "espidf")]
mod firmware {
    use std::{
        ffi::CString,
        time::{Duration, Instant},
    };

    use anyhow::Result;
    use embedded_hal::delay::DelayNs;
    use esp_idf_svc::{
        fs::fatfs::Fatfs,
        hal::{
            delay::FreeRtos,
            gpio::{AnyIOPin, Input, PinDriver, Pull},
            i2c::{I2cConfig, I2cDriver},
            i2s::{
                config::{
                    ClockSource, Config as I2sChannelConfig, DataBitWidth, MclkMultiple, SlotMode,
                    StdClkConfig, StdConfig, StdGpioConfig, StdSlotConfig,
                },
                I2sBiDir, I2sDriver,
            },
            peripherals::Peripherals,
            sd::{
                mmc::{SdMmcHostConfiguration, SdMmcHostDriver},
                SdCardConfiguration, SdCardDriver,
            },
            spi::{config::Config as SpiConfig, Dma, SpiBusDriver, SpiDriver, SpiDriverConfig},
            units::*,
        },
        io::vfs::MountedFatfs,
        log::EspLogger,
        sys,
    };
    use log::{info, warn};
    use waveshare_epd397_rust_app::{
        ai_config::{AiConfig, AI_CONFIG_PATH},
        alarm::{AlarmEngine, AlarmSnapshot, AlarmUiOutcome, ALARMS_CONFIG_PATH},
        app::{
            display::{DisplayPreferences, DISPLAY_CONFIG_PATH},
            render_current_screen, render_sleep_card, render_sleep_mode, AppState, ScreenRoute,
            SleepCard, SleepLayout, ALARM_POLL_SECONDS, IMU_EVENT_SCREEN_REFRESH_SECONDS,
            MOTION_LIVE_REFRESH_SECONDS, NETWORK_LIVE_REFRESH_SECONDS,
            NETWORK_LOG_HEARTBEAT_SECONDS, PANEL_IDLE_SLEEP_SECONDS, SAMPLE_LIVE_REFRESH_SECONDS,
            VOICE_RECORD_SCREEN_REFRESH_SECONDS,
        },
        audio::{
            espidf::AudioRuntime, AudioSnapshot, AudioUiRequest, AUDIO_MCLK_HZ,
            AUDIO_SAMPLE_RATE_HZ, DEFAULT_AUDIO_VOLUME_PERCENT,
        },
        battery_log::{BatteryLog, BATTERY_LOG_PATH, SAMPLE_MINUTES},
        bible::BIBLE_ROOT,
        bible_state::BibleUiState,
        board_services::{reset_reason_label, BoardServices, BoardSnapshot},
        build_info::{FIRMWARE_VERSION, PRODUCT_SLUG, UI_SHELL_MILESTONE},
        buttons::{
            BootButtonEvent, ButtonEvent, Buttons, LongPressBackButton, BOOT_BACK_LONG_PRESS_MS,
        },
        calendar::{
            create_personal_event, delete_personal_event, update_personal_event, CalendarUiRequest,
            CALENDAR_ROOT,
        },
        epaper::Epaper397,
        framebuffer::FrameBuffer,
        games::records::{GameRecords, RECORDS_PATH},
        games::sudoku_save::{SudokuSave, SUDOKU_SAVE_PATH},
        imu_events::IMU_EVENT_SAMPLE_INTERVAL_MS,
        network::{
            espidf::NetworkRuntime, NetworkLogFingerprint, NetworkSnapshot, WifiConnectionState,
        },
        network_config::{NetworkConfig, WIFI_CONFIG_PATH},
        panel_refresh::{
            PanelGlobalReason, PanelRefreshCoordinator, PanelRefreshPlan, PanelRefreshRequest,
            PANEL_PARTIAL_REFRESH_LIMIT,
        },
        photos::{
            worker::{PhotoJobResult, PhotoWorker},
            StarredPhotos, PHOTOS_DIRECTORY, PHOTO_CACHE_DIRECTORY, STARRED_PATH,
        },
        power::{Axp2101, PowerSnapshot},
        power_key::{
            PowerKeyEvent, PowerKeyPresses, PowerKeySource, SleepWakeGuard, SleepWakeGuardDecision,
            POWER_KEY_GPIO, POWER_KEY_POLL_MS, POWER_KEY_WAKE_GUARD_QUIET_MS,
        },
        power_settings::{PowerPreferences, WakeKeys, POWER_CONFIG_PATH},
        radio_burst::{RadioBurst, RadioPhase},
        reader::ReaderTickOutcome,
        reading_stats::{ReadingStats, READING_STATS_PATH},
        regional::RegionalPreferences,
        rtc::RtcDateTime,
        rtc_alarm_interrupt::{espidf::RtcAlarmInterruptMonitor, RTC_ALARM_INTERRUPT_GPIO},
        runtime_memory::log_runtime_memory,
        shared_i2c::SharedI2cBus,
        sleep_images::{SleepImageCatalog, SleepImageSelection, SLEEP_IMAGE_DIRECTORY},
        sleep_mode::{
            light_sleep_budget, LightSleepShare, SleepModeState, SleepReport, SleepWakeCause,
        },
        sleep_screen::{
            choose_sleep_picture, clock_redraw_wait, SleepScreenSettings, SLEEP_GLOBAL_REFRESH,
            SLEEP_SCREEN_CONFIG_PATH,
        },
        storage::{
            StorageBrowser, StorageSnapshot, StorageUiOutcome, SDMMC_COMMAND_TIMEOUT_MS,
            SDMMC_STABLE_SPEED_KHZ, SD_MOUNT_POINT, STORAGE_IO_RETRY_ATTEMPTS,
        },
        voice_note_metadata::{
            load_voice_notes_preferences, save_voice_notes_preferences, VoiceNotesPreferences,
            VOICE_UNKNOWN_RECORDED_AT,
        },
        voice_notes::{
            cleanup_stale_voice_tmp, delete_voice_note, save_voice_note_title, VoiceNotesUiRequest,
            VoicePlaybackSession, VoiceRecordingSession, VOICE_NOTES_ROOT,
            VOICE_PCM_MONO_CHUNK_BYTES, VOICE_PCM_STEREO_CAPTURE_BYTES,
        },
        weather::{
            espidf::fetch_open_meteo_on_worker, WeatherFetchError, WeatherSnapshot,
            WEATHER_RETRY_DELAYS_SECONDS, WEATHER_RETRY_LIMIT,
        },
        weather_config::{WeatherConfig, WEATHER_CONFIG_PATH},
        wifi_transfer::{
            espidf::WifiTransferServer, WifiTransferSnapshot, WifiTransferState,
            WifiTransferUiRequest, WIFI_TRANSFER_ROOT, WIFI_TRANSFER_SERVER_STACK_BYTES,
        },
    };

    pub fn run() -> Result<()> {
        sys::link_patches();
        EspLogger::initialize_default();
        info!("rustmix-wave=epd397-rust-app-start");
        info!(
            "rustmix-wave=product-ui-shell-start product={PRODUCT_SLUG} version={FIRMWARE_VERSION} milestone={UI_SHELL_MILESTONE}"
        );

        let peripherals = Peripherals::take()?;

        // The uploaded Waveshare sample uses SDMMC in 4-bit mode:
        // CMD GPIO17, CLK GPIO16, D0 GPIO15, D1 GPIO7, D2 GPIO8, D3 GPIO18.
        // Mount failure is non-fatal so the verified product shell still boots
        // when no card is inserted.
        let mounted_sd = (|| {
            let host = SdMmcHostDriver::new_4bits(
                peripherals.sdmmc1,
                peripherals.pins.gpio17,
                peripherals.pins.gpio16,
                peripherals.pins.gpio15,
                peripherals.pins.gpio7,
                peripherals.pins.gpio8,
                peripherals.pins.gpio18,
                None::<AnyIOPin>,
                None::<AnyIOPin>,
                &SdMmcHostConfiguration::new(),
            )?;
            let mut card_config = SdCardConfiguration::new();
            card_config.speed_khz = SDMMC_STABLE_SPEED_KHZ;
            card_config.command_timeout_ms = SDMMC_COMMAND_TIMEOUT_MS;
            let card = SdCardDriver::new_mmc(host, &card_config)?;
            let fatfs = Fatfs::new_sdcard(0, card)?;
            MountedFatfs::mount(fatfs, SD_MOUNT_POINT, 5)
        })();
        let mounted_sd = match mounted_sd {
            Ok(mounted) => {
                info!(
                    "rustmix-wave=sdmmc-mount status=ready mount={SD_MOUNT_POINT} mode=4bit-fat access=ui-readonly speed-khz={SDMMC_STABLE_SPEED_KHZ} timeout-ms={SDMMC_COMMAND_TIMEOUT_MS} retry-attempts={STORAGE_IO_RETRY_ATTEMPTS}"
                );
                Some(mounted)
            }
            Err(error) => {
                warn!(
                    "rustmix-wave=sdmmc-mount status=unavailable mount={SD_MOUNT_POINT} mode=4bit-fat access=ui-readonly speed-khz={SDMMC_STABLE_SPEED_KHZ} timeout-ms={SDMMC_COMMAND_TIMEOUT_MS} retry-attempts={STORAGE_IO_RETRY_ATTEMPTS} error={error:#}"
                );
                None
            }
        };
        let mut storage_browser = StorageBrowser::new(SD_MOUNT_POINT, mounted_sd.is_some());
        let _mounted_sd = mounted_sd;
        let display_preferences = match DisplayPreferences::load_from_path(DISPLAY_CONFIG_PATH) {
            Ok(preferences) => {
                info!(
                    "rustmix-wave=display-config status=ready path={DISPLAY_CONFIG_PATH} font-family={} font-size={}",
                    preferences.font_family.marker(),
                    preferences.font_size.marker()
                );
                preferences
            }
            Err(error) => {
                let preferences = DisplayPreferences::default();
                warn!(
                    "rustmix-wave=display-config status=default path={DISPLAY_CONFIG_PATH} font-family={} font-size={} error={error:#}",
                    preferences.font_family.marker(),
                    preferences.font_size.marker()
                );
                preferences
            }
        };
        let power_preferences = match PowerPreferences::load_from_path(POWER_CONFIG_PATH) {
            Ok(preferences) => {
                info!(
                    "rustmix-wave=power-config status=ready path={POWER_CONFIG_PATH} auto-sleep={} wake-keys={}",
                    preferences.auto_sleep.marker(),
                    preferences.wake_keys.marker()
                );
                preferences
            }
            Err(error) => {
                warn!(
                    "rustmix-wave=power-config status=default path={POWER_CONFIG_PATH} error={error:#}"
                );
                PowerPreferences::default()
            }
        };

        // Credentials are read from removable storage. Never log the password.
        let network_config = match NetworkConfig::load_from_path(WIFI_CONFIG_PATH) {
            Ok(config) => {
                info!(
                    "rustmix-wave=wifi-config status=ready path={WIFI_CONFIG_PATH} ssid={} timezone={} ntp-server={}",
                    config.ssid, config.timezone, config.ntp_server
                );
                Some(config)
            }
            Err(error) => {
                warn!(
                    "rustmix-wave=wifi-config status=unavailable path={WIFI_CONFIG_PATH} error={error:#}"
                );
                None
            }
        };

        let mut weather_config = match WeatherConfig::load_from_path(WEATHER_CONFIG_PATH) {
            Ok(config) => {
                info!(
                    "rustmix-wave=weather-config status=ready path={WEATHER_CONFIG_PATH} provider={} location={} latitude={:.4} longitude={:.4} timezone={} refresh-minutes={} enabled={} units={} show-on-home={}",
                    config.provider,
                    config.location,
                    config.latitude,
                    config.longitude,
                    config.timezone,
                    config.refresh_minutes,
                    config.enabled,
                    config.units.marker(),
                    config.show_on_home
                );
                Some(config)
            }
            Err(error) => {
                warn!(
                    "rustmix-wave=weather-config status=unavailable path={WEATHER_CONFIG_PATH} error={error:#}"
                );
                None
            }
        };

        let mut alarm_engine = match AlarmEngine::load_or_empty(ALARMS_CONFIG_PATH) {
            Ok(engine) => {
                let snapshot = engine.snapshot();
                info!(
                    "rustmix-wave=alarm-config status=ready path={ALARMS_CONFIG_PATH} schedules={} snooze-minutes={}",
                    snapshot.alarms.len(), snapshot.snooze_minutes
                );
                engine
            }
            Err(error) => {
                warn!(
                    "rustmix-wave=alarm-config status=unavailable path={ALARMS_CONFIG_PATH} error={error:#}"
                );
                AlarmEngine::unavailable(format!("{error:#}"))
            }
        };

        let i2c_config = I2cConfig::new().baudrate(400.kHz().into());
        let i2c = I2cDriver::new(
            peripherals.i2c0,
            peripherals.pins.gpio41,
            peripherals.pins.gpio42,
            &i2c_config,
        )?;
        let shared_i2c = SharedI2cBus::new(i2c);
        let panel_power = Axp2101::new(shared_i2c.clone());
        let mut board_services = BoardServices::new(shared_i2c.clone());

        // Bidirectional ES8311 Voice Notes milestone. The uploaded BSP uses I2S0 with
        // MCLK GPIO13, BCLK GPIO14, WS GPIO47, ESP-to-codec DOUT GPIO48,
        // codec-to-ESP DIN GPIO21 and amplifier GPIO39. Start muted with the
        // amplifier disabled; audio failure remains non-fatal.
        info!("rustmix-wave=audio-init status=starting codec=es8311 address=0x18 wire-write=0x30");
        let audio_attempt = (|| -> Result<_> {
            let i2s_config = StdConfig::new(
                I2sChannelConfig::new().auto_clear(true),
                StdClkConfig::new(
                    AUDIO_SAMPLE_RATE_HZ,
                    ClockSource::default(),
                    MclkMultiple::M384,
                ),
                StdSlotConfig::philips_slot_default(DataBitWidth::Bits16, SlotMode::Stereo),
                StdGpioConfig::default(),
            );
            let mut i2s = I2sDriver::<I2sBiDir>::new_std_bidir(
                peripherals.i2s0,
                &i2s_config,
                peripherals.pins.gpio14,
                peripherals.pins.gpio21,
                peripherals.pins.gpio48,
                Some(peripherals.pins.gpio13),
                peripherals.pins.gpio47,
            )?;
            i2s.tx_enable()?;
            i2s.rx_enable()?;
            let amplifier = PinDriver::output(peripherals.pins.gpio39)?;
            AudioRuntime::initialize(shared_i2c.clone(), i2s, amplifier, &mut FreeRtosDelay)
        })();
        let (mut audio_runtime, initial_audio_snapshot) = match audio_attempt {
            Ok(runtime) => {
                let snapshot = runtime.snapshot();
                info!(
                    "rustmix-wave=audio-codec status=ready codec=es8311 address={} wire-write={} mclk-hz={AUDIO_MCLK_HZ}",
                    snapshot.codec_address_label(),
                    snapshot.codec_address.map_or_else(|| "--".into(), |address| format!("0x{:02X}", address << 1))
                );
                let profile = runtime.profile();
                info!(
                    "rustmix-wave=audio-codec-profile status=ready source=waveshare-esp-codec-dev-parity gpio44=0x{:02X} dac-reference=ready system14=0x{:02X} adc15=0x{:02X} adc17=0x{:02X} gp45=0x{:02X}",
                    profile.gpio44,
                    profile.system14,
                    profile.adc15,
                    profile.adc17,
                    profile.gp45
                );
                info!("rustmix-wave=audio-i2s status=ready direction=bidir sample-rate={AUDIO_SAMPLE_RATE_HZ} bits=16 tx-channels=2 rx-channels=2 voice-wav-channels=1 mclk-gpio=13 bclk-gpio=14 ws-gpio=47 dout-gpio=48 din-gpio=21");
                info!("rustmix-wave=audio-amp status=ready gpio=39 default=off");
                info!("rustmix-wave=audio-subsystem-ready mute=true volume={DEFAULT_AUDIO_VOLUME_PERCENT}");
                (Some(runtime), snapshot)
            }
            Err(error) => {
                warn!("rustmix-wave=audio-init status=unavailable codec=es8311 error={error:#}");
                (None, AudioSnapshot::unavailable(format!("{error:#}")))
            }
        };

        let spi_driver_config = SpiDriverConfig::new().dma(Dma::Auto(4096));
        let spi_driver = SpiDriver::new(
            peripherals.spi3,
            peripherals.pins.gpio11,
            peripherals.pins.gpio12,
            None::<AnyIOPin>,
            &spi_driver_config,
        )?;
        let spi_config = SpiConfig::new().baudrate(20.MHz().into()).write_only(true);
        let spi = SpiBusDriver::new(spi_driver, &spi_config)?;

        let dc = PinDriver::output(peripherals.pins.gpio9)?;
        let reset = PinDriver::output(peripherals.pins.gpio46)?;
        let cs = PinDriver::output(peripherals.pins.gpio10)?;
        // GPIO3 is display busy. Do not reuse it for rotary or app input.
        let busy = PinDriver::input(peripherals.pins.gpio3, Pull::Up)?;

        let mut panel = Epaper397::new(spi, dc, reset, cs, busy, FreeRtosDelay, panel_power)?;
        let mut buttons = Buttons::new(
            PinDriver::input(peripherals.pins.gpio4, Pull::Up)?,
            PinDriver::input(peripherals.pins.gpio5, Pull::Up)?,
            PinDriver::input(peripherals.pins.gpio6, Pull::Up)?,
        );
        let mut back_button =
            LongPressBackButton::new(PinDriver::input(peripherals.pins.gpio0, Pull::Up)?);
        info!(
            "rustmix-wave=boot-button-back status=ready gpio=0 active-low=true short-press=contextual-navigation hold-ms={BOOT_BACK_LONG_PRESS_MS}"
        );
        // The uploaded BSP routes the PCF85063 active-low alarm output to
        // GPIO45; the line also wakes the chip from light sleep.
        let mut rtc_alarm_interrupt =
            RtcAlarmInterruptMonitor::new(PinDriver::input(peripherals.pins.gpio45, Pull::Up)?);
        info!(
            "rustmix-wave=rtc-alarm-int status=ready gpio={RTC_ALARM_INTERRUPT_GPIO} active-low=true wake-policy=active-loop-readiness"
        );
        // The AXP2101 interrupt line is open-drain and active-low; it wakes light
        // sleep on a Power-key press. The driver keeps the pull-up configured.
        let _pmic_irq = PinDriver::input(peripherals.pins.gpio38, Pull::Up)?;
        // GPIO1 is the board's PWR_OUT line: high while the Power key is held.
        let power_key_pin = PinDriver::input(peripherals.pins.gpio1, Pull::Floating)?;
        info!(
            "rustmix-wave=power-key-gpio status=ready gpio={POWER_KEY_GPIO} active-high=true held={}",
            power_key_held(&power_key_pin)
        );
        let mut button_delay = FreeRtosDelay;
        let mut service_delay = FreeRtosDelay;
        let mut frame = FrameBuffer::new_white();
        // Keep the growing product UI state off the firmware main-task stack.
        // HTTPS weather retrieval and display refreshes still execute from the
        // same orchestrator, but their stack budget is no longer reduced by a
        // long-lived inline AppState allocation.
        let mut state = Box::new(AppState::default());
        let mut panel_refresh = PanelRefreshCoordinator::default();
        sync_panel_refresh_diagnostics(&mut state, &panel_refresh);
        state.display = display_preferences;
        state.power = power_preferences;
        state.reset_reason = reset_reason_label(unsafe { sys::esp_reset_reason() });
        info!(
            "rustmix-wave=reset-reason last-restart={}",
            state.reset_reason
        );
        if _mounted_sd.is_some() {
            match BatteryLog::load_from_path(BATTERY_LOG_PATH) {
                Ok(log) => state.battery_log = log,
                Err(error) => info!("rustmix-wave=battery-log status=new error={error:#}"),
            }
            match StarredPhotos::load_from_path(STARRED_PATH) {
                Ok(starred) => state.photos.starred = starred,
                Err(error) => info!("rustmix-wave=starred-photos status=none error={error:#}"),
            }
            match SleepScreenSettings::load_from_path(SLEEP_SCREEN_CONFIG_PATH) {
                Ok(settings) => state.sleep_screen = settings,
                Err(error) => info!("rustmix-wave=sleep-screen status=default error={error:#}"),
            }
            state.lua_runtime.records = GameRecords::load_from_path(RECORDS_PATH);
            state.lua_runtime.sudoku_save = SudokuSave::load_from_path(SUDOKU_SAVE_PATH);
            match ReadingStats::load_from_path(READING_STATS_PATH) {
                Ok(stats) => state.reading_stats = stats,
                Err(error) => info!("rustmix-wave=reading-stats status=new error={error:#}"),
            }
            state.bible = BibleUiState::with_root(BIBLE_ROOT);
            // A missing AI.TXT is not an error: the row reads Not set up.
            match AiConfig::load_from_path(AI_CONFIG_PATH) {
                Ok(config) => state.ai = config,
                Err(error) => info!("rustmix-wave=ai-settings status=invalid error={error:#}"),
            }
            info!(
                "rustmix-wave=game-records status=ready tetris-zen={} sudoku-saved={}",
                state.lua_runtime.records.tetris_zen,
                state.lua_runtime.sudoku_save.is_some()
            );
        }
        state.photos.fit = state.sleep_screen.fit;
        let reader_persistence = state.reader.load_persistent_state();
        state.reader.refresh_library();
        if _mounted_sd.is_some() {
            match cleanup_stale_voice_tmp(std::path::Path::new(VOICE_NOTES_ROOT)) {
                Ok(removed) => info!(
                    "rustmix-wave=voice-note-stale-tmp-cleanup status=completed removed={removed} root={VOICE_NOTES_ROOT}"
                ),
                Err(error) => warn!(
                    "rustmix-wave=voice-note-stale-tmp-cleanup status=failed root={VOICE_NOTES_ROOT} error={error:#}"
                ),
            }
        }
        if _mounted_sd.is_some() {
            match load_voice_notes_preferences(std::path::Path::new(VOICE_NOTES_ROOT)) {
                Ok(preferences) => {
                    state.voice_notes.mic_gain = preferences.mic_gain;
                    info!(
                        "rustmix-wave=voice-note-settings-load status=completed mic-gain={} path={VOICE_NOTES_ROOT}/SETTINGS.TXT",
                        preferences.mic_gain.marker()
                    );
                }
                Err(error) => warn!(
                    "rustmix-wave=voice-note-settings-load status=failed path={VOICE_NOTES_ROOT}/SETTINGS.TXT error={error:#}"
                ),
            }
        }
        refresh_voice_note_storage_available(&mut state, _mounted_sd.is_some());
        state.refresh_voice_notes_catalog();
        state.refresh_lua_app_catalog(_mounted_sd.is_some());
        log_lua_runtime_events(&mut state);
        info!(
            "rustmix-wave=reader-persistence-load state-loaded={} preferences-loaded={} positions={} recent={} bookmarks={} warning={}",
            reader_persistence.state_loaded,
            reader_persistence.preferences_loaded,
            reader_persistence.position_count,
            reader_persistence.recent_count,
            reader_persistence.bookmark_count,
            reader_persistence.warning.as_deref().unwrap_or("none")
        );
        let mut sleep_images = SleepImageCatalog::default();
        let mut photo_worker = PhotoWorker::new(PHOTOS_DIRECTORY, PHOTO_CACHE_DIRECTORY);
        let mut photo_jobs_queued = false;
        let mut sleep_mode = SleepModeState::default();
        let mut sleep_wake_guard = SleepWakeGuard::default();
        let mut sleep_wake_guard_started_at: Option<Instant> = None;
        let mut radio = RadioBurst::default();
        let mut manual_weather_pending = false;
        let mut sleep_started: Option<(Instant, Option<PowerSnapshot>)> = None;
        // Live sleep screens: the next timed redraw and the last global one.
        let mut sleep_redraw_at: Option<Instant> = None;
        let mut sleep_last_global = Instant::now();
        // Awake time since the last wake, and the part of it spent light-sleeping.
        let mut light_sleep_clock = (Instant::now(), Duration::ZERO);
        let mut imu_enabled = true;
        let mut on_usb_power = false;
        let mut last_usb_check: Option<Instant> = None;
        // Set by a Power-key or button press that ends sleep mode.
        let mut wake_cause: Option<SleepWakeCause> = None;
        let mut last_battery_sample: Option<Instant> = None;
        let mut last_stats_save: Option<Instant> = None;
        state.update_audio_snapshot(initial_audio_snapshot);
        log_audio_snapshot(&state.audio);
        if let Some(config) = network_config.as_ref() {
            state.regional = state.regional.with_timezone_name(&config.timezone)?;
            state.update_network_snapshot(NetworkSnapshot::provisioned(config));
        }
        if let Some(config) = weather_config.as_ref() {
            state.update_weather_snapshot(WeatherSnapshot::provisioned(config));
        }
        state.set_weather_config(weather_config.clone());
        state.update_alarm_snapshot(alarm_engine.snapshot());
        state.update_storage_snapshot(storage_browser.snapshot());
        log_storage_snapshot(&state.storage);
        info!(
            "rustmix-wave=regional-profile timezone={} display-offset={} rtc-storage-offset={} temperature-unit={}",
            state.regional.timezone_name(),
            state.regional.timezone_label_for_rtc(state.board.rtc),
            state.regional.rtc_storage_label(),
            state.regional.temperature_unit.marker()
        );

        let init = board_services.initialize(&mut service_delay);
        info!(
            "rustmix-wave=sample-board-services-init rtc={} environment={} power={} imu={} rtc-integrity-lost={} shtc3-id={} qmi8658-address={} qmi8658-revision={}",
            init.rtc_available,
            init.environment_available,
            init.power_monitoring_available,
            init.imu_available,
            init.rtc_clock_integrity_was_lost,
            init.environment_sensor_id
                .map_or_else(|| "unavailable".into(), |id| format!("0x{id:04X}")),
            init.imu_address
                .map_or_else(|| "unavailable".into(), |value| format!("0x{value:02X}")),
            init.imu_revision
                .map_or_else(|| "unavailable".into(), |value| format!("0x{value:02X}"))
        );
        let mut power_key_retry_at: Option<Instant> = None;
        state.power_key.use_gpio_line();
        match board_services.initialize_power_key_events() {
            Ok(previous_ms) => {
                state.power_key.succeeded();
                info!("rustmix-wave=power-key status=ready source=axp2101-pek events=short-menu,long-sleep poll-ms={POWER_KEY_POLL_MS} long-press-ms=1000 previous-long-press-ms={previous_ms}");
            }
            Err(error) => {
                let pause = state.power_key.failed(format!("{error:#}"));
                power_key_retry_at = Some(Instant::now() + Duration::from_millis(pause));
                warn!(
                    "rustmix-wave=power-key status=unavailable source=axp2101-pek retry-ms={pause} error={error:#}"
                );
            }
        }
        state.update_board_snapshot(board_services.read_snapshot(&mut service_delay));
        log_board_snapshot(state.board, state.regional);
        if let Some(rtc) = state.board.rtc {
            alarm_engine.recompute_next(state.regional.localize_rtc(rtc));
        }
        sync_alarm_hardware(&mut alarm_engine, &mut board_services, state.regional);
        state.update_alarm_snapshot(alarm_engine.snapshot());
        log_alarm_snapshot(&state.alarms);

        panel.initialize()?;
        render_current_screen(&mut frame, &state)?;
        panel.show_base(frame.as_bytes())?;
        panel_refresh.reset_after_external_global(PanelGlobalReason::InitialBoot);
        sync_panel_refresh_diagnostics(&mut state, &panel_refresh);
        info!(
            "rustmix-wave=panel-refresh plan=global-base reason=initial-boot transport=global-base"
        );
        info!("rustmix-wave=epd397-rust-display-ready");

        // Wi-Fi stays off except for short bursts driven by the main loop, so
        // a missing config or an unreachable network never blocks the shell.
        let mut network_runtime = if let Some(config) = network_config.as_ref() {
            match NetworkRuntime::new(peripherals.modem, config) {
                Ok(runtime) => {
                    info!(
                        "rustmix-wave=wifi-radio status=ready ssid={} policy=bursts",
                        config.ssid
                    );
                    runtime
                }
                Err(error) => {
                    warn!(
                        "rustmix-wave=wifi-radio status=failed ssid={} error={error:#}",
                        config.ssid
                    );
                    NetworkRuntime::failed(config, format!("{error:#}"))
                }
            }
        } else {
            NetworkRuntime::configuration_missing()
        };
        state.update_network_snapshot(network_runtime.snapshot());
        log_network_snapshot(&state.network);
        let mut last_network_log = Instant::now();
        let mut last_network_fingerprint = state.network.log_fingerprint();
        // Explicitly activated only.  Normal boot never starts the portal.
        let mut wifi_transfer_server: Option<WifiTransferServer> = None;
        state.update_wifi_transfer_snapshot(WifiTransferSnapshot::default());
        let mut voice_recording: Option<VoiceRecordingSession> = None;
        let mut voice_playback: Option<VoicePlaybackSession> = None;
        let mut voice_stereo_buffer = vec![0_u8; VOICE_PCM_STEREO_CAPTURE_BYTES];
        let mut voice_mono_buffer = vec![0_u8; VOICE_PCM_MONO_CHUNK_BYTES];
        info!("rustmix-wave=imu-event-thresholds tilt-mg={} shake-delta-mg={} rotate-dps={} level-tolerance-mg={} debounce-ms={}", state.imu_events.thresholds.tilt_enter_mg, state.imu_events.thresholds.shake_delta_mg, state.imu_events.thresholds.rotate_dps, state.imu_events.thresholds.level_tolerance_mg, state.imu_events.thresholds.debounce_ms);
        log_runtime_memory("boot-complete");
        info!(
            "rustmix-wave=voice-notes-catalog status=completed notes={} root={VOICE_NOTES_ROOT}",
            state.voice_notes.notes.len()
        );

        let mut last_activity = Instant::now();
        let mut last_status_refresh = Instant::now();
        let mut last_alarm_poll = Instant::now();
        let mut last_power_key_poll = Instant::now();
        // Milliseconds since boot for the Power key, game and reading clocks.
        let uptime = Instant::now();
        // A key still held from powering on is not a press.
        let mut power_presses = PowerKeyPresses::new(power_key_held(&power_key_pin));
        let mut last_weather_attempt: Option<Instant> = None;
        let mut last_reader_tick = Instant::now();
        let imu_event_started_at = Instant::now();
        let mut last_imu_event_sample = Instant::now();
        let mut last_imu_event_screen_refresh = Instant::now();
        let mut weather_retry = WeatherRetryState::default();
        let mut last_voice_record_refresh = Instant::now();
        loop {
            maintain_wifi_transfer_server(
                &mut wifi_transfer_server,
                &mut state,
                &mut storage_browser,
                _mounted_sd.is_some(),
            );
            if state.panel_awake
                && last_activity.elapsed() >= Duration::from_secs(PANEL_IDLE_SLEEP_SECONDS)
            {
                panel.sleep()?;
                state.panel_awake = false;
                info!("rustmix-wave=epd397-panel-sleep");
            }

            let mut voice_capture_failure = None;
            if let Some(session) = voice_recording.as_mut() {
                if state.voice_notes.recording_paused {
                    let discard = audio_runtime
                        .as_mut()
                        .ok_or_else(|| {
                            anyhow::anyhow!(
                                "audio runtime unavailable during paused voice recording"
                            )
                        })
                        .and_then(|runtime| runtime.discard_voice_pcm(&mut voice_stereo_buffer));
                    if let Err(error) = discard {
                        voice_capture_failure = Some(format!("{error:#}"));
                    }
                } else {
                    let capture = audio_runtime
                        .as_mut()
                        .ok_or_else(|| {
                            anyhow::anyhow!("audio runtime unavailable during voice recording")
                        })
                        .and_then(|runtime| {
                            runtime.read_voice_pcm_mono(
                                &mut voice_stereo_buffer,
                                &mut voice_mono_buffer,
                                state.voice_notes.mic_gain,
                            )
                        });
                    match capture {
                        Ok(metrics) if metrics.bytes > 0 => {
                            session.add_clipped_samples(metrics.clipped_samples);
                            if let Err(error) =
                                session.append_pcm16_mono(&voice_mono_buffer[..metrics.bytes])
                            {
                                voice_capture_failure = Some(format!("{error:#}"));
                            } else {
                                state.voice_notes.update_recording_progress(
                                    session.pcm_bytes(),
                                    session.peak(),
                                    session.clipped_samples(),
                                );
                            }
                        }
                        Ok(_) => {}
                        Err(error) => voice_capture_failure = Some(format!("{error:#}")),
                    }
                }
                if voice_capture_failure.is_none()
                    && state.panel_awake
                    && state.active_route() == ScreenRoute::VoiceNoteRecording
                    && last_voice_record_refresh.elapsed()
                        >= Duration::from_secs(VOICE_RECORD_SCREEN_REFRESH_SECONDS)
                {
                    if state.voice_notes.recording_paused {
                        info!("rustmix-wave=voice-record status=paused file={} elapsed-seconds={} pcm-bytes={} peak={} clipped-samples={} mic-gain={}", session.file_name(), state.voice_notes.elapsed_seconds, session.pcm_bytes(), session.peak(), session.clipped_samples(), state.voice_notes.mic_gain.marker());
                    } else {
                        info!("rustmix-wave=voice-record status=active file={} elapsed-seconds={} pcm-bytes={} peak={} clipped-samples={} mic-gain={}", session.file_name(), state.voice_notes.elapsed_seconds, session.pcm_bytes(), session.peak(), session.clipped_samples(), state.voice_notes.mic_gain.marker());
                    }
                    refresh_screen(
                        &mut panel,
                        &mut frame,
                        &mut state,
                        &mut panel_refresh,
                        RefreshRequest::Normal,
                    )?;
                    last_voice_record_refresh = Instant::now();
                }
            }
            if let Some(error) = voice_capture_failure {
                warn!("rustmix-wave=voice-record status=failed stage=capture error={error}");
                if let Some(active) = voice_recording.take() {
                    let _ = active.cancel();
                }
                if let Some(runtime) = audio_runtime.as_mut() {
                    let _ = runtime.finish_voice_recording();
                    state.update_audio_snapshot(runtime.snapshot());
                }
                state.voice_notes.fail(error);
                log_runtime_memory("after-voice-record-stop");
            }

            let mut voice_playback_finished = None;
            let mut voice_playback_failure = None;
            if voice_recording.is_none() {
                if let Some(session) = voice_playback.as_mut() {
                    match session.read_pcm16_mono(&mut voice_mono_buffer) {
                        Ok(0) => voice_playback_finished = Some(session.file_name().to_string()),
                        Ok(bytes) => {
                            let output = audio_runtime
                                .as_mut()
                                .ok_or_else(|| {
                                    anyhow::anyhow!(
                                        "audio runtime unavailable during voice-note playback"
                                    )
                                })
                                .and_then(|runtime| {
                                    runtime.write_voice_pcm16_mono(
                                        &voice_mono_buffer[..bytes],
                                        &mut voice_stereo_buffer,
                                    )
                                });
                            match output {
                                Ok(()) => {
                                    state.voice_notes.update_playback_progress(
                                        session.played_pcm_bytes(),
                                        session.total_pcm_bytes(),
                                    );
                                    if session.is_complete() {
                                        voice_playback_finished =
                                            Some(session.file_name().to_string());
                                    }
                                }
                                Err(error) => {
                                    voice_playback_failure = Some(format!("{error:#}"));
                                }
                            }
                        }
                        Err(error) => voice_playback_failure = Some(format!("{error:#}")),
                    }
                }
            }
            if let Some(file_name) = voice_playback_finished {
                stop_voice_note_playback(
                    &mut voice_playback,
                    &mut audio_runtime,
                    &mut state,
                    "completed",
                );
                info!("rustmix-wave=voice-note-playback status=completed file={file_name}");
                if state.panel_awake && state.active_route() == ScreenRoute::VoiceNoteDetails {
                    refresh_screen(
                        &mut panel,
                        &mut frame,
                        &mut state,
                        &mut panel_refresh,
                        RefreshRequest::Normal,
                    )?;
                }
            }
            if let Some(error) = voice_playback_failure {
                warn!("rustmix-wave=voice-note-playback status=failed error={error}");
                stop_voice_note_playback(
                    &mut voice_playback,
                    &mut audio_runtime,
                    &mut state,
                    "stream-error",
                );
                state.voice_notes.fail(format!("Playback failed: {error}"));
                if state.panel_awake && state.active_route() == ScreenRoute::VoiceNoteDetails {
                    refresh_screen(
                        &mut panel,
                        &mut frame,
                        &mut state,
                        &mut panel_refresh,
                        RefreshRequest::Normal,
                    )?;
                }
            }

            if voice_recording.is_none() && voice_playback.is_none() {
                if let Some(runtime) = audio_runtime.as_mut() {
                    match runtime.tick() {
                        Ok(changed) => {
                            let latest = runtime.snapshot();
                            if latest != state.audio {
                                state.update_audio_snapshot(latest);
                                log_audio_snapshot(&state.audio);
                                if changed
                                    && state.panel_awake
                                    && matches!(
                                        state.active_route(),
                                        ScreenRoute::Audio
                                            | ScreenRoute::AudioDetails
                                            | ScreenRoute::Alarms
                                    )
                                {
                                    refresh_screen(
                                        &mut panel,
                                        &mut frame,
                                        &mut state,
                                        &mut panel_refresh,
                                        RefreshRequest::Normal,
                                    )?;
                                }
                            }
                        }
                        Err(error) => {
                            warn!(
                                "rustmix-wave=audio-event outcome=playback-error error={error:#}"
                            );
                            runtime.record_failure(format!("{error:#}"));
                            state.update_audio_snapshot(runtime.snapshot());
                            log_audio_snapshot(&state.audio);
                        }
                    }
                }
            }

            // Wi-Fi bursts: switch the radio on for due or requested work and
            // off again once that work is done.
            let mut manual_weather_refresh = state.take_weather_refresh_request();
            manual_weather_refresh |= core::mem::take(&mut manual_weather_pending);
            let weather_due = next_weather_refresh(weather_config.as_ref(), last_weather_attempt);
            let transfer_waiting = wifi_transfer_server.is_none()
                && state.wifi_transfer.state == WifiTransferState::Starting;
            if let Some(config) = network_config.as_ref().filter(|_| {
                network_runtime.has_radio()
                    && (!sleep_mode.is_sleeping() || weather_while_asleep(&state))
            }) {
                let now = Instant::now();
                match radio.phase() {
                    RadioPhase::Off => {
                        let reason = if manual_weather_refresh && state.weather_enabled() {
                            Some("weather-refresh")
                        } else if transfer_waiting {
                            Some("wifi-transfer")
                        } else if voice_recording.is_none()
                            && voice_playback.is_none()
                            && now >= radio.next_burst(now, weather_due, last_activity)
                        {
                            Some("scheduled")
                        } else {
                            None
                        };
                        if let Some(reason) = reason {
                            info!("rustmix-wave=wifi-burst status=starting reason={reason}");
                            match network_runtime.begin_connect() {
                                Ok(()) => radio.begin(now),
                                Err(error) => {
                                    warn!("rustmix-wave=wifi-burst status=failed error={error:#}");
                                    network_runtime.fail_connect(format!("{error:#}"));
                                    radio.failed(now);
                                }
                            }
                            state.update_network_snapshot(network_runtime.snapshot());
                        }
                    }
                    RadioPhase::Connecting => {
                        let outcome = match network_runtime.poll_connect(config) {
                            Ok(false) if radio.connect_timed_out(now) => {
                                Err("connection timed out".to_string())
                            }
                            Ok(connected) => Ok(connected),
                            Err(error) => Err(format!("{error:#}")),
                        };
                        match outcome {
                            Ok(true) => {
                                radio.connected(now);
                                info!(
                                    "rustmix-wave=wifi-burst status=connected ssid={}",
                                    config.ssid
                                );
                                state.update_network_snapshot(network_runtime.snapshot());
                            }
                            Ok(false) => {}
                            Err(error) => {
                                warn!("rustmix-wave=wifi-burst status=failed error={error}");
                                network_runtime.fail_connect(error);
                                radio.failed(now);
                                state.update_network_snapshot(network_runtime.snapshot());
                            }
                        }
                    }
                    RadioPhase::Connected => {
                        let weather_settled = weather_due.map_or(true, |due| due > now)
                            && !weather_retry.is_pending()
                            && !manual_weather_refresh;
                        if !transfer_waiting
                            && wifi_transfer_server.is_none()
                            && radio.work_done(now, weather_settled)
                        {
                            radio_off(
                                &mut network_runtime,
                                &mut state,
                                &mut last_network_fingerprint,
                                &mut last_network_log,
                                "burst-done",
                            );
                            radio.finished(now);
                        }
                    }
                }
            }
            if transfer_waiting && radio.phase() != RadioPhase::Connecting {
                if radio.phase() == RadioPhase::Connected {
                    start_wifi_transfer_server(&mut wifi_transfer_server, &mut state);
                } else {
                    let error = if network_runtime.has_radio() {
                        "Wi-Fi did not connect"
                    } else {
                        "Connect Wi-Fi before starting transfer"
                    };
                    warn!("rustmix-wave=wifi-transfer-server status=start-rejected error={error}");
                    state.update_wifi_transfer_snapshot(WifiTransferSnapshot::failed(error));
                }
                if state.panel_awake && state.active_route() == ScreenRoute::WifiTransfer {
                    refresh_screen(
                        &mut panel,
                        &mut frame,
                        &mut state,
                        &mut panel_refresh,
                        RefreshRequest::Normal,
                    )?;
                }
            }

            if radio.phase() == RadioPhase::Connected {
                if let Some(utc) = network_runtime.tick() {
                    radio.time_synced(Instant::now());
                    info!(
                        "rustmix-wave=sntp-sync status=completed utc={}",
                        utc.date_time()
                    );
                    match board_services.sync_rtc_from_utc(utc) {
                        Ok(stored) => info!(
                            "rustmix-wave=rtc-sync status=updated storage-basis={} stored={}",
                            state.regional.rtc_storage_label(),
                            stored.date_time()
                        ),
                        Err(error) => warn!("rustmix-wave=rtc-sync status=failed error={error:#}"),
                    }
                    state.update_board_snapshot(board_services.read_snapshot(&mut service_delay));
                    log_board_snapshot(state.board, state.regional);
                    if let Some(rtc) = state.board.rtc {
                        alarm_engine.recompute_next(state.regional.localize_rtc(rtc));
                        sync_alarm_hardware(&mut alarm_engine, &mut board_services, state.regional);
                        state.update_alarm_snapshot(alarm_engine.snapshot());
                        log_alarm_snapshot(&state.alarms);
                    }
                }
                let latest_network = network_runtime.snapshot();
                if latest_network != state.network {
                    state.update_network_snapshot(latest_network);
                }
                let latest_fingerprint = state.network.log_fingerprint();
                if latest_fingerprint != last_network_fingerprint
                    || last_network_log.elapsed()
                        >= Duration::from_secs(NETWORK_LOG_HEARTBEAT_SECONDS)
                {
                    log_network_snapshot(&state.network);
                    last_network_fingerprint = latest_fingerprint;
                    last_network_log = Instant::now();
                }
            }

            if alarm_engine.should_poll()
                && last_alarm_poll.elapsed() >= Duration::from_secs(ALARM_POLL_SECONDS)
            {
                match board_services.read_rtc() {
                    Ok(rtc) => {
                        let local = state.regional.localize_rtc(rtc);
                        let interrupt_sample = rtc_alarm_interrupt.sample();
                        if interrupt_sample.changed {
                            info!(
                                "rustmix-wave=rtc-alarm-int status={} gpio={RTC_ALARM_INTERRUPT_GPIO} level={}",
                                interrupt_sample.level.marker(),
                                interrupt_sample.level.raw_level_marker()
                            );
                        }
                        let hardware_flag = match board_services.take_rtc_alarm_flag() {
                            Ok(flag) => flag,
                            Err(error) => {
                                warn!("rustmix-wave=rtc-alarm-flag status=unavailable error={error:#}");
                                false
                            }
                        };
                        let outcome =
                            alarm_engine.poll(local, hardware_flag || interrupt_sample.asserted());
                        if outcome.schedule_changed {
                            sync_alarm_hardware(
                                &mut alarm_engine,
                                &mut board_services,
                                state.regional,
                            );
                        }
                        state.update_alarm_snapshot(alarm_engine.snapshot());
                        if outcome.triggered {
                            if let Some(active) = voice_recording.take() {
                                let _ = active.cancel();
                                if let Some(runtime) = audio_runtime.as_mut() {
                                    let _ = runtime.finish_voice_recording();
                                    state.update_audio_snapshot(runtime.snapshot());
                                }
                                state.voice_notes.cancel_recording();
                                info!("rustmix-wave=voice-record status=cancelled reason=alarm-trigger");
                            }
                            if voice_playback.is_some() {
                                stop_voice_note_playback(
                                    &mut voice_playback,
                                    &mut audio_runtime,
                                    &mut state,
                                    "alarm-trigger",
                                );
                            }
                            info!(
                                "rustmix-wave=alarm-triggered active={} local={} hardware-flag={hardware_flag} interrupt-low={}",
                                state.alarms.active.as_ref().map_or("alarm", |active| active.name.as_str()),
                                local.date_time(),
                                interrupt_sample.asserted()
                            );
                            if let Some(runtime) = audio_runtime.as_mut() {
                                match runtime.start_alarm_chime() {
                                    Ok(()) => {
                                        info!("rustmix-wave=audio-event outcome=alarm-chime-start")
                                    }
                                    Err(error) => {
                                        warn!("rustmix-wave=audio-event outcome=alarm-chime-failed error={error:#}");
                                        runtime.record_failure(format!("{error:#}"));
                                    }
                                }
                                state.update_audio_snapshot(runtime.snapshot());
                                log_audio_snapshot(&state.audio);
                            } else {
                                warn!("rustmix-wave=audio-event outcome=alarm-chime-unavailable fallback=visual-only");
                            }
                            let woke_from_sleep = !state.panel_awake;
                            if sleep_mode.is_sleeping() {
                                let _ = sleep_mode.exit(SleepWakeCause::RtcAlarm);
                                sleep_wake_guard.reset_after_wake();
                                sleep_wake_guard_started_at = None;
                                end_sleep(&mut board_services, &mut state, &mut sleep_started);
                                light_sleep_clock = (Instant::now(), Duration::ZERO);
                                info!("rustmix-wave=sleep-mode-exit cause=rtc-alarm restore-route=alarms");
                            }
                            if woke_from_sleep {
                                panel.initialize()?;
                                state.panel_awake = true;
                                panel_refresh
                                    .reset_after_external_global(PanelGlobalReason::AfterWake);
                                sync_panel_refresh_diagnostics(&mut state, &panel_refresh);
                            }
                            state.router.navigate_to(ScreenRoute::Alarms);
                            info!("rustmix-wave=screen-route route=alarms cause=alarm-trigger");
                            if woke_from_sleep {
                                info!(
                                    "rustmix-wave=wake-global-refresh reason=rtc-alarm-sleep-image"
                                );
                            }
                            refresh_screen(
                                &mut panel,
                                &mut frame,
                                &mut state,
                                &mut panel_refresh,
                                if woke_from_sleep {
                                    RefreshRequest::ForceGlobalAfterWake
                                } else {
                                    RefreshRequest::Normal
                                },
                            )?;
                            last_activity = Instant::now();
                            last_status_refresh = Instant::now();
                        }
                    }
                    Err(error) => {
                        warn!("rustmix-wave=rtc-alarm-poll status=unavailable error={error:#}")
                    }
                }
                last_alarm_poll = Instant::now();
            }

            if sleep_mode.is_sleeping() {
                if let Some(started_at) = sleep_wake_guard_started_at.as_ref() {
                    let elapsed_ms = started_at.elapsed().as_millis() as u64;
                    if sleep_wake_guard.arm_after_quiet_window(elapsed_ms) {
                        info!(
                            "rustmix-wave=sleep-wake-guard status=ready-for-new-wake-press minimum-quiet-ms={POWER_KEY_WAKE_GUARD_QUIET_MS}"
                        );
                    }
                }
            }

            if power_key_retry_at.is_some_and(|at| Instant::now() >= at) {
                match board_services.initialize_power_key_events() {
                    Ok(_) => {
                        state.power_key.succeeded();
                        power_key_retry_at = None;
                        info!(
                            "rustmix-wave=power-key status=ready source=axp2101-pek errors={}",
                            state.power_key.errors()
                        );
                    }
                    Err(error) => {
                        let pause = state.power_key.failed(format!("{error:#}"));
                        power_key_retry_at = Some(Instant::now() + Duration::from_millis(pause));
                        warn!("rustmix-wave=power-key status=retrying source=axp2101-pek retry-ms={pause} error={error:#}");
                    }
                }
                last_power_key_poll = Instant::now();
            }

            let auto_sleep = state.power.auto_sleep.delay();
            // Without the Power key, only sleep when another key can wake.
            let can_wake = state.power_key.can_wake() || state.power.wake_keys == WakeKeys::AnyKey;
            let auto_sleep_due = can_wake
                && !sleep_mode.is_sleeping()
                && auto_sleep.is_some_and(|delay| last_activity.elapsed() >= delay)
                && state.alarms.active.is_none()
                && voice_recording.is_none()
                && voice_playback.is_none()
                && wifi_transfer_server.is_none();
            let power_key_now_ms = uptime.elapsed().as_millis() as u64;
            let power_key_was_down = power_presses.is_down();
            let power_key_down = power_key_held(&power_key_pin);
            let gpio_event = power_presses.update(power_key_down, power_key_now_ms);
            let power_key_is_down = power_presses.is_down();
            if power_key_is_down != power_key_was_down {
                info!("rustmix-wave=power-key-gpio down={power_key_is_down}");
            }
            let power_key_poll_due = state.power_key.is_ready()
                && last_power_key_poll.elapsed() >= Duration::from_millis(POWER_KEY_POLL_MS);
            if gpio_event.is_some() || auto_sleep_due || power_key_poll_due {
                let source = if gpio_event.is_some() {
                    PowerKeySource::Gpio
                } else if auto_sleep_due {
                    PowerKeySource::AutoSleep
                } else {
                    PowerKeySource::Pmic
                };
                let event = match source {
                    PowerKeySource::Gpio => Ok(gpio_event),
                    // Idle auto-sleep takes the same path as holding the Power key.
                    PowerKeySource::AutoSleep => {
                        info!(
                            "rustmix-wave=auto-sleep idle={}",
                            state.power.auto_sleep.marker()
                        );
                        Ok(Some(PowerKeyEvent::LongPress))
                    }
                    PowerKeySource::Pmic => board_services.take_power_key_event(),
                };
                // Once GPIO1 has shown a press, the PMIC's key events repeat it.
                let duplicate = source == PowerKeySource::Pmic && power_presses.seen_press();
                match event {
                    Ok(Some(event)) if duplicate => {
                        info!(
                            "rustmix-wave=power-key event={} source=axp2101-pek outcome=ignored reason=gpio1-reads-the-key",
                            event.marker()
                        );
                    }
                    Ok(Some(event)) => {
                        info!(
                            "rustmix-wave=power-key event={} source={}",
                            event.marker(),
                            source.marker()
                        );
                        if sleep_mode.is_sleeping() {
                            let elapsed_ms = sleep_wake_guard_started_at
                                .as_ref()
                                .map_or(0, |started_at| started_at.elapsed().as_millis() as u64);
                            if sleep_wake_guard.on_power_press(elapsed_ms)
                                == SleepWakeGuardDecision::SuppressStalePress
                            {
                                info!(
                                    "rustmix-wave=sleep-wake-guard event=stale-power-key-suppressed source={} elapsed-ms={elapsed_ms} minimum-quiet-ms={POWER_KEY_WAKE_GUARD_QUIET_MS}",
                                    source.marker()
                                );
                                last_power_key_poll = Instant::now();
                                continue;
                            }
                            wake_cause = Some(SleepWakeCause::PowerKey);
                        } else if event == PowerKeyEvent::ShortPress {
                            if state.alarms.active.is_some() {
                                warn!(
                                    "rustmix-wave=power-key-menu outcome=rejected reason=active-alarm"
                                );
                            } else {
                                state.open_power_key_menu();
                                // The panel powers down after a minute idle.
                                let woke_from_sleep = !state.panel_awake;
                                if woke_from_sleep {
                                    panel.initialize()?;
                                    state.panel_awake = true;
                                    panel_refresh
                                        .reset_after_external_global(PanelGlobalReason::AfterWake);
                                    sync_panel_refresh_diagnostics(&mut state, &panel_refresh);
                                }
                                refresh_screen(
                                    &mut panel,
                                    &mut frame,
                                    &mut state,
                                    &mut panel_refresh,
                                    if woke_from_sleep {
                                        RefreshRequest::ForceGlobalAfterWake
                                    } else {
                                        RefreshRequest::Normal
                                    },
                                )?;
                                info!(
                                    "rustmix-wave=power-key-menu outcome=opened return-route={}",
                                    state.power_key_sleep_restore_route().marker()
                                );
                                last_activity = Instant::now();
                                last_status_refresh = Instant::now();
                            }
                        } else if state.alarms.active.is_some() {
                            warn!(
                                "rustmix-wave=sleep-mode-enter status=rejected reason=active-alarm"
                            );
                        } else {
                            stop_wifi_transfer_server(
                                &mut wifi_transfer_server,
                                &mut state,
                                &mut storage_browser,
                                _mounted_sd.is_some(),
                                "sleep-entry",
                            );
                            if let Some(active) = voice_recording.take() {
                                let _ = active.cancel();
                                if let Some(runtime) = audio_runtime.as_mut() {
                                    let _ = runtime.finish_voice_recording();
                                    state.update_audio_snapshot(runtime.snapshot());
                                }
                                state.voice_notes.cancel_recording();
                                info!(
                                    "rustmix-wave=voice-record status=cancelled reason=sleep-entry"
                                );
                            }
                            if voice_playback.is_some() {
                                stop_voice_note_playback(
                                    &mut voice_playback,
                                    &mut audio_runtime,
                                    &mut state,
                                    "sleep-entry",
                                );
                            }
                            if let Some(runtime) = audio_runtime.as_mut() {
                                match runtime.stop_playback() {
                                    Ok(()) => info!("rustmix-wave=audio-event outcome=playback-stop reason=sleep-mode"),
                                    Err(error) => {
                                        warn!("rustmix-wave=audio-event outcome=playback-stop-failed reason=sleep-mode error={error:#}");
                                        runtime.record_failure(format!("{error:#}"));
                                    }
                                }
                                state.update_audio_snapshot(runtime.snapshot());
                                log_audio_snapshot(&state.audio);
                            }
                            let live = state.sleep_screen.mode.is_live();
                            let selection = (!live).then(|| {
                                let selection = next_sleep_picture(
                                    &state,
                                    &mut sleep_images,
                                    sleep_mode.last_image(),
                                );
                                log_sleep_image_selection(&selection);
                                selection
                            });
                            if radio.phase() != RadioPhase::Off {
                                if !radio_off(
                                    &mut network_runtime,
                                    &mut state,
                                    &mut last_network_fingerprint,
                                    &mut last_network_log,
                                    "sleep-entry",
                                ) {
                                    warn!("rustmix-wave=sleep-mode-enter status=rejected reason=network-suspend-failed");
                                    last_activity = Instant::now();
                                    continue;
                                }
                                radio.finished(Instant::now());
                            }
                            if !state.panel_awake {
                                panel.initialize()?;
                                state.panel_awake = true;
                            }
                            let restore_route = state.power_key_sleep_restore_route();
                            let battery = board_services.read_power().ok();
                            let (image_label, layout) = match selection {
                                // No usable SD picture: say why instead.
                                Some(selection) => {
                                    if let Some(note) = selection.note.as_deref() {
                                        let battery_percent =
                                            battery.and_then(|power| power.battery_percent);
                                        let card = SleepCard {
                                            note,
                                            battery_percent,
                                            wake_hint: state.power.wake_keys.wake_hint(),
                                        };
                                        render_sleep_card(&mut frame, state.display, &card)?;
                                    } else {
                                        frame = selection.frame;
                                    }
                                    (selection.file_name, None)
                                }
                                None => {
                                    state.update_board_snapshot(
                                        board_services.read_snapshot(&mut service_delay),
                                    );
                                    let layout = render_sleep_mode(&mut frame, &state)?;
                                    (format!("live-{}", layout.marker()), Some(layout))
                                }
                            };
                            panel.show_base(frame.as_bytes())?;
                            panel_refresh
                                .reset_after_external_global(PanelGlobalReason::SleepImage);
                            sync_panel_refresh_diagnostics(&mut state, &panel_refresh);
                            info!("rustmix-wave=panel-refresh plan=global-base reason=sleep-image transport=global-base");
                            sleep_mode.enter(restore_route, image_label.clone());
                            sleep_wake_guard.begin_sleep_entry();
                            sleep_wake_guard_started_at = Some(Instant::now());
                            info!(
                                "rustmix-wave=sleep-wake-guard status=waiting-for-quiet-window minimum-quiet-ms={POWER_KEY_WAKE_GUARD_QUIET_MS} policy=suppress-stale-power-key"
                            );
                            // Live screens redraw while asleep, so the panel keeps its RAM.
                            if live {
                                panel.sleep_keeping_ram()?;
                                sleep_redraw_at =
                                    layout.and_then(|layout| next_sleep_redraw(&state, layout));
                                sleep_last_global = Instant::now();
                            } else {
                                panel.sleep()?;
                                sleep_redraw_at = None;
                            }
                            state.panel_awake = false;
                            if _mounted_sd.is_some() && state.battery_log.has_unsaved() {
                                save_battery_log(&mut state.battery_log);
                            }
                            if _mounted_sd.is_some() {
                                save_reading_stats_if_dirty(&mut state);
                                state.bible.save_if_changed();
                            }
                            sleep_started = Some((Instant::now(), battery));
                            info!(
                                "rustmix-wave=light-sleep-share asleep-seconds={} awake-seconds={}",
                                state.light_sleep.asleep_seconds, state.light_sleep.awake_seconds
                            );
                            info!(
                                "rustmix-wave=sleep-mode-enter image={} mode={} restore-route={} display=global-refresh panel={} wifi=off network-services=paused mcu-sleep=light",
                                image_label,
                                state.sleep_screen.mode.marker(),
                                restore_route.marker(),
                                if live { "deep-sleep-ram aldo3=on" } else { "deep-sleep aldo3=off" }
                            );
                        }
                    }
                    Ok(None) => {}
                    Err(error) => {
                        let pause = state.power_key.failed(format!("{error:#}"));
                        power_key_retry_at = Some(Instant::now() + Duration::from_millis(pause));
                        warn!("rustmix-wave=power-key status=unavailable source=axp2101-pek retry-ms={pause} error={error:#}");
                    }
                }
                last_power_key_poll = Instant::now();
            }

            if let Some(cause) = wake_cause.take() {
                sleep_wake_guard.reset_after_wake();
                sleep_wake_guard_started_at = None;
                sleep_redraw_at = None;
                end_sleep(&mut board_services, &mut state, &mut sleep_started);
                light_sleep_clock = (Instant::now(), Duration::ZERO);
                let restore_route = sleep_mode.exit(cause);
                panel.initialize()?;
                state.panel_awake = true;
                state.router.navigate_to(restore_route);
                render_current_screen(&mut frame, &state)?;
                panel.show_base(frame.as_bytes())?;
                panel_refresh.reset_after_external_global(PanelGlobalReason::AfterWake);
                sync_panel_refresh_diagnostics(&mut state, &panel_refresh);
                info!("rustmix-wave=panel-refresh plan=global-base reason=after-wake transport=global-base");
                info!(
                    "rustmix-wave=sleep-mode-exit cause={} restore-route={}",
                    cause.marker(),
                    restore_route.marker()
                );
                last_activity = Instant::now();
                last_status_refresh = Instant::now();
            }

            // The live sleep screen: partial redraws, a global one every half hour.
            if sleep_mode.is_sleeping() && sleep_redraw_at.is_some_and(|at| Instant::now() >= at) {
                state.update_board_snapshot(board_services.read_snapshot(&mut service_delay));
                let layout = render_sleep_mode(&mut frame, &state)?;
                let global = sleep_last_global.elapsed() >= SLEEP_GLOBAL_REFRESH;
                if global {
                    panel.initialize()?;
                    panel.show_base(frame.as_bytes())?;
                    sleep_last_global = Instant::now();
                } else {
                    panel.show_partial_fullscreen(frame.as_bytes())?;
                }
                panel.sleep_keeping_ram()?;
                sleep_redraw_at = next_sleep_redraw(&state, layout);
                info!(
                    "rustmix-wave=sleep-redraw layout={} refresh={}",
                    layout.marker(),
                    if global { "global" } else { "partial" }
                );
            }

            if !sleep_mode.is_sleeping() || weather_while_asleep(&state) {
                let enabled_weather = weather_config.as_ref().filter(|config| config.enabled);
                if enabled_weather.is_none() {
                    weather_retry.clear();
                }
                if let Some(config) = enabled_weather {
                    if manual_weather_refresh {
                        weather_retry.clear();
                    }
                    let wifi_connected = radio.phase() == RadioPhase::Connected;
                    // Manual means no automatic updates, not even at start.
                    let interval_due = config.refresh_interval().is_some_and(|interval| {
                        last_weather_attempt.map_or(true, |last| last.elapsed() >= interval)
                    });
                    let scheduled_retry = if wifi_connected {
                        weather_retry.take_due()
                    } else {
                        None
                    };
                    let attempt = if manual_weather_refresh {
                        Some(WeatherFetchAttempt::initial("manual"))
                    } else if let Some(retry) = scheduled_retry {
                        Some(retry)
                    } else if interval_due && !weather_retry.is_pending() {
                        Some(WeatherFetchAttempt::initial(
                            if last_weather_attempt.is_none() {
                                "network-ready"
                            } else {
                                "periodic"
                            },
                        ))
                    } else {
                        None
                    };

                    if let Some(attempt) = attempt {
                        if wifi_connected {
                            run_weather_fetch_attempt(
                                config,
                                attempt,
                                &mut weather_retry,
                                &mut state,
                            );
                            last_weather_attempt = Some(Instant::now());
                            if sleep_mode.is_sleeping() {
                                sleep_redraw_at = Some(Instant::now());
                            }
                            if state.panel_awake
                                && matches!(
                                    state.active_route(),
                                    ScreenRoute::Home
                                        | ScreenRoute::Weather
                                        | ScreenRoute::WeatherDetails
                                )
                            {
                                refresh_screen(
                                    &mut panel,
                                    &mut frame,
                                    &mut state,
                                    &mut panel_refresh,
                                    RefreshRequest::Normal,
                                )?;
                            }
                        } else if manual_weather_refresh && radio.phase() == RadioPhase::Connecting
                        {
                            manual_weather_pending = true;
                        } else if manual_weather_refresh {
                            state.weather.record_failure("Wi-Fi is not connected");
                            warn!("rustmix-wave=weather-fetch status=deferred cause=manual error=wifi-not-connected");
                            if state.panel_awake
                                && matches!(
                                    state.active_route(),
                                    ScreenRoute::Weather | ScreenRoute::WeatherDetails
                                )
                            {
                                refresh_screen(
                                    &mut panel,
                                    &mut frame,
                                    &mut state,
                                    &mut panel_refresh,
                                    RefreshRequest::Normal,
                                )?;
                            }
                        }
                    }
                } else if manual_weather_refresh {
                    state
                        .weather
                        .record_failure("weather configuration is missing");
                    warn!("rustmix-wave=weather-fetch status=deferred cause=manual error=weather-config-missing");
                    if state.panel_awake
                        && matches!(
                            state.active_route(),
                            ScreenRoute::Weather | ScreenRoute::WeatherDetails
                        )
                    {
                        refresh_screen(
                            &mut panel,
                            &mut frame,
                            &mut state,
                            &mut panel_refresh,
                            RefreshRequest::Normal,
                        )?;
                    }
                }
            }

            if !sleep_mode.is_sleeping()
                && matches!(
                    state.active_route(),
                    ScreenRoute::ReaderLoading | ScreenRoute::ReaderPage
                )
                && last_reader_tick.elapsed() >= Duration::from_millis(250)
            {
                let previous_route = state.active_route();
                let outcome = state.tick_reader();
                match outcome {
                    ReaderTickOutcome::FirstPageReady => {
                        info!("rustmix-wave=reader-first-page-ready route={} cache-policy=lazy-nearby-pages", state.active_route().marker());
                    }
                    ReaderTickOutcome::BackgroundCacheAdvanced => {
                        if let Some(session) = state.reader.session.as_ref() {
                            info!("rustmix-wave=reader-background-cache indexed-percent={} pages={} complete={}", session.progress_percent(), session.page_offsets.len(), session.index_complete);
                        }
                    }
                    ReaderTickOutcome::Failed => {
                        warn!(
                            "rustmix-wave=reader-cache-stage status=failed route={}",
                            state.active_route().marker()
                        );
                    }
                    ReaderTickOutcome::None => {}
                }
                apply_wifi_transfer_ui_request(
                    &mut wifi_transfer_server,
                    &mut state,
                    &mut storage_browser,
                    _mounted_sd.is_some(),
                    voice_recording.is_some(),
                    voice_playback.is_some(),
                );
                log_reader_persistence_event(&mut state);
                // The Reader closed (library, home or another book): flush
                // the session's time instead of waiting for the cadence.
                if !state.reader_page_open()
                    && state.reading_stats.has_unsaved()
                    && _mounted_sd.is_some()
                {
                    save_reading_stats_if_dirty(&mut state);
                }
                if state.panel_awake
                    && (outcome == ReaderTickOutcome::FirstPageReady
                        || outcome == ReaderTickOutcome::Failed
                        || state.active_route() != previous_route)
                {
                    refresh_screen(
                        &mut panel,
                        &mut frame,
                        &mut state,
                        &mut panel_refresh,
                        RefreshRequest::Normal,
                    )?;
                    last_activity = Instant::now();
                }
                last_reader_tick = Instant::now();
            }

            // Classic Tetris falls on its own clock and pauses while the panel
            // is off; a tick is not a key press, so `last_activity` stays put.
            if !sleep_mode.is_sleeping()
                && state.panel_awake
                && state.tick_lua_game(uptime.elapsed().as_millis() as u64)
            {
                refresh_screen(
                    &mut panel,
                    &mut frame,
                    &mut state,
                    &mut panel_refresh,
                    RefreshRequest::Normal,
                )?;
            }

            // Photos are prepared in the background while the gallery is open.
            let photos_open = !sleep_mode.is_sleeping()
                && matches!(
                    state.active_route(),
                    ScreenRoute::Photos | ScreenRoute::PhotoViewer
                );
            if photos_open && state.photos.delete_request().is_none() {
                if state.photos.take_jobs_changed() || !photo_jobs_queued {
                    photo_worker.set_jobs(state.photos.cache_jobs());
                    photo_jobs_queued = true;
                }
            } else if core::mem::take(&mut photo_jobs_queued) {
                photo_worker.set_jobs(Vec::new());
            }
            let mut photos_changed = false;
            for result in photo_worker.poll() {
                if let PhotoJobResult::Failed { key, reason } = &result {
                    warn!("rustmix-wave=photo-prepare status=failed key={key:08X} {reason}");
                }
                photos_changed |= state.photos.on_job_result(&result);
            }
            // The worker may be reading the photo, so delete it once idle.
            if !photo_worker.is_busy() {
                if let Some(name) = state.photos.delete_request().map(str::to_string) {
                    let path = state.photos.photos_directory().join(&name);
                    let deleted = std::fs::remove_file(path);
                    match &deleted {
                        Ok(()) => info!("rustmix-wave=photo-delete file={name}"),
                        Err(error) => warn!("rustmix-wave=photo-delete file={name} error={error}"),
                    }
                    state.finish_photo_delete(deleted.is_ok());
                    photos_changed = true;
                }
            }
            if state.photos.take_starred_changed() {
                match state.photos.starred.save_to_path(STARRED_PATH) {
                    Ok(()) => info!(
                        "rustmix-wave=starred-photos-write status=saved count={}",
                        state.photos.starred.len()
                    ),
                    Err(error) => warn!("rustmix-wave=starred-photos-write error={error:#}"),
                }
            }
            if state.lua_runtime.take_records_changed() {
                match state.lua_runtime.records.save_to_path(RECORDS_PATH) {
                    Ok(()) => info!(
                        "rustmix-wave=game-records-write status=saved tetris-zen={}",
                        state.lua_runtime.records.tetris_zen
                    ),
                    Err(error) => warn!("rustmix-wave=game-records-write error={error:#}"),
                }
            }
            if state.lua_runtime.take_sudoku_save_changed() {
                match state.lua_runtime.sudoku_save.clone() {
                    Some(save) => match save.save_to_path(SUDOKU_SAVE_PATH) {
                        Ok(()) => info!(
                            "rustmix-wave=sudoku-save-write status=saved seconds={}",
                            save.seconds
                        ),
                        Err(error) => warn!("rustmix-wave=sudoku-save-write error={error:#}"),
                    },
                    // A solved game removes the resume file.
                    None => SudokuSave::remove_from_path(SUDOKU_SAVE_PATH),
                }
            }
            if photos_changed && photos_open && state.panel_awake {
                let request = if state.take_full_refresh() {
                    RefreshRequest::ForceGlobalManual
                } else {
                    RefreshRequest::Normal
                };
                refresh_screen(
                    &mut panel,
                    &mut frame,
                    &mut state,
                    &mut panel_refresh,
                    request,
                )?;
            }

            // The IMU only runs on the motion diagnostic screens.
            let route = state.active_route();
            let imu_sampling = route == ScreenRoute::MotionEvents;
            let motion_screen = matches!(route, ScreenRoute::Motion | ScreenRoute::MotionDetails);
            let imu_wanted = !sleep_mode.is_sleeping() && (imu_sampling || motion_screen);
            if imu_wanted != imu_enabled {
                imu_enabled = imu_wanted;
                match board_services.set_imu_enabled(imu_wanted) {
                    Ok(()) => info!("rustmix-wave=imu-power enabled={imu_wanted}"),
                    Err(error) => warn!(
                        "rustmix-wave=imu-power status=failed enabled={imu_wanted} error={error:#}"
                    ),
                }
            }

            if !sleep_mode.is_sleeping()
                && state.panel_awake
                && imu_sampling
                && last_imu_event_sample.elapsed()
                    >= Duration::from_millis(IMU_EVENT_SAMPLE_INTERVAL_MS)
            {
                match board_services.read_imu_motion() {
                    Ok(reading) => {
                        let now_ms = imu_event_started_at.elapsed().as_millis() as u64;
                        let event = state.update_imu_event_sample(reading, now_ms);
                        if let Some(event) = event {
                            info!("rustmix-wave=imu-event type={} detail={} at-ms={} samples={} counts=tilt:{},shake:{},rotate:{},level:{} thresholds=tilt:{}mg,shake:{}mg,rotate:{}dps,level:{}mg,debounce:{}ms", event.kind.marker(), event.kind.detail_marker(), event.at_ms, state.imu_events.samples, state.imu_events.counters.tilt, state.imu_events.counters.shake, state.imu_events.counters.rotate, state.imu_events.counters.level, state.imu_events.thresholds.tilt_enter_mg, state.imu_events.thresholds.shake_delta_mg, state.imu_events.thresholds.rotate_dps, state.imu_events.thresholds.level_tolerance_mg, state.imu_events.thresholds.debounce_ms);
                        }
                        let diagnostic_refresh = state.active_route() == ScreenRoute::MotionEvents
                            && (event.is_some()
                                || last_imu_event_screen_refresh.elapsed()
                                    >= Duration::from_secs(IMU_EVENT_SCREEN_REFRESH_SECONDS));
                        if diagnostic_refresh {
                            refresh_screen(
                                &mut panel,
                                &mut frame,
                                &mut state,
                                &mut panel_refresh,
                                RefreshRequest::Normal,
                            )?;
                            last_imu_event_screen_refresh = Instant::now();
                        }
                    }
                    Err(error) => {
                        warn!("rustmix-wave=imu-event-sample status=unavailable error={error:#}")
                    }
                }
                last_imu_event_sample = Instant::now();
            }

            let live_refresh_seconds = match state.active_route() {
                ScreenRoute::Motion | ScreenRoute::MotionDetails => MOTION_LIVE_REFRESH_SECONDS,
                ScreenRoute::Network | ScreenRoute::NetworkDetails => NETWORK_LIVE_REFRESH_SECONDS,
                _ => SAMPLE_LIVE_REFRESH_SECONDS,
            };
            if state.panel_awake
                && state.active_route().uses_live_status()
                && last_status_refresh.elapsed() >= Duration::from_secs(live_refresh_seconds)
            {
                state.update_board_snapshot(board_services.read_snapshot(&mut service_delay));
                log_board_snapshot(state.board, state.regional);
                refresh_screen(
                    &mut panel,
                    &mut frame,
                    &mut state,
                    &mut panel_refresh,
                    RefreshRequest::Normal,
                )?;
                info!(
                    "rustmix-wave=sample-board-status-auto-refresh route={}",
                    state.active_route().marker()
                );
                last_status_refresh = Instant::now();
            }

            match back_button.poll(&mut button_delay, || power_key_held(&power_key_pin))? {
                Some(BootButtonEvent::LongPress) => {
                    info!(
                        "rustmix-wave=boot-button event=long-press action=back hold-ms={BOOT_BACK_LONG_PRESS_MS}"
                    );
                    if sleep_mode.is_sleeping() {
                        wake_cause = button_wake(state.power, "boot-long-press");
                        FreeRtos::delay_ms(20);
                        continue;
                    }
                    let woke_from_sleep = !state.panel_awake;
                    if woke_from_sleep {
                        panel.initialize()?;
                        state.panel_awake = true;
                        panel_refresh.reset_after_external_global(PanelGlobalReason::AfterWake);
                        sync_panel_refresh_diagnostics(&mut state, &panel_refresh);
                    }
                    state.update_board_snapshot(board_services.read_snapshot(&mut service_delay));
                    log_board_snapshot(state.board, state.regional);
                    let previous_route = state.active_route();
                    if previous_route == ScreenRoute::Home {
                        info!("rustmix-wave=hierarchical-back outcome=ignored route=home");
                    } else {
                        state.back();
                        if previous_route == ScreenRoute::BibleReading {
                            state.bible.save_if_changed();
                        }
                        apply_voice_notes_ui_request(
                            &mut voice_recording,
                            &mut voice_playback,
                            &mut audio_runtime,
                            &mut state,
                            _mounted_sd.is_some(),
                        );
                        apply_wifi_transfer_ui_request(
                            &mut wifi_transfer_server,
                            &mut state,
                            &mut storage_browser,
                            _mounted_sd.is_some(),
                            voice_recording.is_some(),
                            voice_playback.is_some(),
                        );
                        log_lua_runtime_events(&mut state);
                        info!(
                            "rustmix-wave=hierarchical-back outcome=navigated from={} to={}",
                            previous_route.marker(),
                            state.active_route().marker()
                        );
                        info!(
                            "rustmix-wave=screen-route route={}",
                            state.active_route().marker()
                        );
                    }
                    // Back may only close a list or a picture, so redraw unless on Home.
                    if woke_from_sleep || previous_route != ScreenRoute::Home {
                        let full_refresh = state.take_full_refresh();
                        let request = if woke_from_sleep {
                            RefreshRequest::ForceGlobalAfterWake
                        } else if full_refresh {
                            RefreshRequest::ForceGlobalManual
                        } else {
                            RefreshRequest::Normal
                        };
                        refresh_screen(
                            &mut panel,
                            &mut frame,
                            &mut state,
                            &mut panel_refresh,
                            request,
                        )?;
                    }
                    last_activity = Instant::now();
                    last_status_refresh = Instant::now();
                }
                Some(BootButtonEvent::ShortPress) => {
                    info!(
                        "rustmix-wave=boot-button event=short-press action=contextual-navigation"
                    );
                    if sleep_mode.is_sleeping() {
                        wake_cause = button_wake(state.power, "boot-short-press");
                        FreeRtos::delay_ms(20);
                        continue;
                    }
                    state.event_clock_ms = uptime.elapsed().as_millis() as u64;
                    let calendar_agenda_context = state.apply_calendar_boot_short_press();
                    let keyboard_context = if calendar_agenda_context {
                        false
                    } else {
                        state.apply_keyboard_boot_short_press()
                    };
                    let lua_game_context = if calendar_agenda_context || keyboard_context {
                        false
                    } else {
                        state.apply_lua_game_boot_short_press()
                    };
                    let screen_context = state.apply_photos_boot_short_press()
                        || state.apply_bible_boot_short_press()
                        || state.apply_sleep_screen_boot_short_press()
                        || state.apply_weather_boot_short_press()
                        || state.apply_ai_hub_boot_short_press();
                    if calendar_agenda_context
                        || keyboard_context
                        || lua_game_context
                        || screen_context
                    {
                        if calendar_agenda_context {
                            info!("rustmix-wave=calendar-agenda route=selected-day outcome=opened");
                        }
                        if keyboard_context {
                            if state.active_route() == ScreenRoute::CalendarEventEditor {
                                if let Some(editor) = state.calendar.editor.as_ref() {
                                    info!(
                                        "rustmix-wave=calendar-editor-keyboard-nav axis={} outcome=toggled",
                                        editor.navigation_mode_label()
                                    );
                                }
                            } else if state.active_route() == ScreenRoute::VoiceNoteDetails
                                && state.voice_notes.title_editing
                            {
                                info!(
                                    "rustmix-wave=voice-note-title-keyboard-nav axis={} outcome=toggled",
                                    state.voice_notes.title_editor_navigation_mode_label()
                                );
                            } else {
                                info!(
                                    "rustmix-wave=dictionary-keyboard-nav axis={} outcome=toggled",
                                    state.dictionary.navigation_mode_label()
                                );
                            }
                        }
                        let woke_from_sleep = !state.panel_awake;
                        // The AI hub's short BOOT queues a new recording.
                        apply_voice_notes_ui_request(
                            &mut voice_recording,
                            &mut voice_playback,
                            &mut audio_runtime,
                            &mut state,
                            _mounted_sd.is_some(),
                        );
                        if woke_from_sleep {
                            panel.initialize()?;
                            state.panel_awake = true;
                            panel_refresh.reset_after_external_global(PanelGlobalReason::AfterWake);
                            sync_panel_refresh_diagnostics(&mut state, &panel_refresh);
                        }
                        state.update_board_snapshot(
                            board_services.read_snapshot(&mut service_delay),
                        );
                        log_board_snapshot(state.board, state.regional);
                        log_lua_runtime_events(&mut state);
                        sync_weather_config(&state, &mut weather_config);
                        if state.take_sleep_preview_request() {
                            let previous = sleep_mode.last_image();
                            let picture = sleep_preview(&state, &sleep_images, previous)?;
                            state.show_sleep_preview(picture);
                        }
                        let full_refresh = state.take_full_refresh();
                        let request = if woke_from_sleep {
                            RefreshRequest::ForceGlobalAfterWake
                        } else if full_refresh {
                            RefreshRequest::ForceGlobalManual
                        } else {
                            RefreshRequest::Normal
                        };
                        refresh_screen(
                            &mut panel,
                            &mut frame,
                            &mut state,
                            &mut panel_refresh,
                            request,
                        )?;
                        last_activity = Instant::now();
                        last_status_refresh = Instant::now();
                    } else {
                        info!(
                            "rustmix-wave=boot-button event=short-press action=ignored route={}",
                            state.active_route().marker()
                        );
                    }
                }
                None => {}
            }

            let now_ms = uptime.elapsed().as_millis() as u64;
            let repeats = state.active_route().repeats_keys();
            if let Some(event) = buttons.poll(&mut button_delay, now_ms, repeats)? {
                info!("rustmix-wave=button-event event={event:?}");
                if sleep_mode.is_sleeping() {
                    wake_cause = button_wake(state.power, "wheel");
                    FreeRtos::delay_ms(20);
                    continue;
                }
                let woke_from_sleep = !state.panel_awake;
                if woke_from_sleep {
                    panel.initialize()?;
                    state.panel_awake = true;
                    panel_refresh.reset_after_external_global(PanelGlobalReason::AfterWake);
                    sync_panel_refresh_diagnostics(&mut state, &panel_refresh);
                }

                state.update_board_snapshot(board_services.read_snapshot(&mut service_delay));
                log_board_snapshot(state.board, state.regional);
                let previous_route = state.active_route();
                let previous_display = state.display;
                let previous_power = state.power;
                let previous_sleep_screen = state.sleep_screen;
                if previous_route == ScreenRoute::Files {
                    apply_storage_event(&mut storage_browser, &mut state, event);
                } else if previous_route == ScreenRoute::Alarms {
                    let local = state
                        .board
                        .rtc
                        .map_or_else(fallback_local_time, |rtc| state.regional.localize_rtc(rtc));
                    let outcome = apply_alarm_event(&mut alarm_engine, &mut state, event, local);
                    if matches!(
                        outcome,
                        AlarmUiOutcome::Saved | AlarmUiOutcome::Snoozed | AlarmUiOutcome::Dismissed
                    ) {
                        sync_alarm_hardware(&mut alarm_engine, &mut board_services, state.regional);
                        state.update_alarm_snapshot(alarm_engine.snapshot());
                        log_alarm_snapshot(&state.alarms);
                    }
                    if alarm_engine.take_changed() {
                        match alarm_engine.save_to_path(ALARMS_CONFIG_PATH) {
                            Ok(()) => info!(
                                "rustmix-wave=alarm-config status=saved path={ALARMS_CONFIG_PATH}"
                            ),
                            Err(error) => warn!(
                                "rustmix-wave=alarm-config status=save-failed error={error:#}"
                            ),
                        }
                    }
                    if matches!(outcome, AlarmUiOutcome::Snoozed | AlarmUiOutcome::Dismissed) {
                        if let Some(runtime) = audio_runtime.as_mut() {
                            match runtime.stop_playback() {
                                Ok(()) => info!("rustmix-wave=audio-event outcome=alarm-chime-stop reason={outcome:?}"),
                                Err(error) => {
                                    warn!("rustmix-wave=audio-event outcome=alarm-chime-stop-failed error={error:#}");
                                    runtime.record_failure(format!("{error:#}"));
                                }
                            }
                            state.update_audio_snapshot(runtime.snapshot());
                            log_audio_snapshot(&state.audio);
                        }
                    }
                } else if previous_route == ScreenRoute::Audio {
                    if let Some(request) = state.apply_audio_button(event) {
                        apply_audio_request(&mut audio_runtime, &mut state, request);
                    }
                } else {
                    // Game and reading timers measure between key presses.
                    state.apply_with_clock(event, uptime.elapsed().as_millis() as u64);
                    log_lua_runtime_events(&mut state);
                    if state.active_route() == ScreenRoute::Files {
                        storage_browser.refresh();
                        state.update_storage_snapshot(storage_browser.snapshot());
                        log_storage_snapshot(&state.storage);
                    }
                }
                // Consume Settings > Network transfer start/stop intents before
                // rendering the next frame.  This guarantees that the transfer
                // route shows READY plus its LAN URL and code on the same normal
                // partial refresh that follows the SELECT event.
                apply_calendar_ui_request(&mut state, _mounted_sd.is_some());
                apply_voice_notes_ui_request(
                    &mut voice_recording,
                    &mut voice_playback,
                    &mut audio_runtime,
                    &mut state,
                    _mounted_sd.is_some(),
                );
                apply_wifi_transfer_ui_request(
                    &mut wifi_transfer_server,
                    &mut state,
                    &mut storage_browser,
                    _mounted_sd.is_some(),
                    voice_recording.is_some(),
                    voice_playback.is_some(),
                );
                log_reader_persistence_event(&mut state);
                if state.take_ai_changed() {
                    if let Some(config) = state.ai.as_ref() {
                        match config.save_to_path(AI_CONFIG_PATH) {
                            Ok(()) => {
                                info!("rustmix-wave=ai-settings status=saved path={AI_CONFIG_PATH}")
                            }
                            Err(error) => {
                                warn!("rustmix-wave=ai-settings status=save-failed error={error:#}")
                            }
                        }
                    }
                }
                if state.display != previous_display {
                    match state.display.save_to_path(DISPLAY_CONFIG_PATH) {
                        Ok(()) => info!(
                            "rustmix-wave=display-config-write status=saved path={DISPLAY_CONFIG_PATH}"
                        ),
                        Err(error) => warn!(
                            "rustmix-wave=display-config-write status=failed path={DISPLAY_CONFIG_PATH} error={error:#}"
                        ),
                    }
                    info!(
                        "rustmix-wave=display-settings-updated font-family={} font-size={} persistence=sd-file path={DISPLAY_CONFIG_PATH}",
                        state.display.font_family.marker(),
                        state.display.font_size.marker()
                    );
                }
                if state.power != previous_power {
                    match state.power.save_to_path(POWER_CONFIG_PATH) {
                        Ok(()) => info!("rustmix-wave=power-config-write status=saved"),
                        Err(error) => warn!("rustmix-wave=power-config-write error={error:#}"),
                    }
                    info!(
                        "rustmix-wave=power-settings-updated auto-sleep={} wake-keys={}",
                        state.power.auto_sleep.marker(),
                        state.power.wake_keys.marker()
                    );
                }
                if state.sleep_screen != previous_sleep_screen {
                    match state.sleep_screen.save_to_path(SLEEP_SCREEN_CONFIG_PATH) {
                        Ok(()) => info!("rustmix-wave=sleep-screen-write status=saved"),
                        Err(error) => warn!("rustmix-wave=sleep-screen-write error={error:#}"),
                    }
                    info!(
                        "rustmix-wave=sleep-screen-updated mode={} source={} order={} fit={} clock-refresh={}",
                        state.sleep_screen.mode.marker(),
                        state.sleep_screen.source.marker(),
                        state.sleep_screen.order.marker(),
                        state.sleep_screen.fit.marker(),
                        state.sleep_screen.clock_refresh.marker()
                    );
                }
                sync_weather_config(&state, &mut weather_config);
                if state.active_route() != previous_route {
                    info!(
                        "rustmix-wave=screen-route route={}",
                        state.active_route().marker()
                    );
                }
                let reader_clear_ghost = state.take_reader_clear_ghost_request();
                let power_key_clear_ghost = state.take_power_key_manual_refresh_request();
                let full_refresh = state.take_full_refresh();
                let request = if woke_from_sleep {
                    RefreshRequest::ForceGlobalAfterWake
                } else if reader_clear_ghost || power_key_clear_ghost || full_refresh {
                    RefreshRequest::ForceGlobalManual
                } else {
                    RefreshRequest::Normal
                };
                refresh_screen(
                    &mut panel,
                    &mut frame,
                    &mut state,
                    &mut panel_refresh,
                    request,
                )?;
                last_activity = Instant::now();
                last_status_refresh = Instant::now();
            }

            if last_battery_sample.map_or(true, |at| at.elapsed() >= BATTERY_SAMPLE_INTERVAL) {
                record_battery_sample(&mut board_services, &mut state, _mounted_sd.is_some());
                last_battery_sample = Some(Instant::now());
            }

            // Reading stats save at most every five minutes while it changed.
            if last_stats_save.map_or(true, |at| at.elapsed() >= READING_STATS_SAVE_INTERVAL) {
                if _mounted_sd.is_some() {
                    save_reading_stats_if_dirty(&mut state);
                }
                last_stats_save = Some(Instant::now());
            }

            // While awake, light-sleep between presses until the next timed job.
            // On USB power stay awake so flashing and the serial console work.
            let reader_open = matches!(
                state.active_route(),
                ScreenRoute::ReaderLoading | ScreenRoute::ReaderPage
            );
            let idle = !sleep_mode.is_sleeping()
                && radio.phase() == RadioPhase::Off
                && wifi_transfer_server.is_none()
                && voice_recording.is_none()
                && voice_playback.is_none()
                && state.alarms.active.is_none()
                && !state.audio.playback_state.is_streaming()
                && !(reader_open && state.reader.has_background_work())
                && !(state.panel_awake && imu_sampling)
                && !photo_worker.is_busy()
                && last_activity.elapsed() >= IDLE_LIGHT_SLEEP_DELAY;
            let usb_check_due =
                last_usb_check.map_or(true, |at| at.elapsed() >= USB_CHECK_INTERVAL);
            if (idle || sleep_mode.is_sleeping()) && usb_check_due {
                on_usb_power = board_services
                    .read_power()
                    .is_ok_and(|power| power.vbus_present);
                last_usb_check = Some(Instant::now());
            }
            // A stuck line, or one an RTC alarm holds, must not block light sleep.
            let power_key_wakes = rtc_line_idle() && !power_presses.is_stuck(power_key_now_ms);
            let slept = if sleep_mode.is_sleeping() {
                let lines = match state.power.wake_keys {
                    WakeKeys::AnyKey => &AWAKE_WAKE_GPIOS[..],
                    WakeKeys::PowerKey => &SLEEP_WAKE_GPIOS[..],
                };
                let now = Instant::now();
                let weather_due =
                    next_weather_refresh(weather_config.as_ref(), last_weather_attempt);
                let burst_due = (network_config.is_some()
                    && network_runtime.has_radio()
                    && weather_while_asleep(&state))
                .then(|| {
                    radio
                        .next_burst(now, weather_due, last_activity)
                        .saturating_duration_since(now)
                });
                let budget = light_sleep_budget(&[
                    sleep_redraw_at.map(|at| at.saturating_duration_since(now)),
                    burst_due,
                ]);
                // Light sleep drops the USB serial port, and consoles then reset the
                // board; it would also stall the photo worker in an SD write.
                !on_usb_power
                    && !photo_worker.is_busy()
                    && sleep_wake_guard.is_armed()
                    && radio.phase() == RadioPhase::Off
                    && budget.is_some_and(|budget| {
                        light_sleep_until_wake(lines, power_key_wakes, budget)
                    })
            } else if idle && !on_usb_power {
                let now = Instant::now();
                let weather_due =
                    next_weather_refresh(weather_config.as_ref(), last_weather_attempt);
                let burst_due = radio
                    .next_burst(now, weather_due, last_activity)
                    .saturating_duration_since(now);
                let live_status = state.panel_awake && state.active_route().uses_live_status();
                let alarm_polling = alarm_engine.should_poll() && !state.alarms.hardware_programmed;
                let budget = light_sleep_budget(&[
                    state
                        .panel_awake
                        .then(|| time_left(last_activity, PANEL_IDLE_SLEEP_SECONDS)),
                    auto_sleep
                        .filter(|_| can_wake)
                        .map(|delay| delay.saturating_sub(last_activity.elapsed())),
                    power_key_retry_at.map(|at| at.saturating_duration_since(now)),
                    live_status.then(|| time_left(last_status_refresh, live_refresh_seconds)),
                    alarm_polling.then(|| time_left(last_alarm_poll, ALARM_POLL_SECONDS)),
                    network_runtime.has_radio().then_some(burst_due),
                    state
                        .next_lua_game_tick_ms()
                        .filter(|_| state.panel_awake)
                        .map(|at| Duration::from_millis(at).saturating_sub(uptime.elapsed())),
                ]);
                // The RTC line only needs watching while an alarm is programmed.
                let lines = if state.alarms.hardware_programmed {
                    &AWAKE_WAKE_GPIOS[..]
                } else {
                    &AWAKE_WAKE_GPIOS[1..]
                };
                budget.is_some_and(|budget| {
                    let started = Instant::now();
                    let slept = light_sleep_until_wake(lines, power_key_wakes, budget);
                    if slept {
                        light_sleep_clock.1 += started.elapsed();
                    }
                    slept
                })
            } else {
                false
            };
            if !slept {
                FreeRtos::delay_ms(20);
            }
            state.light_sleep = LightSleepShare {
                asleep_seconds: light_sleep_clock.1.as_secs(),
                awake_seconds: light_sleep_clock.0.elapsed().as_secs(),
            };
        }
    }

    /// AXP2101 interrupt line; low while a Power-key event is latched.
    const PMIC_IRQ_GPIO: i32 = 38;
    /// Lines that end the sleep-image light sleep, besides the Power key.
    static SLEEP_WAKE_GPIOS: [i32; 2] = [PMIC_IRQ_GPIO, RTC_ALARM_INTERRUPT_GPIO as i32];
    /// While awake BOOT (GPIO0) and the wheel (GPIO4-6) wake it too. The RTC
    /// line comes first so it can be left out when no alarm is programmed.
    static AWAKE_WAKE_GPIOS: [i32; 6] =
        [RTC_ALARM_INTERRUPT_GPIO as i32, PMIC_IRQ_GPIO, 0, 4, 5, 6];
    /// Grace after the last press before the idle loop light-sleeps.
    const IDLE_LIGHT_SLEEP_DELAY: Duration = Duration::from_millis(300);
    const USB_CHECK_INTERVAL: Duration = Duration::from_secs(5);
    const BATTERY_SAMPLE_INTERVAL: Duration = Duration::from_secs(SAMPLE_MINUTES as u64 * 60);
    const READING_STATS_SAVE_INTERVAL: Duration = Duration::from_secs(300);

    fn time_left(since: Instant, period_seconds: u64) -> Duration {
        Duration::from_secs(period_seconds).saturating_sub(since.elapsed())
    }

    /// GPIO1 is high while the Power key is held. An RTC alarm pulls the same
    /// key line low, so GPIO1 is only the key while GPIO45 is idle.
    fn power_key_held(power_key: &PinDriver<'_, Input>) -> bool {
        power_key.is_high() && rtc_line_idle()
    }

    fn rtc_line_idle() -> bool {
        let level = unsafe { sys::gpio_get_level(RTC_ALARM_INTERRUPT_GPIO as i32) };
        level != 0
    }

    /// While asleep, BOOT and the wheel end sleep only when the wake-keys
    /// setting allows any key.
    fn button_wake(power: PowerPreferences, source: &str) -> Option<SleepWakeCause> {
        if power.wake_keys == WakeKeys::AnyKey {
            Some(SleepWakeCause::Button)
        } else {
            info!("rustmix-wave=sleep-mode-input-suppressed source={source}");
            None
        }
    }

    /// The picture for the next sleep, as set in Settings › Sleep screen.
    fn next_sleep_picture(
        state: &AppState,
        folder: &mut SleepImageCatalog,
        previous: Option<&str>,
    ) -> SleepImageSelection {
        let photos = &state.photos;
        let roots = (photos.photos_directory(), photos.cache_directory());
        let random = unsafe { sys::esp_random() };
        choose_sleep_picture(
            state.sleep_screen,
            &photos.starred,
            roots,
            folder,
            previous,
            random,
        )
    }

    /// What the next sleep would show, or the card that explains why it shows
    /// no picture. A copy of the folder keeps its order where it was.
    fn sleep_preview(
        state: &AppState,
        folder: &SleepImageCatalog,
        previous: Option<&str>,
    ) -> Result<FrameBuffer> {
        if state.sleep_screen.mode.is_live() {
            let mut picture = FrameBuffer::new_white();
            let layout = render_sleep_mode(&mut picture, state)?;
            info!("rustmix-wave=sleep-preview layout={}", layout.marker());
            return Ok(picture);
        }
        let selection = next_sleep_picture(state, &mut folder.clone(), previous);
        log_sleep_image_selection(&selection);
        let mut picture = selection.frame;
        if let Some(note) = selection.note.as_deref() {
            let card = SleepCard {
                note,
                battery_percent: state.board.power.and_then(|power| power.battery_percent),
                wake_hint: state.power.wake_keys.wake_hint(),
            };
            render_sleep_card(&mut picture, state.display, &card)?;
        }
        Ok(picture)
    }

    /// Add a battery reading to the history, saving it in batches.
    fn record_battery_sample<I2C>(
        board_services: &mut BoardServices<I2C>,
        state: &mut AppState,
        mounted: bool,
    ) where
        I2C: embedded_hal::i2c::I2c,
        I2C::Error: core::fmt::Debug,
    {
        let (Ok(power), Ok(now)) = (board_services.read_power(), board_services.read_rtc()) else {
            return;
        };
        let Some(percent) = power.battery_percent else {
            return;
        };
        if state.battery_log.record(now.epoch_minutes(), percent)
            && mounted
            && state.battery_log.needs_save()
        {
            save_battery_log(&mut state.battery_log);
        }
    }

    fn save_battery_log(log: &mut BatteryLog) {
        match log.save_to_path(BATTERY_LOG_PATH) {
            Ok(()) => info!("rustmix-wave=battery-log status=saved path={BATTERY_LOG_PATH}"),
            Err(error) => warn!("rustmix-wave=battery-log status=save-failed error={error:#}"),
        }
    }

    /// Accrue the reading session into the stats and save when changed. The
    /// first save creates `/RUSTMIX/READER/` (the SD card has no folders).
    fn save_reading_stats_if_dirty(state: &mut AppState) {
        state.collect_reading_stats(state.local_day());
        if !state.reading_stats.has_unsaved() {
            return;
        }
        if let Some(parent) = std::path::Path::new(READING_STATS_PATH).parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        match state.reading_stats.save_to_path(READING_STATS_PATH) {
            Ok(()) => info!("rustmix-wave=reading-stats status=saved path={READING_STATS_PATH}"),
            Err(error) => warn!("rustmix-wave=reading-stats status=save-failed error={error:#}"),
        }
    }

    /// Weather sleep modes keep the weather updated while asleep.
    fn weather_while_asleep(state: &AppState) -> bool {
        state.sleep_screen.mode.shows_weather() && state.weather_enabled()
    }

    /// When the clock of a live sleep screen next changes; `None` for the
    /// weather layout, which redraws after each weather update.
    fn next_sleep_redraw(state: &AppState, layout: SleepLayout) -> Option<Instant> {
        if layout != SleepLayout::Clock {
            return None;
        }
        // Without a clock reading, look again in a minute.
        let wait = state.board.rtc.map_or(Duration::from_secs(60), |rtc| {
            let local = state.regional.localize_rtc(rtc);
            clock_redraw_wait(local, state.sleep_screen.clock_refresh)
        });
        Some(Instant::now() + wait)
    }

    /// When the next automatic weather update falls due; `None` when weather
    /// is missing, off or manual.
    fn next_weather_refresh(
        config: Option<&WeatherConfig>,
        last_attempt: Option<Instant>,
    ) -> Option<Instant> {
        let interval = config?.refresh_interval()?;
        Some(last_attempt.map_or_else(Instant::now, |last| last + interval))
    }

    /// Save a Settings › Weather or BOOT change to WEATHER.TXT and adopt it.
    fn sync_weather_config(state: &AppState, weather_config: &mut Option<WeatherConfig>) {
        if state.weather_config == *weather_config {
            return;
        }
        weather_config.clone_from(&state.weather_config);
        let Some(config) = weather_config.as_ref() else {
            return;
        };
        match config.save_to_path(WEATHER_CONFIG_PATH) {
            Ok(()) => info!("rustmix-wave=weather-config-write status=saved"),
            Err(error) => warn!("rustmix-wave=weather-config-write error={error:#}"),
        }
        info!(
            "rustmix-wave=weather-settings-updated enabled={} refresh-minutes={} units={} show-on-home={}",
            config.enabled,
            config.refresh_minutes,
            config.units.marker(),
            config.show_on_home
        );
    }

    /// Light-sleep until one of `lines` is pulled low, the Power key line goes
    /// high (when `power_key` is set) or `timer` runs out; the event loop then
    /// handles whatever woke it. Returns false without sleeping when a line is
    /// already active.
    fn light_sleep_until_wake(lines: &[i32], power_key: bool, timer: Duration) -> bool {
        let line_low = lines
            .iter()
            .any(|&gpio| unsafe { sys::gpio_get_level(gpio) } == 0);
        let key_high = power_key && unsafe { sys::gpio_get_level(POWER_KEY_GPIO) } != 0;
        if line_low || key_high {
            return false;
        }
        let result = unsafe {
            for &gpio in lines {
                sys::gpio_wakeup_enable(gpio, sys::gpio_int_type_t_GPIO_INTR_LOW_LEVEL);
            }
            if power_key {
                sys::gpio_wakeup_enable(POWER_KEY_GPIO, sys::gpio_int_type_t_GPIO_INTR_HIGH_LEVEL);
            }
            sys::esp_sleep_enable_gpio_wakeup();
            sys::esp_sleep_enable_timer_wakeup(timer.as_micros() as u64);
            let result = sys::esp_light_sleep_start();
            sys::esp_sleep_disable_wakeup_source(sys::esp_sleep_source_t_ESP_SLEEP_WAKEUP_ALL);
            for &gpio in lines {
                sys::gpio_wakeup_disable(gpio);
            }
            if power_key {
                sys::gpio_wakeup_disable(POWER_KEY_GPIO);
            }
            result
        };
        if let Err(error) = sys::EspError::convert(result) {
            warn!("rustmix-wave=light-sleep status=failed error={error}");
            return false;
        }
        true
    }

    /// Report the battery used asleep.
    fn end_sleep<I2C>(
        board_services: &mut BoardServices<I2C>,
        state: &mut AppState,
        sleep_started: &mut Option<(Instant, Option<PowerSnapshot>)>,
    ) where
        I2C: embedded_hal::i2c::I2c,
        I2C::Error: core::fmt::Debug,
    {
        let Some((started_at, before)) = sleep_started.take() else {
            return;
        };
        let after = board_services.read_power().ok();
        let report = SleepReport {
            seconds: started_at.elapsed().as_secs(),
            battery_before: before.and_then(|power| power.battery_percent),
            battery_after: after.and_then(|power| power.battery_percent),
        };
        info!(
            "rustmix-wave=sleep-energy seconds={} battery-percent={:?}->{:?} battery-mv={:?}->{:?}",
            report.seconds,
            report.battery_before,
            report.battery_after,
            before.and_then(|power| power.battery_voltage_mv),
            after.and_then(|power| power.battery_voltage_mv)
        );
        state.last_sleep = Some(report);
    }

    fn apply_calendar_ui_request(state: &mut AppState, mounted: bool) {
        let Some(request) = state.take_calendar_request() else {
            return;
        };
        if !mounted {
            state.calendar.fail("SD card unavailable");
            warn!(
                "rustmix-wave=calendar-personal-event-write status=rejected reason=sd-unavailable"
            );
            return;
        }
        let root = std::path::Path::new(CALENDAR_ROOT);
        let outcome = match request {
            CalendarUiRequest::CreatePersonal { date, title, detail } => {
                create_personal_event(root, date, &title, &detail).map(|()| {
                    info!("rustmix-wave=calendar-personal-event-write status=completed operation=create title={title}");
                    "Personal event created"
                })
            }
            CalendarUiRequest::UpdatePersonal {
                source_row,
                title,
                detail,
            } => update_personal_event(root, source_row, &title, &detail).map(|()| {
                info!("rustmix-wave=calendar-personal-event-write status=completed operation=edit source-row={source_row} title={title}");
                "Personal event updated"
            }),
            CalendarUiRequest::DeletePersonal { source_row } => {
                delete_personal_event(root, source_row).map(|()| {
                    info!("rustmix-wave=calendar-personal-event-write status=completed operation=delete source-row={source_row}");
                    "Personal event deleted"
                })
            }
        };
        match outcome {
            Ok(message) => {
                state.calendar.refresh_events();
                state.calendar.mark_persistence_completed(message);
                state.router.navigate_to(ScreenRoute::CalendarAgenda);
            }
            Err(error) => {
                state.calendar.fail(format!("{error:#}"));
                warn!("rustmix-wave=calendar-personal-event-write status=failed error={error:#}");
            }
        }
    }

    fn maintain_wifi_transfer_server(
        server: &mut Option<WifiTransferServer>,
        state: &mut AppState,
        storage_browser: &mut StorageBrowser,
        mounted: bool,
    ) {
        let stop_reason = server.as_ref().and_then(|active| {
            if state.network.wifi_state != WifiConnectionState::Connected {
                Some("wifi-loss")
            } else if active.is_expired() {
                Some("inactivity-timeout")
            } else {
                None
            }
        });
        if let Some(reason) = stop_reason {
            stop_wifi_transfer_server(server, state, storage_browser, mounted, reason);
        } else if let Some(active) = server.as_ref() {
            let snapshot = active.snapshot();
            if snapshot != state.wifi_transfer {
                state.update_wifi_transfer_snapshot(snapshot);
            }
        }
    }

    fn apply_wifi_transfer_ui_request(
        server: &mut Option<WifiTransferServer>,
        state: &mut AppState,
        storage_browser: &mut StorageBrowser,
        mounted: bool,
        voice_recording_active: bool,
        voice_playback_active: bool,
    ) {
        let Some(request) = state.take_wifi_transfer_request() else {
            return;
        };
        match request {
            WifiTransferUiRequest::Start => {
                info!(
                    "rustmix-wave=wifi-transfer-ui-request request=start dispatch=before-refresh"
                );
                if voice_recording_active {
                    state.update_wifi_transfer_snapshot(WifiTransferSnapshot::failed(
                        "Voice recording is active; stop recording before Wi-Fi transfer",
                    ));
                    warn!("rustmix-wave=wifi-transfer-server status=rejected reason=voice-recording-active");
                    return;
                }
                if voice_playback_active {
                    state.update_wifi_transfer_snapshot(WifiTransferSnapshot::failed(
                        "Voice-note playback is active; stop playback before Wi-Fi transfer",
                    ));
                    warn!("rustmix-wave=wifi-transfer-server status=rejected reason=voice-note-playback-active");
                    return;
                }
                if server.is_some() {
                    return;
                }
                state.update_wifi_transfer_snapshot(WifiTransferSnapshot::starting());
                if state.network.ipv4_address.is_none() {
                    // Wi-Fi is off between bursts; the main loop connects first.
                    info!("rustmix-wave=wifi-transfer-server status=waiting-for-wifi");
                    return;
                }
                start_wifi_transfer_server(server, state);
            }
            WifiTransferUiRequest::Stop => {
                info!("rustmix-wave=wifi-transfer-ui-request request=stop dispatch=before-refresh");
                stop_wifi_transfer_server(
                    server,
                    state,
                    storage_browser,
                    mounted,
                    "settings-toggle",
                );
            }
        }
    }

    fn start_wifi_transfer_server(server: &mut Option<WifiTransferServer>, state: &mut AppState) {
        let Some(ipv4) = state.network.ipv4_address.clone() else {
            state.update_wifi_transfer_snapshot(WifiTransferSnapshot::failed(
                "Connect Wi-Fi before starting transfer",
            ));
            warn!(
                "rustmix-wave=wifi-transfer-server status=start-rejected reason=wifi-not-connected"
            );
            return;
        };
        let code = format!("{:06}", unsafe { sys::esp_random() } % 1_000_000);
        info!("rustmix-wave=wifi-transfer-server status=starting ipv4={ipv4} port=80 root={WIFI_TRANSFER_ROOT} stack-bytes={WIFI_TRANSFER_SERVER_STACK_BYTES}");
        log_runtime_memory("before-wifi-transfer-start");
        match WifiTransferServer::start(&ipv4, code) {
            Ok(active) => {
                state.update_wifi_transfer_snapshot(active.snapshot());
                *server = Some(active);
                log_runtime_memory("after-wifi-transfer-start");
            }
            Err(error) => {
                warn!("rustmix-wave=wifi-transfer-server status=start-failed error={error:#}");
                state.update_wifi_transfer_snapshot(WifiTransferSnapshot::failed(format!(
                    "{error:#}"
                )));
            }
        }
    }

    fn stop_wifi_transfer_server(
        server: &mut Option<WifiTransferServer>,
        state: &mut AppState,
        storage_browser: &mut StorageBrowser,
        mounted: bool,
        reason: &'static str,
    ) {
        if server.take().is_some() {
            info!("rustmix-wave=wifi-transfer-server status=stopped reason={reason}");
            log_runtime_memory("after-wifi-transfer-stop");
        }
        state.update_wifi_transfer_snapshot(WifiTransferSnapshot::default());
        state.refresh_lua_app_catalog(mounted);
        state.reader.refresh_library();
        state.calendar.refresh_events();
        storage_browser.refresh();
        state.update_storage_snapshot(storage_browser.snapshot());
    }

    #[derive(Clone, Copy, Debug)]
    struct WeatherFetchAttempt {
        cause: &'static str,
        retry_attempt: usize,
    }

    impl WeatherFetchAttempt {
        const fn initial(cause: &'static str) -> Self {
            Self {
                cause,
                retry_attempt: 0,
            }
        }
    }

    #[derive(Clone, Copy, Debug)]
    struct PendingWeatherRetry {
        due: Instant,
        attempt: WeatherFetchAttempt,
    }

    #[derive(Clone, Copy, Debug, Default)]
    struct WeatherRetryState {
        pending: Option<PendingWeatherRetry>,
    }

    impl WeatherRetryState {
        fn clear(&mut self) {
            self.pending = None;
        }

        #[must_use]
        const fn is_pending(&self) -> bool {
            self.pending.is_some()
        }

        fn take_due(&mut self) -> Option<WeatherFetchAttempt> {
            if self
                .pending
                .as_ref()
                .is_some_and(|pending| Instant::now() >= pending.due)
            {
                self.pending.take().map(|pending| pending.attempt)
            } else {
                None
            }
        }

        fn schedule_next(&mut self, failed_attempt: WeatherFetchAttempt) -> Option<(usize, u64)> {
            let retry_attempt = failed_attempt.retry_attempt.saturating_add(1);
            let delay_seconds = *WEATHER_RETRY_DELAYS_SECONDS.get(retry_attempt - 1)?;
            self.pending = Some(PendingWeatherRetry {
                due: Instant::now() + Duration::from_secs(delay_seconds),
                attempt: WeatherFetchAttempt {
                    cause: failed_attempt.cause,
                    retry_attempt,
                },
            });
            Some((retry_attempt, delay_seconds))
        }
    }

    fn run_weather_fetch_attempt(
        config: &WeatherConfig,
        attempt: WeatherFetchAttempt,
        retry: &mut WeatherRetryState,
        state: &mut AppState,
    ) {
        let attempt_label = if attempt.retry_attempt == 0 {
            "initial".into()
        } else {
            format!("{}/{}", attempt.retry_attempt, WEATHER_RETRY_LIMIT)
        };
        info!(
            "rustmix-wave=weather-fetch status=starting cause={} attempt={} provider={} location={}",
            attempt.cause, attempt_label, config.provider, config.location
        );
        state.weather.mark_fetching();
        match fetch_open_meteo_on_worker(config) {
            Ok(data) => {
                retry.clear();
                state.weather.record_success(data);
                log_weather_snapshot(&state.weather);
                info!(
                    "rustmix-wave=weather-fetch status=completed cause={} attempt={} forecast-days={}",
                    attempt.cause,
                    attempt_label,
                    state.weather.forecast.len()
                );
            }
            Err(error) => {
                handle_weather_fetch_failure(attempt, error, retry, state);
                log_weather_snapshot(&state.weather);
            }
        }
    }

    fn handle_weather_fetch_failure(
        attempt: WeatherFetchAttempt,
        error: WeatherFetchError,
        retry: &mut WeatherRetryState,
        state: &mut AppState,
    ) {
        let message = error.to_string();
        if error.is_retryable() {
            if let Some((retry_attempt, delay_seconds)) = retry.schedule_next(attempt) {
                state.weather.mark_retrying(message.clone());
                warn!(
                    "rustmix-wave=weather-fetch-retry-scheduled cause={} attempt={}/{} delay-seconds={} classification={} error={}",
                    attempt.cause,
                    retry_attempt,
                    WEATHER_RETRY_LIMIT,
                    delay_seconds,
                    error.category(),
                    message
                );
                if state.weather.current.is_some() {
                    info!(
                        "rustmix-wave=weather-fetch outcome=stale-cache-retained state=retrying last-success={} error={}",
                        state.weather.last_success_label(),
                        message
                    );
                }
                return;
            }
        }

        retry.clear();
        state.weather.record_failure(message.clone());
        warn!(
            "rustmix-wave=weather-fetch status=failed cause={} retryable={} retries-exhausted={} classification={} error={}",
            attempt.cause,
            error.is_retryable(),
            error.is_retryable(),
            error.category(),
            message
        );
        if state.weather.current.is_some() {
            info!(
                "rustmix-wave=weather-fetch outcome=stale-cache-retained state=stale last-success={} error={}",
                state.weather.last_success_label(),
                message
            );
        }
    }

    /// Switch the Wi-Fi radio off; false when the driver refused.
    fn radio_off(
        runtime: &mut NetworkRuntime,
        state: &mut AppState,
        last_network_fingerprint: &mut NetworkLogFingerprint,
        last_network_log: &mut Instant,
        reason: &str,
    ) -> bool {
        match runtime.suspend() {
            Ok(()) => {
                state.update_network_snapshot(runtime.snapshot());
                *last_network_fingerprint = state.network.log_fingerprint();
                *last_network_log = Instant::now();
                info!("rustmix-wave=wifi-burst status=off reason={reason}");
                true
            }
            Err(error) => {
                warn!("rustmix-wave=wifi-burst status=off-failed reason={reason} error={error:#}");
                false
            }
        }
    }

    fn apply_alarm_event(
        engine: &mut AlarmEngine,
        state: &mut AppState,
        event: ButtonEvent,
        now_local: RtcDateTime,
    ) -> AlarmUiOutcome {
        if event == ButtonEvent::Select {
            state.note_select_press();
        }
        let outcome = engine.apply_button(event, now_local);
        if outcome == AlarmUiOutcome::ReturnHome {
            state.router.back();
        }
        state.update_alarm_snapshot(engine.snapshot());
        info!(
            "rustmix-wave=alarm-ui-event outcome={outcome:?} active={} schedules={} selected={} next={} hardware-programmed={}",
            state.alarms.active.is_some(),
            state.alarms.alarms.len(),
            state.alarms.selected,
            state.alarms.next_label(),
            state.alarms.hardware_programmed
        );
        outcome
    }

    fn apply_audio_request<'d, I2C>(
        runtime: &mut Option<AudioRuntime<'d, I2C>>,
        state: &mut AppState,
        request: AudioUiRequest,
    ) where
        I2C: embedded_hal::i2c::I2c,
        I2C::Error: core::fmt::Debug,
    {
        let Some(runtime) = runtime.as_mut() else {
            warn!("rustmix-wave=audio-event outcome=unavailable request={request:?}");
            return;
        };
        match runtime.apply_request(request) {
            Ok(outcome) => info!("rustmix-wave=audio-event outcome={outcome}"),
            Err(error) => {
                warn!("rustmix-wave=audio-event outcome=request-failed request={request:?} error={error:#}");
                runtime.record_failure(format!("{error:#}"));
            }
        }
        state.update_audio_snapshot(runtime.snapshot());
        log_audio_snapshot(&state.audio);
    }

    fn sd_available_bytes(path: &str) -> Option<u64> {
        let path = CString::new(path).ok()?;
        let mut total_bytes = 0_u64;
        let mut free_bytes = 0_u64;
        if unsafe { sys::esp_vfs_fat_info(path.as_ptr(), &mut total_bytes, &mut free_bytes) }
            != sys::ESP_OK
        {
            return None;
        }
        Some(free_bytes)
    }

    fn refresh_voice_note_storage_available(state: &mut AppState, mounted: bool) {
        let available = mounted
            .then(|| sd_available_bytes(SD_MOUNT_POINT))
            .flatten();
        state.voice_notes.set_available_storage_bytes(available);
    }

    fn stop_voice_note_playback<'d, I2C>(
        session: &mut Option<VoicePlaybackSession>,
        audio_runtime: &mut Option<AudioRuntime<'d, I2C>>,
        state: &mut AppState,
        reason: &str,
    ) where
        I2C: embedded_hal::i2c::I2c,
        I2C::Error: core::fmt::Debug,
    {
        let Some(active) = session.take() else {
            return;
        };
        let file_name = active.file_name().to_string();
        if let Some(runtime) = audio_runtime.as_mut() {
            if let Err(error) = runtime.finish_voice_note_playback() {
                warn!("rustmix-wave=voice-note-playback status=stop-failed file={file_name} reason={reason} error={error:#}");
                runtime.record_failure(format!("{error:#}"));
            }
            state.update_audio_snapshot(runtime.snapshot());
            log_audio_snapshot(&state.audio);
        }
        state.voice_notes.stop_playback();
        info!("rustmix-wave=voice-note-playback status=stopped file={file_name} reason={reason}");
    }

    fn apply_voice_notes_ui_request<'d, I2C>(
        session: &mut Option<VoiceRecordingSession>,
        playback: &mut Option<VoicePlaybackSession>,
        audio_runtime: &mut Option<AudioRuntime<'d, I2C>>,
        state: &mut AppState,
        mounted: bool,
    ) where
        I2C: embedded_hal::i2c::I2c,
        I2C::Error: core::fmt::Debug,
    {
        let Some(request) = state.take_voice_notes_request() else {
            return;
        };
        match request {
            VoiceNotesUiRequest::StartRecording => {
                if !mounted {
                    state.voice_notes.fail("SD card unavailable");
                    warn!("rustmix-wave=voice-record status=rejected reason=sd-unavailable");
                    return;
                }
                if state.wifi_transfer.is_active() {
                    state
                        .voice_notes
                        .fail("Stop Wi-Fi Transfer before recording");
                    warn!("rustmix-wave=voice-record status=rejected reason=wifi-transfer-active");
                    return;
                }
                if state.alarms.active.is_some() {
                    state.voice_notes.fail("Alarm active");
                    warn!("rustmix-wave=voice-record status=rejected reason=active-alarm");
                    return;
                }
                if playback.is_some() {
                    stop_voice_note_playback(playback, audio_runtime, state, "recording-start");
                }
                let Some(runtime) = audio_runtime.as_mut() else {
                    state.voice_notes.fail("Microphone unavailable");
                    warn!("rustmix-wave=voice-record status=rejected reason=audio-unavailable");
                    return;
                };
                if session.is_some() {
                    return;
                }
                let recorded_at = state
                    .board
                    .rtc
                    .map(|rtc| state.regional.localize_rtc(rtc).date_time())
                    .unwrap_or_else(|| VOICE_UNKNOWN_RECORDED_AT.into());
                log_runtime_memory("before-voice-record");
                match VoiceRecordingSession::start_with_recorded_at(
                    std::path::Path::new(VOICE_NOTES_ROOT),
                    recorded_at.clone(),
                ) {
                    Ok(created) => {
                        if let Err(error) = runtime.begin_voice_recording() {
                            let _ = created.cancel();
                            state.voice_notes.fail(format!("{error:#}"));
                            warn!("rustmix-wave=voice-record status=failed stage=audio-start error={error:#}");
                            return;
                        }
                        let file_name = created.file_name().to_string();
                        state
                            .voice_notes
                            .begin_recording(file_name.clone(), recorded_at.clone());
                        *session = Some(created);
                        log_runtime_memory("after-voice-record-start");
                        info!("rustmix-wave=voice-record status=starting file={} recorded-at={} sample-rate=16000 bits=16 channels=1 chunk-bytes={} capture=cooperative-bounded-i2s-rx mic-gain={}", file_name, recorded_at, VOICE_PCM_MONO_CHUNK_BYTES, state.voice_notes.mic_gain.marker());
                    }
                    Err(error) => {
                        state.voice_notes.fail(format!("{error:#}"));
                        warn!("rustmix-wave=voice-record status=failed stage=storage-start error={error:#}");
                    }
                }
            }
            VoiceNotesUiRequest::StopRecording => {
                let Some(active) = session.take() else {
                    return;
                };
                match active.finalize() {
                    Ok(entry) => {
                        if let Some(runtime) = audio_runtime.as_mut() {
                            let _ = runtime.finish_voice_recording();
                            state.update_audio_snapshot(runtime.snapshot());
                        }
                        info!("rustmix-wave=voice-record status=completed file={} recorded-at={} duration-seconds={} pcm-bytes={} wav-bytes={}", entry.file_name, entry.recorded_at, entry.duration_seconds, entry.pcm_bytes, entry.wav_bytes);
                        state.voice_notes.complete_recording(entry);
                        state.refresh_voice_notes_catalog();
                        refresh_voice_note_storage_available(state, mounted);
                        log_runtime_memory("after-voice-record-stop");
                    }
                    Err(error) => {
                        if let Some(runtime) = audio_runtime.as_mut() {
                            let _ = runtime.finish_voice_recording();
                            state.update_audio_snapshot(runtime.snapshot());
                        }
                        state.voice_notes.fail(format!("{error:#}"));
                        warn!("rustmix-wave=voice-record status=failed stage=finalize error={error:#}");
                        log_runtime_memory("after-voice-record-stop");
                    }
                }
            }
            VoiceNotesUiRequest::PauseRecording => {
                if session.is_some() {
                    state.voice_notes.pause_recording();
                    info!("rustmix-wave=voice-record status=paused");
                }
            }
            VoiceNotesUiRequest::ResumeRecording => {
                if session.is_some() {
                    state.voice_notes.resume_recording();
                    info!("rustmix-wave=voice-record status=resumed");
                }
            }
            VoiceNotesUiRequest::CancelRecording => {
                if let Some(active) = session.take() {
                    let _ = active.cancel();
                }
                if let Some(runtime) = audio_runtime.as_mut() {
                    let _ = runtime.finish_voice_recording();
                    state.update_audio_snapshot(runtime.snapshot());
                }
                state.voice_notes.cancel_recording();
                refresh_voice_note_storage_available(state, mounted);
                info!("rustmix-wave=voice-record status=cancelled");
            }
            VoiceNotesUiRequest::StartPlayback => {
                if !mounted {
                    state.voice_notes.fail("SD card unavailable");
                    warn!("rustmix-wave=voice-note-playback status=rejected reason=sd-unavailable");
                    return;
                }
                if session.is_some() {
                    state.voice_notes.fail("Stop recording before playback");
                    warn!("rustmix-wave=voice-note-playback status=rejected reason=voice-recording-active");
                    return;
                }
                if state.wifi_transfer.is_active() {
                    state
                        .voice_notes
                        .fail("Stop Wi-Fi Transfer before playback");
                    warn!("rustmix-wave=voice-note-playback status=rejected reason=wifi-transfer-active");
                    return;
                }
                if state.alarms.active.is_some() {
                    state.voice_notes.fail("Alarm active");
                    warn!("rustmix-wave=voice-note-playback status=rejected reason=active-alarm");
                    return;
                }
                let Some(file_name) = state
                    .voice_notes
                    .selected_note()
                    .map(|note| note.file_name.clone())
                else {
                    state.voice_notes.fail("No voice note selected");
                    warn!("rustmix-wave=voice-note-playback status=rejected reason=no-selection");
                    return;
                };
                if audio_runtime.is_none() {
                    state.voice_notes.fail("Speaker unavailable");
                    warn!(
                        "rustmix-wave=voice-note-playback status=rejected reason=audio-unavailable"
                    );
                    return;
                }
                if playback.is_some() {
                    stop_voice_note_playback(playback, audio_runtime, state, "replace-selection");
                }
                match VoicePlaybackSession::open(std::path::Path::new(VOICE_NOTES_ROOT), &file_name)
                {
                    Ok(created) => {
                        let total_pcm_bytes = created.total_pcm_bytes();
                        let runtime = audio_runtime
                            .as_mut()
                            .expect("audio runtime checked before playback start");
                        if let Err(error) = runtime.begin_voice_note_playback() {
                            runtime.record_failure(format!(
                                "Voice-note playback start failed: {error:#}"
                            ));
                            state.update_audio_snapshot(runtime.snapshot());
                            state.voice_notes.fail(format!("{error:#}"));
                            warn!("rustmix-wave=voice-note-playback status=failed stage=audio-start file={file_name} error={error:#}");
                            return;
                        }
                        state
                            .voice_notes
                            .begin_playback(file_name.clone(), total_pcm_bytes);
                        *playback = Some(created);
                        state.update_audio_snapshot(runtime.snapshot());
                        log_audio_snapshot(&state.audio);
                        info!("rustmix-wave=voice-note-playback status=starting file={file_name} pcm-bytes={total_pcm_bytes} sample-rate=16000 bits=16 source-channels=1 output-channels=2 chunk-bytes={VOICE_PCM_MONO_CHUNK_BYTES} volume={}", state.audio.volume_percent);
                    }
                    Err(error) => {
                        state.voice_notes.fail(format!("{error:#}"));
                        warn!("rustmix-wave=voice-note-playback status=failed stage=storage-open file={file_name} error={error:#}");
                    }
                }
            }
            VoiceNotesUiRequest::StopPlayback => {
                stop_voice_note_playback(playback, audio_runtime, state, "ui-stop");
            }
            VoiceNotesUiRequest::PersistMicGain(mic_gain) => {
                let preferences = VoiceNotesPreferences { mic_gain };
                match save_voice_notes_preferences(std::path::Path::new(VOICE_NOTES_ROOT), preferences) {
                    Ok(()) => info!("rustmix-wave=voice-note-settings-write status=completed mic-gain={} path={VOICE_NOTES_ROOT}/SETTINGS.TXT", mic_gain.marker()),
                    Err(error) => {
                        state.voice_notes.fail(format!("{error:#}"));
                        warn!("rustmix-wave=voice-note-settings-write status=failed mic-gain={} error={error:#}", mic_gain.marker());
                    }
                }
            }
            VoiceNotesUiRequest::SaveEditedTitle { file_name, title } => {
                match save_voice_note_title(
                    std::path::Path::new(VOICE_NOTES_ROOT),
                    &file_name,
                    &title,
                ) {
                    Ok(()) => {
                        state.refresh_voice_notes_catalog();
                        info!("rustmix-wave=voice-note-title-write status=completed file={file_name} title={title}");
                    }
                    Err(error) => {
                        state.voice_notes.fail(format!("{error:#}"));
                        warn!("rustmix-wave=voice-note-title-write status=failed file={file_name} error={error:#}");
                    }
                }
            }
            VoiceNotesUiRequest::ExportSelected => {
                if !mounted {
                    state.voice_notes.fail("SD card unavailable");
                    warn!("rustmix-wave=voice-note-export status=rejected reason=sd-unavailable");
                    return;
                }
                if session.is_some() {
                    state.voice_notes.fail("Stop recording before export");
                    warn!("rustmix-wave=voice-note-export status=rejected reason=voice-recording-active");
                    return;
                }
                if playback.is_some() {
                    stop_voice_note_playback(playback, audio_runtime, state, "export-note");
                }
                let Some(file_name) = state
                    .voice_notes
                    .selected_note()
                    .map(|note| note.file_name.clone())
                else {
                    state.voice_notes.fail("No voice note selected");
                    return;
                };
                state.voice_notes.mark_export_requested(file_name.clone());
                state.request_wifi_transfer_start();
                info!("rustmix-wave=voice-note-export status=requested file={file_name} portal-path=VOICE/{file_name}");
            }
            VoiceNotesUiRequest::DeleteSelected => {
                if playback.is_some() {
                    stop_voice_note_playback(playback, audio_runtime, state, "delete-note");
                }
                let selected = state
                    .voice_notes
                    .selected_note()
                    .map(|note| note.file_name.clone());
                if let Some(file_name) = selected {
                    match delete_voice_note(std::path::Path::new(VOICE_NOTES_ROOT), &file_name) {
                        Ok(()) => {
                            state.voice_notes.remove_selected_note();
                            state.refresh_voice_notes_catalog();
                            refresh_voice_note_storage_available(state, mounted);
                            state.router.navigate_to(ScreenRoute::VoiceNotes);
                            info!(
                                "rustmix-wave=voice-note-delete status=completed file={file_name} confirmation=accepted"
                            );
                        }
                        Err(error) => {
                            state.voice_notes.fail(format!("{error:#}"));
                            warn!("rustmix-wave=voice-note-delete status=failed file={file_name} error={error:#}");
                        }
                    }
                }
            }
            VoiceNotesUiRequest::RefreshCatalog => {
                state.refresh_voice_notes_catalog();
                refresh_voice_note_storage_available(state, mounted);
            }
        }
    }

    fn sync_alarm_hardware<I2C>(
        engine: &mut AlarmEngine,
        board_services: &mut BoardServices<I2C>,
        regional: RegionalPreferences,
    ) where
        I2C: embedded_hal::i2c::I2c,
        I2C::Error: core::fmt::Debug,
    {
        if let Some(next) = engine.next_occurrence() {
            let stored = regional.local_to_rtc(next.local);
            match board_services.program_rtc_alarm(stored) {
                Ok(()) => {
                    engine.set_hardware_programmed(true);
                    info!(
                        "rustmix-wave=rtc-alarm-program status=armed local={} stored={} snooze={}",
                        next.local.date_time(),
                        stored.date_time(),
                        next.snooze
                    );
                }
                Err(error) => {
                    engine.set_hardware_programmed(false);
                    warn!("rustmix-wave=rtc-alarm-program status=failed error={error:#}");
                }
            }
        } else {
            match board_services.disable_rtc_alarm() {
                Ok(()) => {
                    engine.set_hardware_programmed(false);
                    info!("rustmix-wave=rtc-alarm-program status=idle");
                }
                Err(error) => warn!("rustmix-wave=rtc-alarm-disable status=failed error={error:#}"),
            }
        }
    }

    fn fallback_local_time() -> RtcDateTime {
        RtcDateTime {
            year: 2000,
            month: 1,
            day: 1,
            weekday: 6,
            hour: 0,
            minute: 0,
            second: 0,
        }
    }

    fn apply_storage_event(browser: &mut StorageBrowser, state: &mut AppState, event: ButtonEvent) {
        if event == ButtonEvent::Select {
            state.note_select_press();
        }
        let outcome = browser.apply_button(event);
        if outcome == StorageUiOutcome::ReturnHome {
            state.router.back();
        }
        state.update_storage_snapshot(browser.snapshot());
        info!(
            "rustmix-wave=storage-browser-event outcome={outcome:?} path={} entries={} retained-entries={} raw-entries={} selected={} preview={}",
            state.storage.current_path,
            state.storage.entries.len(),
            state.storage.scan.retained_entries,
            state.storage.scan.raw_entries,
            state.storage.selected,
            state.storage.preview.is_some()
        );
    }

    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    enum RefreshRequest {
        Normal,
        ForceGlobalAfterWake,
        ForceGlobalManual,
        #[allow(dead_code)]
        ForceGlobalSafetyFallback,
    }

    fn refresh_screen<SPI, DC, RST, CS, BUSY, DELAY, POWER>(
        panel: &mut Epaper397<SPI, DC, RST, CS, BUSY, DELAY, POWER>,
        frame: &mut FrameBuffer,
        state: &mut AppState,
        coordinator: &mut PanelRefreshCoordinator,
        request: RefreshRequest,
    ) -> Result<()>
    where
        SPI: embedded_hal::spi::SpiBus<u8>,
        SPI::Error: core::fmt::Debug,
        DC: embedded_hal::digital::OutputPin,
        DC::Error: core::fmt::Debug,
        RST: embedded_hal::digital::OutputPin,
        RST::Error: core::fmt::Debug,
        CS: embedded_hal::digital::OutputPin,
        CS::Error: core::fmt::Debug,
        BUSY: embedded_hal::digital::InputPin,
        BUSY::Error: core::fmt::Debug,
        DELAY: DelayNs,
        POWER: waveshare_epd397_rust_app::power::PanelPower,
    {
        let coordinator_request = match request {
            RefreshRequest::Normal => PanelRefreshRequest::Normal,
            RefreshRequest::ForceGlobalAfterWake => PanelRefreshRequest::AfterWake,
            RefreshRequest::ForceGlobalManual => PanelRefreshRequest::ManualGhostCleanup,
            RefreshRequest::ForceGlobalSafetyFallback => PanelRefreshRequest::SafetyFallback,
        };
        let plan = coordinator.plan(coordinator_request);
        sync_panel_refresh_diagnostics(state, coordinator);
        render_current_screen(frame, state)?;

        match plan {
            PanelRefreshPlan::GlobalBase { reason } => {
                panel.show_base(frame.as_bytes())?;
                info!(
                    "rustmix-wave=panel-refresh plan=global-base reason={} transport=global-base",
                    reason.marker()
                );
                match reason {
                    PanelGlobalReason::AfterWake => info!("rustmix-wave=wake-global-refresh"),
                    PanelGlobalReason::ManualGhostCleanup => {
                        info!("rustmix-wave=reader-clear-ghosting refresh=global-base");
                        info!("rustmix-wave=power-key-clear-ghosting refresh=global-base")
                    }
                    PanelGlobalReason::PeriodicCleanup => {
                        info!("rustmix-wave=global-refresh-after-partials")
                    }
                    PanelGlobalReason::SafetyFallback => {
                        warn!("rustmix-wave=panel-refresh safety-fallback refresh=global-base")
                    }
                    PanelGlobalReason::InitialBoot | PanelGlobalReason::SleepImage => {}
                }
            }
            PanelRefreshPlan::PartialFullscreen { partial_count } => {
                panel.show_partial_fullscreen(frame.as_bytes())?;
                info!(
                    "rustmix-wave=panel-refresh plan=partial-fullscreen reason=normal partial-count={partial_count} partial-limit={PANEL_PARTIAL_REFRESH_LIMIT} transport=existing-fullscreen-partial"
                );
            }
        }
        Ok(())
    }

    fn sync_panel_refresh_diagnostics(state: &mut AppState, coordinator: &PanelRefreshCoordinator) {
        state.partial_refreshes = coordinator.partial_count();
    }

    fn log_lua_runtime_events(state: &mut AppState) {
        for line in state.take_lua_runtime_diagnostics() {
            info!("{line}");
        }
    }

    fn log_reader_persistence_event(state: &mut AppState) {
        if let Some(event) = state.reader.take_persistence_event() {
            info!("rustmix-wave=reader-persistence {event}");
        }
    }

    fn log_sleep_image_selection(selection: &SleepImageSelection) {
        let error = selection.scan_error.as_deref().unwrap_or("none");
        if selection.fallback {
            warn!(
                "rustmix-wave=sleep-image-fallback source=sleep-card path={SLEEP_IMAGE_DIRECTORY} raw={} candidates={} metadata-fallbacks={} ignored={} valid={} rejected={} error={} note={}",
                selection.raw_entries,
                selection.candidate_entries,
                selection.metadata_fallbacks,
                selection.ignored_entries,
                selection.valid_count,
                selection.rejected_count,
                error,
                selection.note.as_deref().unwrap_or("none")
            );
        } else {
            info!(
                "rustmix-wave=sleep-image-scan status=ready path={SLEEP_IMAGE_DIRECTORY} raw={} candidates={} metadata-fallbacks={} ignored={} valid={} rejected={} error={}",
                selection.raw_entries,
                selection.candidate_entries,
                selection.metadata_fallbacks,
                selection.ignored_entries,
                selection.valid_count,
                selection.rejected_count,
                error
            );
            info!(
                "rustmix-wave=sleep-image-selected file={}",
                selection.file_name
            );
            if let Some(choice) = selection.choice {
                info!(
                    "rustmix-wave=sleep-image-choice mode=hardware-random candidates={} random-word=0x{:08X} previous-index={} selected-index={} anti-repeat={}",
                    selection.valid_count,
                    choice.random_word,
                    choice
                        .previous_index
                        .map_or_else(|| "none".into(), |index| index.to_string()),
                    choice.selected_index,
                    choice.anti_repeat
                );
            }
        }
    }

    fn log_board_snapshot(snapshot: BoardSnapshot, regional: RegionalPreferences) {
        let imu = snapshot.imu.map_or_else(
            || "unavailable".into(),
            |reading| {
                format!(
                    "motion={}mg axis={} acc=[{}] gyro=[{}]",
                    reading.motion_magnitude_mg,
                    reading.dominant_axis.label(),
                    reading.acceleration_mg_tenths.compact_label(),
                    reading.gyroscope_dps_tenths.compact_label()
                )
            },
        );
        info!(
            "rustmix-wave=sample-board-snapshot time={} timezone={} battery={} temperature={} humidity={} imu={}",
            snapshot.time_label(regional),
            regional.timezone_label_for_rtc(snapshot.rtc),
            snapshot.battery_label(),
            snapshot.temperature_label(regional.temperature_unit),
            snapshot.humidity_label(),
            imu
        );
    }

    fn log_storage_snapshot(snapshot: &StorageSnapshot) {
        info!(
            "rustmix-wave=storage-browser-snapshot mounted={} path={} entries={} retained-entries={} raw-entries={} metadata-fallbacks={} ignored-special={} selected={} preview={} error={}",
            snapshot.mounted,
            snapshot.current_path,
            snapshot.entries.len(),
            snapshot.scan.retained_entries,
            snapshot.scan.raw_entries,
            snapshot.scan.metadata_fallbacks,
            snapshot.scan.ignored_special,
            snapshot.selected,
            snapshot.preview.is_some(),
            snapshot.error.as_deref().unwrap_or("none")
        );
    }

    fn log_network_snapshot(snapshot: &NetworkSnapshot) {
        info!(
            "rustmix-wave=network-snapshot wifi={} ntp={} ssid={} ipv4={} rssi={} timezone={} ntp-server={} last-sync={} error={}",
            snapshot.wifi_state.label(),
            snapshot.ntp_state.label(),
            snapshot.ssid_label(),
            snapshot.ipv4_label(),
            snapshot.rssi_label(),
            snapshot.timezone_name,
            snapshot.ntp_server,
            snapshot.last_sync_label(),
            snapshot.error.as_deref().unwrap_or("none")
        );
    }

    fn log_audio_snapshot(snapshot: &AudioSnapshot) {
        info!(
            "rustmix-wave=audio-snapshot available={} codec-address={} codec-ready={} i2s-ready={} amp={} mute={} volume={} state={} error={}",
            snapshot.available,
            snapshot.codec_address_label(),
            snapshot.codec_ready,
            snapshot.i2s_ready,
            snapshot.amplifier_enabled,
            snapshot.muted,
            snapshot.volume_percent,
            snapshot.playback_state.label(),
            snapshot.error.as_deref().unwrap_or("none")
        );
    }

    fn log_alarm_snapshot(snapshot: &AlarmSnapshot) {
        info!(
            "rustmix-wave=alarm-snapshot schedules={} active={} selected={} next={} snooze-minutes={} hardware-programmed={} error={}",
            snapshot.alarms.len(),
            snapshot.active.as_ref().map_or("none", |active| active.name.as_str()),
            snapshot.selected,
            snapshot.next_label(),
            snapshot.snooze_minutes,
            snapshot.hardware_programmed,
            snapshot.error.as_deref().unwrap_or("none")
        );
    }

    fn log_weather_snapshot(snapshot: &WeatherSnapshot) {
        info!(
            "rustmix-wave=weather-snapshot state={} provider={} location={} timezone={} current={} forecast-days={} last-success={} error={}",
            snapshot.state.label(),
            snapshot.provider,
            snapshot.location,
            snapshot.provider_timezone,
            snapshot.current_summary(),
            snapshot.forecast.len(),
            snapshot.last_success_label(),
            snapshot.error.as_deref().unwrap_or("none")
        );
    }

    /// FreeRTOS-backed delays are sufficient for the millisecond timings used
    /// by the panel and sample-board reference sequences. Round sub-millisecond
    /// requests up so short sensor waits remain conservative.
    #[derive(Clone, Copy, Debug, Default)]
    struct FreeRtosDelay;

    impl DelayNs for FreeRtosDelay {
        fn delay_ns(&mut self, nanoseconds: u32) {
            let milliseconds = nanoseconds.saturating_add(999_999) / 1_000_000;
            if milliseconds > 0 {
                FreeRtos::delay_ms(milliseconds);
            }
        }
    }
}

#[cfg(target_os = "espidf")]
fn main() -> anyhow::Result<()> {
    firmware::run()
}

#[cfg(not(target_os = "espidf"))]
fn main() {
    println!("Build this firmware for xtensa-esp32s3-espidf. See README.md.");
}
