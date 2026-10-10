// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The time of day a moment falls on where the person sits, which is
//! what the console's time gutter writes (`crates/sprawling/spec/Console.lean`
//! §8-11).
//!
//! The ledger keeps UTC milliseconds; a person reads their own clock, as
//! the WebUI's message heads do in the browser's zone. The zone is read
//! from the operating system for each moment, so a line written after a
//! change to daylight saving time reads the new offset. Nothing here
//! reads the time itself: the moment is always a parameter.

use console_ffi::scene::TimeOfDay;
use kernel::TimeMs;

/// How a moment becomes a time of day: the machine's zone in
/// production, UTC in a test that must read the same everywhere.
pub(crate) type Zone = fn(TimeMs) -> Option<TimeOfDay>;

/// The time of day `at` falls on in this machine's zone; nothing for a
/// moment the zone cannot place.
pub(crate) fn local(at: TimeMs) -> Option<TimeOfDay> {
    use chrono::{TimeZone as _, Timelike as _};
    let millis = i64::try_from(at.value()).ok()?;
    let moment = chrono::Local.timestamp_millis_opt(millis).single()?;
    TimeOfDay::from_seconds(moment.num_seconds_from_midnight())
}

/// The time of day `at` falls on in UTC.
#[cfg(test)]
pub(crate) fn utc(at: TimeMs) -> Option<TimeOfDay> {
    let seconds = at.value().checked_div(1_000)?.checked_rem(86_400)?;
    TimeOfDay::from_seconds(u32::try_from(seconds).ok()?)
}
