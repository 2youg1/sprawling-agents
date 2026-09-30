// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! ClockStamp: pure formatting of an injected instant against
//! already-resolved zone offsets, and the one spelling of a moment a
//! person or a model reads: ISO 8601, UTC, to the second. Nothing here
//! samples a clock; the calendar is integer arithmetic so replay and
//! live share one algorithm and no tz database.
//!
//! Emission (`StampGate`): `Off` emits never (A18 zero-byte); the first
//! result of a run emits once; `Timestamped` tools emit every result;
//! `Timeless` tools emit only when the granularity bucket changed. The
//! bucket decides how often, never what a stamp says: a stamp always
//! carries the reading it was given.
//!
//! [`ClockReading`] is how a tool face learns what time it is without a
//! clock of its own (runtime-SPEC 8-53).

use std::sync::{Arc, Mutex, PoisonError};

use kernel::consts_policy::CLOCK_ZONES_MAX;
use kernel::{AxCode, AxError, ClockStampGranularity, ClockZone, Temporal, TimeMs};

/// One instant in ISO 8601, UTC, to the second:
/// `2026-05-14T09:31:07Z`. The clock line and `status` both write a
/// moment through this; a reader that parses one back belongs beside it.
pub fn iso(at: TimeMs) -> String {
    format_at(at, 0)
}

/// One configured zone's row: the offset it was computed from and the
/// same second in that zone, `2026-05-14T18:31:07+09:00`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ZoneEntry {
    pub id: String,
    pub offset_min: i32,
    pub local: String,
}

/// The stamp a result envelope carries: the reading itself, and the
/// rows of the zones the run was frozen with (none, in a real city:
/// the city refuses `[clock] zones`, city-SPEC 12.7).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClockStamp {
    pub utc_ms: TimeMs,
    pub zones: Vec<ZoneEntry>,
}

impl ClockStamp {
    /// The envelope's clock line: `clock: 2026-05-14T09:31:07Z;`, then
    /// ` <id> <local>;` for each zone.
    pub fn render(&self) -> String {
        let mut out = format!("clock: {};", iso(self.utc_ms));
        for zone in &self.zones {
            out.push(' ');
            out.push_str(&zone.id);
            out.push(' ');
            out.push_str(&zone.local);
            out.push(';');
        }
        out
    }
}

fn bucket_ms(granularity: ClockStampGranularity) -> Option<u64> {
    match granularity {
        ClockStampGranularity::Off => None,
        ClockStampGranularity::Minute => Some(60_000),
        ClockStampGranularity::FiveMinute => Some(300_000),
        ClockStampGranularity::Hour => Some(3_600_000),
    }
}

/// Civil date from days since 1970-01-01 (era-based integer algorithm).
#[expect(
    clippy::arithmetic_side_effects,
    reason = "era arithmetic on i128: the input is u64 milliseconds plus an i32 offset in \
              minutes, divided down to days, far inside i128; every divisor is a positive \
              constant, so neither overflow nor division by zero is reachable"
)]
fn civil_from_days(days: i128) -> (i128, i128, i128) {
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 }.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = if m <= 2 { y + 1 } else { y };
    (year, m, d)
}

/// `at` in the zone `offset_min` minutes east of UTC, to the second,
/// with `Z` for UTC itself and `+HH:MM` or `-HH:MM` otherwise.
#[expect(
    clippy::arithmetic_side_effects,
    reason = "i128 values bounded by u64 milliseconds plus an i32 offset in minutes times \
              60_000; div_euclid and rem_euclid by positive constants, so i128::MIN and \
              division by zero are unreachable"
)]
fn format_at(at: TimeMs, offset_min: i32) -> String {
    let shifted = i128::from(at.value()) + i128::from(offset_min) * 60_000;
    let seconds = shifted.div_euclid(1_000);
    let (year, month, day) = civil_from_days(seconds.div_euclid(86_400));
    let second_of_day = seconds.rem_euclid(86_400);
    let (hour, minute, second) = (
        second_of_day / 3_600,
        second_of_day % 3_600 / 60,
        second_of_day % 60,
    );
    format!(
        "{year:04}-{month:02}-{day:02}T{hour:02}:{minute:02}:{second:02}{}",
        offset_suffix(offset_min)
    )
}

fn offset_suffix(offset_min: i32) -> String {
    if offset_min == 0 {
        return "Z".to_owned();
    }
    let sign = if offset_min < 0 { '-' } else { '+' };
    let whole = offset_min.unsigned_abs();
    format!("{sign}{:02}:{:02}", whole / 60, whole % 60)
}

/// Formats one stamp of the reading `now` and every configured zone.
/// Zones beyond `CLOCK_ZONES_MAX` are refused, not silently dropped.
pub fn stamp(now: TimeMs, zones: &[ClockZone]) -> Result<ClockStamp, AxError> {
    CLOCK_ZONES_MAX.admit(zones.len())?;
    Ok(ClockStamp {
        utc_ms: now,
        zones: zones
            .iter()
            .map(|zone| ZoneEntry {
                id: zone.id.clone(),
                offset_min: zone.offset_min,
                local: format_at(now, zone.offset_min),
            })
            .collect(),
    })
}

/// Decides, per tool result, whether a stamp rides the envelope. One
/// per run, built from the run's frozen configuration.
#[derive(Debug)]
pub struct StampGate {
    granularity: ClockStampGranularity,
    zones: Vec<ClockZone>,
    last_bucket: Option<u64>,
}

impl StampGate {
    /// `granularity` and `zones` are the run's `FrozenConfig`
    /// `clock_stamp` and `clock_zones`.
    pub fn new(granularity: ClockStampGranularity, zones: Vec<ClockZone>) -> StampGate {
        StampGate {
            granularity,
            zones,
            last_bucket: None,
        }
    }

    /// The emission rule, in one place.
    pub fn observe(
        &mut self,
        now: TimeMs,
        temporal: Temporal,
    ) -> Result<Option<ClockStamp>, AxError> {
        let Some(width) = bucket_ms(self.granularity) else {
            return Ok(None);
        };
        let bucket = now.value().checked_div(width).ok_or_else(|| {
            AxError::failure(
                AxCode::InvalidArgs,
                "bucket clock stamp",
                "zero bucket width",
            )
            .with_recovery(
                "report this against runtime::clock: every granularity except the one \
                 that shows no clock owes a bucket width above zero",
            )
        })?;
        let due = match temporal {
            Temporal::Timestamped => true,
            Temporal::Timeless => self.last_bucket != Some(bucket),
        } || self.last_bucket.is_none();
        if !due {
            return Ok(None);
        }
        self.last_bucket = Some(bucket);
        stamp(now, &self.zones).map(Some)
    }
}

/// The moment a run's driver read its clock last, shared with the tool
/// faces that report the time (runtime-SPEC 8-53).
///
/// The driver's clock hook is the one sampling point of a run, and a
/// tool face receives readings rather than sampling: the hook `keep`s
/// each reading before handing it on, and a command's stamp or
/// `status`'s `now:` line reads `latest`. What they report is therefore
/// always some line's `t`. Cloned, it is one cell.
///
/// "Not read yet" is a state of its own rather than an integer standing
/// for one. The lock holds one `Copy` value replaced whole, so a thread
/// that panicked while holding it cannot have left half a value behind,
/// and a poisoned lock is read like any other.
#[derive(Debug, Clone, Default)]
pub struct ClockReading(Arc<Mutex<Option<TimeMs>>>);

impl ClockReading {
    /// Records one reading the driver took.
    pub fn keep(&self, at: TimeMs) {
        *self.0.lock().unwrap_or_else(PoisonError::into_inner) = Some(at);
    }

    /// The last reading kept; `None` before the driver has read its
    /// clock.
    #[must_use]
    pub fn latest(&self) -> Option<TimeMs> {
        *self.0.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "test code"
)]
mod tests {
    use super::*;

    fn tokyo() -> ClockZone {
        ClockZone {
            id: "tokyo".to_owned(),
            offset_min: 540,
        }
    }

    fn rendered(at: u64, zones: &[ClockZone]) -> String {
        stamp(TimeMs::new(at), zones).unwrap().render()
    }

    #[test]
    fn a_stamp_reads_as_iso_utc_to_the_second() {
        assert_eq!(rendered(0, &[]), "clock: 1970-01-01T00:00:00Z;");
        assert_eq!(
            rendered(1_709_251_199_999, &[]),
            "clock: 2024-02-29T23:59:59Z;"
        );
        assert_eq!(
            rendered(1_785_585_607_000, &[]),
            "clock: 2026-08-01T12:00:07Z;"
        );
        assert_eq!(
            rendered(978_287_400_000, &[]),
            "clock: 2000-12-31T18:30:00Z;"
        );
    }

    #[test]
    fn a_zone_row_is_the_same_second_with_its_offset_across_midnight_both_ways() {
        // 2026-08-01 12:00:07 UTC at +540 is 21:00:07 the same day; at
        // -780 it crosses back to 23:00:07 of July 31.
        let west = ClockZone {
            id: "west".to_owned(),
            offset_min: -780,
        };
        assert_eq!(
            rendered(1_785_585_607_000, &[tokyo(), west]),
            "clock: 2026-08-01T12:00:07Z; tokyo 2026-08-01T21:00:07+09:00; \
             west 2026-07-31T23:00:07-13:00;"
        );
        assert!(stamp(TimeMs::new(0), &[]).unwrap().zones.is_empty());
    }

    #[test]
    fn zones_beyond_the_cap_are_refused_not_dropped() {
        let too_many: Vec<ClockZone> = (0..5)
            .map(|i| ClockZone {
                id: format!("z{i}"),
                offset_min: 0,
            })
            .collect();
        let err = stamp(TimeMs::new(0), &too_many).unwrap_err();
        // The refusal is the limit's own, whole: a caller that spelled a
        // sentence of its own would be a second authority for one limit.
        assert_eq!(err, CLOCK_ZONES_MAX.admit(5).unwrap_err());
    }

    #[test]
    fn off_emits_never_even_for_timestamped_tools() {
        let mut gate = StampGate::new(ClockStampGranularity::Off, Vec::new());
        for t in [1u64, 60_001, 3_600_001] {
            let out = gate.observe(TimeMs::new(t), Temporal::Timestamped).unwrap();
            assert!(out.is_none(), "A18: Off must cost zero bytes");
        }
    }

    #[test]
    fn first_result_emits_once_then_timeless_deduplicates_within_a_bucket() {
        let mut gate = StampGate::new(ClockStampGranularity::Minute, Vec::new());
        // First result of the run always stamps (even Timeless).
        assert!(
            gate.observe(TimeMs::new(1_000), Temporal::Timeless)
                .unwrap()
                .is_some()
        );
        // Same minute bucket: no stamp.
        assert!(
            gate.observe(TimeMs::new(30_000), Temporal::Timeless)
                .unwrap()
                .is_none()
        );
        // Next minute: stamps again, with the reading itself - the
        // bucket decides how often, never what the stamp says.
        let s = gate
            .observe(TimeMs::new(61_000), Temporal::Timeless)
            .unwrap()
            .unwrap();
        assert_eq!(s.utc_ms, TimeMs::new(61_000));
    }

    #[test]
    fn timestamped_emits_every_result_with_its_own_reading() {
        let mut gate = StampGate::new(ClockStampGranularity::Hour, Vec::new());
        let a = gate
            .observe(TimeMs::new(1_000), Temporal::Timestamped)
            .unwrap()
            .unwrap();
        let b = gate
            .observe(TimeMs::new(2_000), Temporal::Timestamped)
            .unwrap()
            .unwrap();
        assert_eq!(a.render(), "clock: 1970-01-01T00:00:01Z;");
        assert_eq!(b.render(), "clock: 1970-01-01T00:00:02Z;");
    }
}
