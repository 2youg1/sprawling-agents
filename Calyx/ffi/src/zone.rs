// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

pub fn seconds_since_midnight(millis: u64) -> Option<u32> {
    let (hour, minute, second) = local_fields(millis)?;
    since_midnight(hour, minute, second)
}

fn since_midnight(hour: u32, minute: u32, second: u32) -> Option<u32> {
    if hour > 23 || minute > 59 || second > 59 {
        return None;
    }
    hour.checked_mul(3_600)?
        .checked_add(minute.checked_mul(60)?)?
        .checked_add(second)
}

#[cfg(windows)]
fn local_fields(millis: u64) -> Option<(u32, u32, u32)> {
    use windows::Win32::Foundation::{FILETIME, SYSTEMTIME};
    use windows::Win32::System::Time::{
        FileTimeToSystemTime, GetTimeZoneInformation, SystemTimeToTzSpecificLocalTime,
        TIME_ZONE_INFORMATION,
    };

    let ticks = i128::from(millis)
        .checked_mul(10_000)?
        .checked_add(116_444_736_000_000_000)?;
    let whole = u64::try_from(ticks).ok()?;
    let file = FILETIME {
        dwLowDateTime: u32::try_from(whole & u64::from(u32::MAX)).ok()?,
        dwHighDateTime: u32::try_from(whole >> 32).ok()?,
    };
    let mut universal = SYSTEMTIME::default();
    // SAFETY: file is a live local and universal a live local the call fills for this one call.
    #[expect(unsafe_code, reason = "the Win32 call admits no safe interface")]
    unsafe { FileTimeToSystemTime(&file, &mut universal) }.ok()?;
    // SAFETY: TIME_ZONE_INFORMATION is all-integer fields so zeroed memory is valid, and zone is a live local the call fills; it answers u32::MAX on failure.
    #[expect(
        unsafe_code,
        reason = "GetTimeZoneInformation's out-parameter needs valid memory"
    )]
    let mut zone = unsafe { std::mem::zeroed::<TIME_ZONE_INFORMATION>() };
    // SAFETY: zone is the zeroed local above, borrowed mutably for this one call.
    #[expect(
        unsafe_code,
        reason = "GetTimeZoneInformation admits no safe interface"
    )]
    let status = unsafe { GetTimeZoneInformation(&mut zone) };
    if status == u32::MAX {
        return None;
    }
    let mut local = SYSTEMTIME::default();
    // SAFETY: zone and universal are live locals filled above and local a live local; the call applies the zone's DST rule to the given moment and fills local once.
    #[expect(unsafe_code, reason = "the Win32 call admits no safe interface")]
    unsafe { SystemTimeToTzSpecificLocalTime(Some(&zone), &universal, &mut local) }.ok()?;
    Some((
        u32::from(local.wHour),
        u32::from(local.wMinute),
        u32::from(local.wSecond),
    ))
}

#[cfg(unix)]
fn local_fields(millis: u64) -> Option<(u32, u32, u32)> {
    let seconds = i64::try_from(millis.checked_div(1_000)?).ok()?;
    // SAFETY: libc::tm is all-integer fields so zeroed memory is valid, and
    // seconds/broken are live locals borrowed for this one call; the call
    // fills broken and answers null on failure.
    #[expect(unsafe_code, reason = "localtime_r's out-parameter needs valid memory")]
    let mut broken: libc::tm = unsafe { std::mem::zeroed() };
    #[expect(unsafe_code, reason = "localtime_r admits no safe interface")]
    let filled = unsafe { libc::localtime_r(&seconds, &mut broken) };
    if filled.is_null() {
        return None;
    }
    Some((
        u32::try_from(broken.tm_hour).ok()?,
        u32::try_from(broken.tm_min).ok()?,
        u32::try_from(broken.tm_sec).ok()?,
    ))
}

#[cfg(not(any(windows, unix)))]
fn local_fields(_millis: u64) -> Option<(u32, u32, u32)> {
    None
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

    #[test]
    fn a_clock_face_is_seconds_since_midnight_and_out_of_range_fields_are_refused() {
        assert_eq!(since_midnight(0, 0, 0), Some(0));
        assert_eq!(since_midnight(1, 1, 1), Some(3_661));
        assert_eq!(since_midnight(23, 59, 59), Some(86_399));
        assert_eq!(since_midnight(24, 0, 0), None);
        assert_eq!(since_midnight(0, 60, 0), None);
        assert_eq!(since_midnight(0, 0, 60), None);
    }

    #[test]
    fn the_machine_agrees_with_itself_across_a_grid_of_moments() {
        for millis in [
            0,
            1,
            86_399_999,
            86_400_000,
            1_785_585_607_000,
            u64::MAX / 4,
        ] {
            let face = seconds_since_midnight(millis);
            assert!(face.is_none_or(|seconds| seconds < 86_400));
        }
    }
}
