//! The state of `Settings › System › Check for updates`: what the screen
//! shows, what its keys do, and the requests it makes of the update worker.
//! No drawing, no network and no clock: the main loop passes the power state
//! in and applies the worker's events.

use crate::ota::{self, may_start, Refusal, Release, UpdateCheck, Version};

/// The four steps the screen's chips name.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Step {
    Check,
    Download,
    Verify,
    Restart,
}

impl Step {
    pub const ALL: [Self; 4] = [Self::Check, Self::Download, Self::Verify, Self::Restart];

    /// `1 · CHECK`, as the mockup's chips.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Check => "1 \u{b7} CHECK",
            Self::Download => "2 \u{b7} DOWNLOAD",
            Self::Verify => "3 \u{b7} VERIFY",
            Self::Restart => "4 \u{b7} RESTART",
        }
    }
}

/// Why an update did not happen. Every one leaves the installed version as
/// it was.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Failure {
    LowBattery { percent: u8 },
    BatteryUnknown,
    NoWifi,
    Unreachable,
    Stopped,
    WrongImage(String),
    NoSlot,
    Busy,
    Other(String),
}

impl Failure {
    /// The headline of the screen.
    #[must_use]
    pub fn title(&self) -> String {
        match self {
            Self::LowBattery { percent } => format!("Battery {percent}%"),
            Self::BatteryUnknown => "Battery level unknown".into(),
            Self::NoWifi => "No Wi-Fi".into(),
            Self::Unreachable => "Cannot reach GitHub".into(),
            Self::Stopped => "Download stopped".into(),
            Self::WrongImage(_) => "Wrong image".into(),
            Self::NoSlot => "No update slot".into(),
            Self::Busy => "Busy".into(),
            Self::Other(_) => "Update failed".into(),
        }
    }

    /// The line under it.
    #[must_use]
    pub fn detail(&self) -> String {
        match self {
            Self::LowBattery { .. } => format!(
                "Charge to {}% or connect USB to update.",
                ota::MIN_BATTERY_PERCENT
            ),
            Self::BatteryUnknown => "Connect USB to update.".into(),
            Self::NoWifi => "Set it in Wi-Fi & transfer.".into(),
            Self::Unreachable => "Try again later.".into(),
            Self::Stopped => "The current version is untouched.".into(),
            Self::WrongImage(why) => format!("Not written: {why}"),
            Self::NoSlot => "Flash the full image by USB once (v0.13.0).".into(),
            Self::Busy => "Finish the recording, playback or transfer first.".into(),
            Self::Other(why) => why.clone(),
        }
    }
}

impl From<Refusal> for Failure {
    fn from(refusal: Refusal) -> Self {
        match refusal {
            Refusal::LowBattery { percent } => Self::LowBattery { percent },
            Refusal::BatteryUnknown => Self::BatteryUnknown,
        }
    }
}

/// What the worker reports.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum UpdateEvent {
    /// The check ended: the latest release, or why not.
    Checked(Result<Release, Failure>),
    /// A step of the install, with bytes done and expected (0 when unknown).
    Progress {
        step: Step,
        done: u64,
        total: u64,
    },
    /// The new image is written and verified; the next start uses it.
    Installed,
    Failed(Failure),
}

/// What the screen asks of the main loop.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum UpdateRequest {
    Check,
    Install(Release),
    Cancel,
    Restart,
}

/// A newer release the user can install.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Found {
    pub release: Release,
    pub version: Version,
    pub size: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum UpdatePhase {
    Checking,
    Available(Found),
    UpToDate,
    Working {
        step: Step,
        done: u64,
        total: u64,
    },
    /// Written and verified; waiting for a restart.
    Installed(Version),
    Failed(Failure),
}

/// What a key asks the caller to do beyond the state's own change.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Outcome {
    Stay,
    Leave,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UpdateUi {
    installed: Version,
    phase: UpdatePhase,
    row: usize,
    request: Option<UpdateRequest>,
    /// `12:04`, when the last check ended, if the clock was set.
    checked_at: Option<String>,
    /// The version being installed, for the `Installed` screen.
    target: Option<Version>,
    /// The ten-percent bucket last drawn, so progress redraws are rare.
    drawn_bucket: Option<u8>,
}

impl UpdateUi {
    #[must_use]
    pub fn new(installed: Version) -> Self {
        Self {
            installed,
            phase: UpdatePhase::UpToDate,
            row: 0,
            request: None,
            checked_at: None,
            target: None,
            drawn_bucket: None,
        }
    }

    #[must_use]
    pub const fn installed(&self) -> Version {
        self.installed
    }

    #[must_use]
    pub const fn phase(&self) -> &UpdatePhase {
        &self.phase
    }

    #[must_use]
    pub const fn row(&self) -> usize {
        self.row
    }

    /// The version an install is writing, or just wrote.
    #[must_use]
    pub const fn target(&self) -> Option<Version> {
        self.target
    }

    #[must_use]
    pub fn checked_at(&self) -> Option<&str> {
        self.checked_at.as_deref()
    }

    /// The labels of the rows under the picture, in order.
    #[must_use]
    pub const fn rows(&self) -> &'static [&'static str] {
        match self.phase {
            UpdatePhase::Available(_) => &["Download and install", "Check again", "Back"],
            UpdatePhase::UpToDate => &["Check again", "Back"],
            UpdatePhase::Installed(_) => &["Restart now", "Later"],
            UpdatePhase::Failed(_) => &["Back"],
            UpdatePhase::Checking | UpdatePhase::Working { .. } => &[],
        }
    }

    /// The radio must stay on: a check or an install is running, or a
    /// release waits for the user's choice.
    #[must_use]
    pub const fn wants_radio(&self) -> bool {
        matches!(
            self.phase,
            UpdatePhase::Checking | UpdatePhase::Available(_) | UpdatePhase::Working { .. }
        )
    }

    /// A check or an install is running: sleep, recording and the portal
    /// must wait.
    #[must_use]
    pub const fn busy(&self) -> bool {
        matches!(
            self.phase,
            UpdatePhase::Checking | UpdatePhase::Working { .. }
        )
    }

    /// Percent done of the running step, 0 to 100.
    #[must_use]
    pub fn percent(&self) -> u8 {
        match self.phase {
            UpdatePhase::Working { done, total, .. } if total > 0 => {
                u8::try_from((done.min(total) * 100) / total).unwrap_or(100)
            }
            _ => 0,
        }
    }

    pub fn take_request(&mut self) -> Option<UpdateRequest> {
        self.request.take()
    }

    /// The screen was opened: start a check, unless an install is running or
    /// waiting for its restart.
    pub fn open(&mut self) {
        if matches!(
            self.phase,
            UpdatePhase::Working { .. } | UpdatePhase::Installed(_)
        ) {
            return;
        }
        self.begin_check();
    }

    fn begin_check(&mut self) {
        self.phase = UpdatePhase::Checking;
        self.row = 0;
        self.request = Some(UpdateRequest::Check);
        self.drawn_bucket = None;
    }

    /// ▲ (negative) or ▼ (positive) over the rows.
    pub fn move_row(&mut self, delta: i32) {
        let count = self.rows().len();
        if count == 0 {
            return;
        }
        let next = (self.row as i32 + delta).rem_euclid(count as i32);
        self.row = next as usize;
    }

    /// Select on the chosen row. `battery_percent` and `on_usb` are the
    /// power state now, for the install's gate.
    pub fn select(&mut self, battery_percent: Option<u8>, on_usb: bool) -> Outcome {
        let label = self.rows().get(self.row).copied();
        match (&self.phase, label) {
            (UpdatePhase::Available(found), Some("Download and install")) => {
                if let Err(refusal) = may_start(battery_percent, on_usb) {
                    self.phase = UpdatePhase::Failed(refusal.into());
                    self.row = 0;
                    return Outcome::Stay;
                }
                let found = found.clone();
                self.target = Some(found.version);
                self.phase = UpdatePhase::Working {
                    step: Step::Download,
                    done: 0,
                    total: found.size,
                };
                self.drawn_bucket = None;
                self.request = Some(UpdateRequest::Install(found.release));
                Outcome::Stay
            }
            (_, Some("Check again")) => {
                self.begin_check();
                Outcome::Stay
            }
            (UpdatePhase::Installed(_), Some("Restart now")) => {
                self.request = Some(UpdateRequest::Restart);
                Outcome::Stay
            }
            (_, Some("Back" | "Later")) => Outcome::Leave,
            _ => Outcome::Stay,
        }
    }

    /// Hold BOOT: leave, except in the middle of an install, which it
    /// cancels. The screen then says the download stopped.
    pub fn hold_back(&mut self) -> Outcome {
        match self.phase {
            UpdatePhase::Working { .. } => {
                self.request = Some(UpdateRequest::Cancel);
                self.phase = UpdatePhase::Failed(Failure::Stopped);
                self.row = 0;
                Outcome::Stay
            }
            UpdatePhase::Checking => {
                self.request = Some(UpdateRequest::Cancel);
                self.phase = UpdatePhase::UpToDate;
                Outcome::Leave
            }
            _ => Outcome::Leave,
        }
    }

    /// The main loop reports a failure the worker never saw (no Wi-Fi, a
    /// recording in the way). Only a running check or install takes it.
    pub fn fail(&mut self, failure: Failure) -> bool {
        if self.busy() {
            self.phase = UpdatePhase::Failed(failure);
            self.row = 0;
            true
        } else {
            false
        }
    }

    /// Apply a worker event; `now` is the clock text for `checked_at`. True
    /// when the screen must be drawn again.
    pub fn on_event(&mut self, event: UpdateEvent, now: Option<String>) -> bool {
        match (event, &self.phase) {
            (UpdateEvent::Checked(result), UpdatePhase::Checking) => {
                self.checked_at = now;
                self.row = 0;
                self.phase = match result {
                    Err(failure) => UpdatePhase::Failed(failure),
                    Ok(release) => match ota::check(self.installed, release.version) {
                        UpdateCheck::UpToDate => UpdatePhase::UpToDate,
                        UpdateCheck::Available(version) => match release.app_image() {
                            Ok(asset) => UpdatePhase::Available(Found {
                                size: asset.size,
                                version,
                                release,
                            }),
                            Err(error) => {
                                UpdatePhase::Failed(Failure::WrongImage(format!("{error}")))
                            }
                        },
                    },
                };
                true
            }
            (
                UpdateEvent::Progress { step, done, total },
                UpdatePhase::Working { step: before, .. },
            ) => {
                let changed_step = step != *before;
                self.phase = UpdatePhase::Working { step, done, total };
                let bucket = self.percent() / 10;
                let redraw = changed_step || self.drawn_bucket != Some(bucket);
                if redraw {
                    self.drawn_bucket = Some(bucket);
                }
                redraw
            }
            (UpdateEvent::Installed, UpdatePhase::Working { .. }) => {
                self.row = 0;
                self.phase = UpdatePhase::Installed(self.target.unwrap_or(self.installed));
                true
            }
            (UpdateEvent::Failed(failure), UpdatePhase::Working { .. } | UpdatePhase::Checking) => {
                self.row = 0;
                self.phase = UpdatePhase::Failed(failure);
                true
            }
            // A late event of a cancelled job, or a stray one: ignored.
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Failure, Outcome, Step, UpdateEvent, UpdatePhase, UpdateRequest, UpdateUi};
    use crate::ota::{parse_release, Release, Version};

    fn v(text: &str) -> Version {
        Version::parse(text).unwrap()
    }

    fn release(tag: &str, size: u64) -> Release {
        let json = format!(
            r#"{{"tag_name":"{tag}","assets":[
              {{"name":"wave-epaper-{tag}-app.bin","size":{size},
                "browser_download_url":"https://github.com/o/r/releases/download/{tag}/wave-epaper-{tag}-app.bin"}},
              {{"name":"SHA256SUMS.txt","size":300,
                "browser_download_url":"https://github.com/o/r/releases/download/{tag}/SHA256SUMS.txt"}}]}}"#
        );
        parse_release(&json).unwrap()
    }

    fn ui() -> UpdateUi {
        UpdateUi::new(v("0.13.0"))
    }

    /// Open, check, find a newer release: the Available screen.
    fn available() -> UpdateUi {
        let mut ui = ui();
        ui.open();
        ui.take_request();
        assert!(ui.on_event(
            UpdateEvent::Checked(Ok(release("v0.13.1", 3_200_000))),
            Some("12:04".into())
        ));
        ui
    }

    #[test]
    fn opening_starts_a_check_and_the_radio() {
        let mut ui = ui();
        ui.open();
        assert_eq!(ui.phase(), &UpdatePhase::Checking);
        assert!(ui.wants_radio() && ui.busy());
        assert_eq!(ui.take_request(), Some(UpdateRequest::Check));
        assert_eq!(ui.take_request(), None, "a request is taken once");
        assert!(ui.rows().is_empty(), "no rows while checking");
    }

    #[test]
    fn a_newer_release_is_offered_with_its_size() {
        let ui = available();
        let UpdatePhase::Available(found) = ui.phase() else {
            panic!("not available: {:?}", ui.phase());
        };
        assert_eq!(found.version, v("0.13.1"));
        assert_eq!(found.size, 3_200_000);
        assert_eq!(ui.checked_at(), Some("12:04"));
        assert_eq!(ui.rows(), ["Download and install", "Check again", "Back"]);
        assert!(
            ui.wants_radio() && !ui.busy(),
            "the radio waits for the choice"
        );
    }

    #[test]
    fn the_same_or_an_older_release_is_up_to_date_and_the_radio_goes_off() {
        for tag in ["v0.13.0", "v0.12.7"] {
            let mut ui = ui();
            ui.open();
            ui.on_event(UpdateEvent::Checked(Ok(release(tag, 1_000))), None);
            assert_eq!(ui.phase(), &UpdatePhase::UpToDate, "{tag}");
            assert!(!ui.wants_radio());
            assert_eq!(ui.rows(), ["Check again", "Back"]);
        }
    }

    #[test]
    fn a_release_without_its_app_image_is_a_wrong_image_not_an_offer() {
        let mut ui = ui();
        ui.open();
        let mut bad = release("v0.13.1", 1_000);
        bad.assets.retain(|asset| asset.name == "SHA256SUMS.txt");
        ui.on_event(UpdateEvent::Checked(Ok(bad)), None);
        assert!(matches!(
            ui.phase(),
            UpdatePhase::Failed(Failure::WrongImage(_))
        ));
    }

    #[test]
    fn a_failed_check_shows_its_failure() {
        let mut ui = ui();
        ui.open();
        ui.on_event(UpdateEvent::Checked(Err(Failure::Unreachable)), None);
        assert_eq!(ui.phase(), &UpdatePhase::Failed(Failure::Unreachable));
        assert_eq!(ui.rows(), ["Back"]);
        assert_eq!(ui.select(Some(80), false), Outcome::Leave);
    }

    #[test]
    fn install_needs_thirty_percent_or_usb() {
        let mut low = available();
        assert_eq!(low.select(Some(24), false), Outcome::Stay);
        let UpdatePhase::Failed(failure) = low.phase() else {
            panic!("{:?}", low.phase());
        };
        assert_eq!(failure.title(), "Battery 24%");
        assert_eq!(failure.detail(), "Charge to 30% or connect USB to update.");
        assert_eq!(low.take_request(), None, "nothing is requested");

        let mut unknown = available();
        unknown.select(None, false);
        assert_eq!(
            unknown.phase(),
            &UpdatePhase::Failed(Failure::BatteryUnknown)
        );

        let mut usb = available();
        usb.select(Some(5), true);
        assert!(matches!(usb.phase(), UpdatePhase::Working { .. }));
        assert!(matches!(
            usb.take_request(),
            Some(UpdateRequest::Install(_))
        ));
    }

    #[test]
    fn progress_redraws_once_per_ten_percent_and_on_a_new_step() {
        let mut ui = available();
        ui.select(Some(80), false);
        ui.take_request();
        let total = 1_000_u64;
        let mut redraws = 0;
        for done in (0..=total).step_by(10) {
            let drew = ui.on_event(
                UpdateEvent::Progress {
                    step: Step::Download,
                    done,
                    total,
                },
                None,
            );
            redraws += u32::from(drew);
        }
        // Buckets 0 to 10: eleven redraws for a hundred events.
        assert_eq!(redraws, 11);
        assert_eq!(ui.percent(), 100);
        assert!(ui.on_event(
            UpdateEvent::Progress {
                step: Step::Verify,
                done: 0,
                total: 0
            },
            None
        ));
        assert_eq!(ui.percent(), 0, "an unknown total is zero percent");
    }

    #[test]
    fn an_installed_image_waits_for_a_restart_and_keeps_its_screen_on_reopen() {
        let mut ui = available();
        ui.select(Some(80), false);
        ui.take_request();
        ui.on_event(UpdateEvent::Installed, None);
        assert_eq!(ui.phase(), &UpdatePhase::Installed(v("0.13.1")));
        assert!(!ui.wants_radio());
        assert_eq!(ui.rows(), ["Restart now", "Later"]);
        // Opening again does not start a check over the finished install.
        ui.open();
        assert_eq!(ui.phase(), &UpdatePhase::Installed(v("0.13.1")));
        assert_eq!(ui.take_request(), None);
        ui.move_row(1);
        assert_eq!(ui.select(Some(80), false), Outcome::Leave, "Later leaves");
        ui.move_row(-1);
        assert_eq!(ui.select(Some(80), false), Outcome::Stay);
        assert_eq!(ui.take_request(), Some(UpdateRequest::Restart));
    }

    #[test]
    fn hold_boot_cancels_an_install_and_leaves_everything_else() {
        let mut ui = available();
        ui.select(Some(80), false);
        ui.take_request();
        assert_eq!(ui.hold_back(), Outcome::Stay, "it cancels, not leaves");
        assert_eq!(ui.take_request(), Some(UpdateRequest::Cancel));
        assert_eq!(ui.phase(), &UpdatePhase::Failed(Failure::Stopped));
        // The worker's late answer does not change the screen.
        assert!(!ui.on_event(UpdateEvent::Failed(Failure::Stopped), None));
        assert!(!ui.on_event(
            UpdateEvent::Progress {
                step: Step::Download,
                done: 5,
                total: 10
            },
            None
        ));
        assert_eq!(ui.hold_back(), Outcome::Leave, "a second hold leaves");

        let mut checking = self::ui();
        checking.open();
        checking.take_request();
        assert_eq!(checking.hold_back(), Outcome::Leave);
        assert_eq!(checking.take_request(), Some(UpdateRequest::Cancel));
        assert!(!checking.on_event(UpdateEvent::Checked(Ok(release("v9.0.0", 1))), None));
    }

    #[test]
    fn rows_wrap_and_check_again_checks_again() {
        let mut ui = available();
        ui.move_row(-1);
        assert_eq!(ui.row(), 2);
        ui.move_row(1);
        assert_eq!(ui.row(), 0);
        ui.move_row(1);
        assert_eq!(ui.select(Some(80), false), Outcome::Stay);
        assert_eq!(ui.phase(), &UpdatePhase::Checking);
        assert_eq!(ui.take_request(), Some(UpdateRequest::Check));
    }

    #[test]
    fn the_main_loop_can_fail_a_running_check_but_not_a_finished_one() {
        let mut ui = ui();
        ui.open();
        assert!(ui.fail(Failure::NoWifi));
        assert_eq!(ui.phase(), &UpdatePhase::Failed(Failure::NoWifi));
        assert!(!ui.fail(Failure::Busy), "a failed screen stays as it is");
        assert_eq!(Failure::NoWifi.detail(), "Set it in Wi-Fi & transfer.");
        assert_eq!(Failure::NoSlot.title(), "No update slot");
        assert!(Failure::WrongImage("version differs".into())
            .detail()
            .contains("version differs"));
    }
}
