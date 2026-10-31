use std::fs;
use std::path::Path;
use std::time::{Duration, Instant};

use chrono::{DateTime, Local};

pub use crate::storage::FileFingerprint as Fingerprint;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Observation {
    Unchanged,
    Settling,
    ReadNow,
    Missing,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChangeDetector {
    known_fp: Option<Fingerprint>,
    settling_fp: Option<Fingerprint>,
    settling_count: usize,
    missing_reported: bool,
    failed_fp: Option<Fingerprint>,
    warned_failed_fp: Option<Fingerprint>,
    last_read_now_fp: Option<Fingerprint>,
    last_observed_at: Option<Instant>,
}

impl Default for ChangeDetector {
    fn default() -> Self {
        Self::new(None)
    }
}

impl ChangeDetector {
    pub fn new(known_fp: Option<Fingerprint>) -> Self {
        Self {
            known_fp,
            settling_fp: None,
            settling_count: 0,
            missing_reported: false,
            failed_fp: None,
            warned_failed_fp: None,
            last_read_now_fp: None,
            last_observed_at: None,
        }
    }

    /// Record a known fingerprint (after load, save, or reload).
    pub fn record_known(&mut self, fp: Option<Fingerprint>) {
        self.known_fp = fp;
        self.settling_fp = None;
        self.settling_count = 0;
        self.missing_reported = false;
        self.failed_fp = None;
        self.warned_failed_fp = None;
        self.last_read_now_fp = None;
    }

    /// Record a read failure for a fingerprint. Returns true if this is the first
    /// time reporting a failure for this fingerprint (i.e. caller should warn).
    pub fn record_failed(&mut self, fp: Fingerprint) -> bool {
        self.failed_fp = Some(fp);
        self.settling_fp = None;
        self.settling_count = 0;
        self.last_read_now_fp = None;

        if self.warned_failed_fp == Some(fp) {
            false
        } else {
            self.warned_failed_fp = Some(fp);
            true
        }
    }

    /// Observe the current fingerprint of the file.
    pub fn observe(&mut self, fp: Option<Fingerprint>, now: Instant) -> Observation {
        self.last_observed_at = Some(now);

        let curr_fp = match fp {
            Some(f) => f,
            None => {
                // File is missing
                self.settling_fp = None;
                self.settling_count = 0;
                if !self.missing_reported {
                    self.missing_reported = true;
                    // Reset known_fp so reappearance is recognized as a change
                    self.known_fp = None;
                    self.last_read_now_fp = None;
                    return Observation::Missing;
                }
                return Observation::Unchanged;
            }
        };

        // File exists
        self.missing_reported = false;

        // If identical to the known fingerprint, no change.
        if self.known_fp == Some(curr_fp) {
            self.settling_fp = None;
            self.settling_count = 0;
            return Observation::Unchanged;
        }

        // If this fingerprint previously failed to read, do not retry until it changes.
        if self.failed_fp == Some(curr_fp) {
            self.settling_fp = None;
            self.settling_count = 0;
            return Observation::Unchanged;
        }

        // If ReadNow was already emitted for this exact fingerprint and hasn't yet been
        // recorded as known or failed, don't spam ReadNow.
        if self.last_read_now_fp == Some(curr_fp) {
            return Observation::Unchanged;
        }

        // Settle before reading: wait until identical on two consecutive observations.
        if self.settling_fp == Some(curr_fp) {
            self.settling_count += 1;
            if self.settling_count >= 2 {
                self.settling_fp = None;
                self.settling_count = 0;
                self.last_read_now_fp = Some(curr_fp);
                Observation::ReadNow
            } else {
                Observation::Settling
            }
        } else {
            // New or changed fingerprint during settling resets wait
            self.settling_fp = Some(curr_fp);
            self.settling_count = 1;
            Observation::Settling
        }
    }
}

/// Stat the fingerprint of a path. Returns:
/// - Ok(Some(fp)) if file exists
/// - Ok(None) if file does not exist (NotFound)
/// - Err(e) for any other I/O errors (permission denied, etc.)
pub fn stat_fingerprint(path: &Path) -> Result<Option<Fingerprint>, std::io::Error> {
    match fs::metadata(path) {
        Ok(meta) => Ok(Some(Fingerprint::from_meta(&meta))),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(e),
    }
}

/// Compute the next poll timeout.
/// With live reload on, the poll timeout becomes `min(time until next midnight capped at 60s, reload_interval)`,
/// still floored so it never reaches zero.
/// With live reload off, behavior is exactly as in Phase 1 (clamped between 1s and 60s).
pub fn next_timeout(now: DateTime<Local>, interval: Duration, live_reload: bool) -> Duration {
    let tomorrow = now.date_naive().succ_opt().unwrap_or(now.date_naive());
    let time_until_midnight = if let Some(naive_midnight) = tomorrow.and_hms_opt(0, 0, 0) {
        let next_midnight = match naive_midnight.and_local_timezone(now.timezone()) {
            chrono::LocalResult::Single(dt) => Some(dt),
            chrono::LocalResult::Ambiguous(dt, _) => Some(dt),
            chrono::LocalResult::None => None,
        };
        if let Some(next_midnight) = next_midnight {
            (next_midnight - now)
                .to_std()
                .unwrap_or(Duration::from_secs(60))
        } else {
            Duration::from_secs(60)
        }
    } else {
        Duration::from_secs(60)
    };

    if !live_reload {
        // Phase 1 behavior
        time_until_midnight.clamp(Duration::from_secs(1), Duration::from_secs(60))
    } else {
        // Live reload on: min(time until next midnight capped at 60s, reload_interval),
        // floored so it never reaches zero (at least 1ms).
        let capped_midnight = time_until_midnight.min(Duration::from_secs(60));
        let timeout = capped_midnight.min(interval);
        timeout.max(Duration::from_millis(1))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Timelike;
    use std::time::SystemTime;
    use tempfile::tempdir;

    fn make_fp(secs: u64, len: u64, ino: u64) -> Fingerprint {
        Fingerprint::new_test(SystemTime::UNIX_EPOCH + Duration::from_secs(secs), len, ino)
    }

    #[test]
    fn test_detector_no_change() {
        let fp0 = make_fp(100, 50, 1);
        let mut detector = ChangeDetector::new(Some(fp0));
        let now = Instant::now();

        assert_eq!(detector.observe(Some(fp0), now), Observation::Unchanged);
        assert_eq!(detector.observe(Some(fp0), now), Observation::Unchanged);
    }

    #[test]
    fn test_detector_change_then_settle_then_read_now() {
        let fp0 = make_fp(100, 50, 1);
        let fp1 = make_fp(105, 60, 1);
        let mut detector = ChangeDetector::new(Some(fp0));
        let now = Instant::now();

        // First observation of fp1 -> Settling
        assert_eq!(detector.observe(Some(fp1), now), Observation::Settling);

        // Second consecutive observation of fp1 -> ReadNow
        assert_eq!(detector.observe(Some(fp1), now), Observation::ReadNow);

        // Subsequent observation before known/failed recorded -> Unchanged (no read storm)
        assert_eq!(detector.observe(Some(fp1), now), Observation::Unchanged);

        // After record_known -> Unchanged
        detector.record_known(Some(fp1));
        assert_eq!(detector.observe(Some(fp1), now), Observation::Unchanged);
    }

    #[test]
    fn test_detector_second_change_during_settling_resets_wait() {
        let fp0 = make_fp(100, 50, 1);
        let fp1 = make_fp(105, 60, 1);
        let fp2 = make_fp(110, 70, 1);
        let mut detector = ChangeDetector::new(Some(fp0));
        let now = Instant::now();

        // fp1 observed once -> Settling
        assert_eq!(detector.observe(Some(fp1), now), Observation::Settling);

        // fp2 observed (changed during settling!) -> resets wait, still Settling
        assert_eq!(detector.observe(Some(fp2), now), Observation::Settling);

        // fp2 observed second time -> ReadNow
        assert_eq!(detector.observe(Some(fp2), now), Observation::ReadNow);
    }

    #[test]
    fn test_detector_change_that_reverts_to_original_fingerprint() {
        let fp0 = make_fp(100, 50, 1);
        let fp1 = make_fp(105, 60, 1);
        let mut detector = ChangeDetector::new(Some(fp0));
        let now = Instant::now();

        assert_eq!(detector.observe(Some(fp1), now), Observation::Settling);
        // Reverts to fp0 (known)
        assert_eq!(detector.observe(Some(fp0), now), Observation::Unchanged);
    }

    #[test]
    fn test_detector_own_write_ignored_after_record_known() {
        let fp0 = make_fp(100, 50, 1);
        let fp1 = make_fp(105, 60, 1);
        let mut detector = ChangeDetector::new(Some(fp0));
        let now = Instant::now();

        // Simulate save: Slate records its own write
        detector.record_known(Some(fp1));

        // Observation of fp1 must be Unchanged (not Settling or ReadNow)
        assert_eq!(detector.observe(Some(fp1), now), Observation::Unchanged);
    }

    #[test]
    fn test_detector_missing_then_reappearing() {
        let fp0 = make_fp(100, 50, 1);
        let fp1 = make_fp(105, 60, 1);
        let mut detector = ChangeDetector::new(Some(fp0));
        let now = Instant::now();

        // First missing observation -> Missing
        assert_eq!(detector.observe(None, now), Observation::Missing);

        // Second missing observation -> Unchanged (reported once)
        assert_eq!(detector.observe(None, now), Observation::Unchanged);

        // Reappears -> Settling, then ReadNow
        assert_eq!(detector.observe(Some(fp1), now), Observation::Settling);
        assert_eq!(detector.observe(Some(fp1), now), Observation::ReadNow);
    }

    #[test]
    fn test_detector_failed_read_not_retried_until_fp_changes_and_warned_once() {
        let fp0 = make_fp(100, 50, 1);
        let fp_bad = make_fp(105, 60, 1);
        let fp_fixed = make_fp(110, 80, 1);
        let mut detector = ChangeDetector::new(Some(fp0));
        let now = Instant::now();

        // Bad file settles and triggers ReadNow
        assert_eq!(detector.observe(Some(fp_bad), now), Observation::Settling);
        assert_eq!(detector.observe(Some(fp_bad), now), Observation::ReadNow);

        // Read fails, record_failed returns true (should warn once)
        assert!(detector.record_failed(fp_bad));

        // While fp_bad persists, observe returns Unchanged
        assert_eq!(detector.observe(Some(fp_bad), now), Observation::Unchanged);
        assert_eq!(detector.observe(Some(fp_bad), now), Observation::Unchanged);

        // If record_failed is called again for the same fp, it returns false (already warned)
        assert!(!detector.record_failed(fp_bad));

        // When fp changes to fp_fixed, it settles and reads normally
        assert_eq!(detector.observe(Some(fp_fixed), now), Observation::Settling);
        assert_eq!(detector.observe(Some(fp_fixed), now), Observation::ReadNow);
    }

    #[test]
    fn test_detector_stat_errors_treated_as_unchanged() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("todos.json");

        // Non-existent file -> Ok(None)
        let res = stat_fingerprint(&path).unwrap();
        assert_eq!(res, None);

        // Existing file -> Ok(Some(fp))
        fs::write(&path, "content").unwrap();
        let res = stat_fingerprint(&path).unwrap();
        assert!(res.is_some());
    }

    #[test]
    fn test_next_timeout_live_reload_on_stays_at_or_below_interval() {
        use chrono::TimeZone;
        // Noon: midnight is 12 hours away
        let now = Local.with_ymd_and_hms(2026, 10, 31, 12, 0, 0).unwrap();
        let interval = Duration::from_millis(500);

        let timeout = next_timeout(now, interval, true);
        assert_eq!(timeout, Duration::from_millis(500));
        assert!(timeout <= interval);
    }

    #[test]
    fn test_next_timeout_live_reload_off_equals_phase1() {
        use chrono::TimeZone;
        // Noon: midnight is 12 hours away, capped at 60s
        let now = Local.with_ymd_and_hms(2026, 10, 31, 12, 0, 0).unwrap();
        let interval = Duration::from_millis(500);

        let timeout_off = next_timeout(now, interval, false);
        assert_eq!(timeout_off, Duration::from_secs(60));
    }

    #[test]
    fn test_next_timeout_midnight_sooner_than_interval_wins_and_never_zero() {
        use chrono::TimeZone;
        // 23:59:59.900 -> 100ms until midnight
        let now = Local
            .with_ymd_and_hms(2026, 10, 31, 23, 59, 59)
            .unwrap()
            .with_nanosecond(900_000_000)
            .unwrap();
        let interval = Duration::from_millis(500);

        let timeout = next_timeout(now, interval, true);
        // Midnight is 100ms away, interval is 500ms -> midnight wins (100ms)
        assert_eq!(timeout, Duration::from_millis(100));
        assert!(timeout < interval);

        // Exactly at midnight (or 0ms left) -> never zero (at least 1ms)
        let midnight = Local
            .with_ymd_and_hms(2026, 10, 31, 23, 59, 59)
            .unwrap()
            .with_nanosecond(999_999_999)
            .unwrap();
        let timeout_zero = next_timeout(midnight, interval, true);
        assert!(timeout_zero >= Duration::from_millis(1));
        assert_ne!(timeout_zero, Duration::ZERO);
    }
}
