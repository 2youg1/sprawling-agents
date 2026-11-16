// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::sync::{Arc, Mutex, PoisonError};

use kernel::consts_policy::CLOCK_ZONES_MAX;
use kernel::{AxCode, AxError, ClockStampGranularity, ClockZone, Temporal, TimeMs};

pub fn iso(at: TimeMs) -> String {
    format_at(at, 0)
}

pub fn parse_iso(raw: &str) -> Result<TimeMs, AxError> {
    let refused = || {
        AxError::failure(AxCode::InvalidArgs, "read a UTC moment", raw).with_recovery(
            "write the moment in UTC to the second, ending in Z, the way a clock line \
             does: 2026-05-14T09:31:07Z; turn a local time or an offset into UTC first",
        )
    };
    let millis = civil_fields(raw.as_bytes())
        .and_then(millis_of)
        .ok_or_else(refused)?;
    u64::try_from(millis)
        .map(TimeMs::new)
        .map_err(|_before_1970| refused())
}

const ISO_SEPARATORS: [(usize, u8); 6] = [
    (4, b'-'),
    (7, b'-'),
    (10, b'T'),
    (13, b':'),
    (16, b':'),
    (19, b'Z'),
];
const ISO_FIELDS: [(usize, usize); 6] = [(0, 4), (5, 7), (8, 10), (11, 13), (14, 16), (17, 19)];

fn civil_fields(bytes: &[u8]) -> Option<[i128; 6]> {
    let shaped = bytes.len() == 20
        && ISO_SEPARATORS
            .iter()
            .all(|(at, separator)| bytes.get(*at) == Some(separator));
    if !shaped {
        return None;
    }
    let [year, month, day, hour, minute, second] = ISO_FIELDS.map(|(from, to)| {
        bytes.get(from..to)?.iter().try_fold(0i128, |number, byte| {
            let digit = char::from(*byte).to_digit(10)?;
            number.checked_mul(10)?.checked_add(i128::from(digit))
        })
    });
    Some([year?, month?, day?, hour?, minute?, second?])
}

#[expect(
    clippy::arithmetic_side_effects,
    reason = "every field is at most four decimal digits, so the day count and the \
              milliseconds stay far inside i128; the divisors are positive constants"
)]
fn millis_of([year, month, day, hour, minute, second]: [i128; 6]) -> Option<i128> {
    if !(1..=12).contains(&month) || hour > 23 || minute > 59 || second > 59 {
        return None;
    }
    let y = if month <= 2 { year - 1 } else { year };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let mp = if month > 2 { month - 3 } else { month + 9 };
    let doy = (153 * mp + 2) / 5 + day - 1;
    let days = era * 146_097 + yoe * 365 + yoe / 4 - yoe / 100 + doy - 719_468;
    (civil_from_days(days) == (year, month, day))
        .then_some((days * 86_400 + hour * 3_600 + minute * 60 + second) * 1_000)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct UtcSpan {
    since: Option<TimeMs>,
    until: Option<TimeMs>,
}

impl UtcSpan {
    pub fn new(since: Option<TimeMs>, until: Option<TimeMs>) -> Result<UtcSpan, AxError> {
        if let (Some(start), Some(end)) = (since, until)
            && end <= start
        {
            return Err(AxError::failure(
                AxCode::InvalidArgs,
                "select a span of time",
                format!("{} to {}", iso(start), iso(end)),
            )
            .with_recovery(
                "give an end after the start; the end itself is left out, so a span that \
                 ends where it starts holds no moment",
            ));
        }
        Ok(UtcSpan { since, until })
    }

    #[must_use]
    pub fn since(&self) -> Option<TimeMs> {
        self.since
    }

    #[must_use]
    pub fn until(&self) -> Option<TimeMs> {
        self.until
    }

    #[must_use]
    pub fn contains(&self, at: TimeMs) -> bool {
        self.since.is_none_or(|since| since <= at) && self.until.is_none_or(|until| at < until)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ZoneEntry {
    pub id: String,
    pub offset_min: i32,
    pub local: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClockStamp {
    pub utc_ms: TimeMs,
    pub zones: Vec<ZoneEntry>,
}

impl ClockStamp {
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

#[derive(Debug)]
pub struct StampGate {
    granularity: ClockStampGranularity,
    zones: Vec<ClockZone>,
    last_bucket: Option<u64>,
}

impl StampGate {
    pub fn new(granularity: ClockStampGranularity, zones: Vec<ClockZone>) -> StampGate {
        StampGate {
            granularity,
            zones,
            last_bucket: None,
        }
    }

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

#[derive(Debug, Clone, Default)]
pub struct ClockReading(Arc<Mutex<Option<TimeMs>>>);

impl ClockReading {
    pub fn keep(&self, at: TimeMs) {
        *self.0.lock().unwrap_or_else(PoisonError::into_inner) = Some(at);
    }

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

        assert!(
            gate.observe(TimeMs::new(1_000), Temporal::Timeless)
                .unwrap()
                .is_some()
        );

        assert!(
            gate.observe(TimeMs::new(30_000), Temporal::Timeless)
                .unwrap()
                .is_none()
        );

        let s = gate
            .observe(TimeMs::new(61_000), Temporal::Timeless)
            .unwrap()
            .unwrap();
        assert_eq!(s.utc_ms, TimeMs::new(61_000));
    }

    #[test]
    fn an_iso_moment_reads_back_to_the_second_it_was_written() {
        let last = 253_402_300_799_999u64;
        let step = 7_919_999_999u64;
        let moments = (0..=last / step).map(|k| k * step).chain([
            0,
            999,
            951_782_400_000,
            1_709_251_199_999,
            4_107_542_400_000,
            last,
        ]);
        for at in moments {
            let written = iso(TimeMs::new(at));
            assert_eq!(
                parse_iso(&written),
                Ok(TimeMs::new(at - at % 1_000)),
                "{written}"
            );
        }
        assert_eq!(
            parse_iso("2024-02-29T23:59:59Z"),
            Ok(TimeMs::new(1_709_251_199_000))
        );
    }

    #[test]
    fn parse_iso_refuses_every_spelling_iso_does_not_write() {
        for raw in [
            "2026-05-14T18:31:07+09:00",
            "2026-05-14T09:31:07.000Z",
            "2026-05-14",
            "2026-05-14 09:31:07Z",
            "2026-05-14t09:31:07z",
            "2026-02-30T00:00:00Z",
            "2025-02-29T00:00:00Z",
            "2100-02-29T00:00:00Z",
            "2026-13-01T00:00:00Z",
            "2026-00-10T00:00:00Z",
            "2026-05-00T00:00:00Z",
            "2026-05-14T24:00:00Z",
            "2026-05-14T09:60:00Z",
            "2026-12-31T23:59:60Z",
            "1969-12-31T23:59:59Z",
            "+026-05-14T09:31:07Z",
            " 2026-05-14T09:31:07Z",
            "",
        ] {
            let refused = parse_iso(raw);
            assert!(
                matches!(&refused, Err(err) if *err.code() == AxCode::InvalidArgs
                    && err.subject() == raw
                    && err.recovery().contains("2026-05-14T09:31:07Z")),
                "{raw:?}: {refused:?}"
            );
        }
    }

    #[test]
    fn a_span_keeps_its_start_and_leaves_out_its_end() {
        let at = TimeMs::new;
        let both = UtcSpan::new(Some(at(1_000)), Some(at(2_000))).unwrap();
        assert_eq!(
            [999, 1_000, 1_999, 2_000].map(|t| both.contains(at(t))),
            [false, true, true, false]
        );
        let until = UtcSpan::new(None, Some(at(2_000))).unwrap();
        assert_eq!([0, 2_000].map(|t| until.contains(at(t))), [true, false]);
        let since = UtcSpan::new(Some(at(1_000)), None).unwrap();
        assert_eq!(
            [999, u64::MAX].map(|t| since.contains(at(t))),
            [false, true]
        );
        assert!(UtcSpan::default().contains(at(0)));
    }

    #[test]
    fn a_span_with_no_moment_in_it_is_refused() {
        let at = TimeMs::new;
        for (since, until) in [(2_000, 2_000), (2_001, 2_000)] {
            let refused = UtcSpan::new(Some(at(since)), Some(at(until)));
            assert!(
                matches!(&refused, Err(err) if *err.code() == AxCode::InvalidArgs
                    && err.subject().contains(&iso(at(since)))
                    && err.subject().contains(&iso(at(until)))),
                "{since}..{until}: {refused:?}"
            );
        }
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
