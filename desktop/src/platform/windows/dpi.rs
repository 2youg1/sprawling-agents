// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Which DPI awareness this process reads the desktop in.
//!
//! A process that declares nothing is given scaled coordinates on a
//! scaled monitor by `GetWindowRect` and `GetSystemMetrics`, while UI
//! Automation reports physical ones, and a click placed from one lands
//! somewhere else in the other (desktop-SPEC.md section 12.6).

use windows::Win32::UI::HiDpi::{GetProcessDpiAwareness, PROCESS_PER_MONITOR_DPI_AWARE};

use super::fault;
use crate::refusal::Refusal;

/// How this process reads the desktop's coordinates.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Awareness {
    /// Physical pixels on every monitor, which is what every other
    /// module in this arm assumes.
    PerMonitor,
    /// Anything else: scaled somewhere.
    Other,
}

/// The awareness this process has now.
///
/// # Errors
/// Refuses when the operating system will not say.
#[expect(
    unsafe_code,
    reason = "a process's DPI awareness is readable only through the FFI entry point"
)]
pub(super) fn current() -> Result<Awareness, Refusal> {
    // SAFETY: `None` asks about this process, and the call only writes
    // one enum value back through the binding's own out-parameter; no
    // memory of ours is lent to it.
    let awareness = unsafe { GetProcessDpiAwareness(None) }.map_err(|err| {
        fault::win32(
            "say which DPI awareness this process has",
            "start the connector as its own process, `sprawling desktop`",
            &err,
        )
    })?;
    Ok(if awareness == PROCESS_PER_MONITOR_DPI_AWARE {
        Awareness::PerMonitor
    } else {
        Awareness::Other
    })
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

    /// A desk reads the desktop in physical pixels from its first call.
    ///
    /// The awareness belongs to the process, so this holds only where
    /// each test runs in a process of its own, as nextest runs them: under
    /// `cargo test` another test's desk may already have declared it, and
    /// this one would pass without its own desk doing anything.
    #[test]
    fn a_desk_reads_this_desktop_in_physical_pixels() {
        let _desk = super::super::Desk::new();
        assert_eq!(current(), Ok(Awareness::PerMonitor));
    }
}
