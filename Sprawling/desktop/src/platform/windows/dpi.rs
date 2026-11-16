// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use windows::Win32::UI::HiDpi::PROCESS_PER_MONITOR_DPI_AWARE;

use super::fault;
use crate::refusal::{Refusal, RefusalCode};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct PhysicalPixels(());

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Awareness {

    PerMonitor,

    Other,
}

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

    #[test]
    fn a_desk_reads_this_desktop_in_physical_pixels() {
        let _desk = super::super::Desk::new();
        assert_eq!(current(), Ok(Awareness::PerMonitor));
    }
}
