// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use crate::scene::TimeOfDay;
use kernel::TimeMs;

pub(crate) type Zone = fn(TimeMs) -> Option<TimeOfDay>;

pub(crate) fn local(at: TimeMs) -> Option<TimeOfDay> {
    TimeOfDay::from_seconds(console_ffi::zone::seconds_since_midnight(at.value())?)
}

#[cfg(test)]
pub(crate) fn utc(at: TimeMs) -> Option<TimeOfDay> {
    let seconds = at.value().checked_div(1_000)?.checked_rem(86_400)?;
    TimeOfDay::from_seconds(u32::try_from(seconds).ok()?)
}
