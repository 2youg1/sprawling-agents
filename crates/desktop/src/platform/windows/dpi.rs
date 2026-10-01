// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Which DPI awareness this process reads the desktop in.
//!
//! A process that declares nothing is given scaled coordinates on a
//! scaled monitor by `GetWindowRect` and `GetSystemMetrics`, while UI
//! Automation reports physical ones, and a click placed from one lands
//! somewhere else in the other (desktop-SPEC.md section 12.6). So the
//! desk declares per-monitor awareness when it opens, before it reads
//! any coordinate, and holds the proof that it did.

use windows::Win32::UI::HiDpi::PROCESS_PER_MONITOR_DPI_AWARE;

use super::fault;
use crate::refusal::{Refusal, RefusalCode};

/// Proof that this process reads the desktop in physical pixels. Only
/// [`declare`] makes one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct PhysicalPixels(());

/// The awareness this server asks for, as the binding defines it and as
/// the leaf passes it to shcore.
fn per_monitor() -> Result<u32, Refusal> {
    u32::try_from(PROCESS_PER_MONITOR_DPI_AWARE.0).map_err(|_negative| {
        Refusal::new(
            RefusalCode::ToolUnavailable,
            "use the desktop",
            "the binding defines per-monitor awareness as a negative number".to_owned(),
            "this is a defect in this server rather than in the call; report it",
        )
    })
}

/// Makes this process read every monitor in physical pixels.
///
/// A process whose awareness was already set elsewhere cannot change
/// it; that is accepted when what was set is per-monitor awareness.
///
/// # Errors
/// Refuses when this process is left reading scaled coordinates.
pub(super) fn declare() -> Result<PhysicalPixels, Refusal> {
    let Err(refused) = desktop_ffi::dpi::declare(per_monitor()?) else {
        return Ok(PhysicalPixels(()));
    };
    match current()? {
        Awareness::PerMonitor => Ok(PhysicalPixels(())),
        Awareness::Other => Err(Refusal::new(
            RefusalCode::ToolUnavailable,
            "use the desktop",
            format!(
                "this process reads the desktop in scaled coordinates and could not switch to \
                 per-monitor physical pixels: {refused}"
            ),
            "start the connector as its own process, `sprawling desktop`, which declares this \
             before anything else runs",
        )),
    }
}

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
pub(super) fn current() -> Result<Awareness, Refusal> {
    let awareness = desktop_ffi::dpi::awareness().map_err(|err| {
        fault::com(
            "say which DPI awareness this process has",
            "start the connector as its own process, `sprawling desktop`",
            err,
        )
    })?;
    Ok(if awareness == per_monitor()? {
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
