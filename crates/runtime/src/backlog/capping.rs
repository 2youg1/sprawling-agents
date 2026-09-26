// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The resident memory one dispatched process may hold (runtime-SPEC
//! §8-13-4).
//!
//! On Windows the cap is a job's working-set limit: it bounds what a
//! process keeps resident, not what it commits, and it holds for each
//! process of the job rather than for their sum. A process is placed in
//! its job right after it starts, so the two calls between are a window
//! in which it runs uncapped. Unix has no cap: Linux does not enforce
//! `RLIMIT_RSS`, and setting a limit in the child needs `pre_exec`,
//! which is `unsafe`.

use std::process::Child;

use kernel::AxError;

/// The least a capped process keeps resident: the minimum working set
/// Windows gives a new process, so the cap adds a ceiling and reserves
/// nothing.
#[cfg(windows)]
const MIN_RESIDENT: u64 = 204_800;

/// The most memory one dispatched process may keep resident, in bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResidentCap(u64);

impl ResidentCap {
    /// Half of this machine's physical memory, so that one process
    /// leaves the other half to the core and the person's programs.
    /// `None` when that half is no larger than the least a process
    /// keeps resident, which no machine that runs a city has.
    #[must_use]
    pub fn of_machine(physical: u64) -> Option<ResidentCap> {
        physical
            .checked_div(2)
            .filter(|half| *half > 204_800)
            .map(ResidentCap)
    }

    #[must_use]
    pub fn bytes(self) -> u64 {
        self.0
    }
}

/// Places a process that has just started in a job holding it to
/// `cap`, and hands back the job; dropping it closes the handle while
/// the job and its limit last as long as a process in it does.
///
/// # Errors
/// `E_TOOL_UNAVAILABLE` when the job cannot be made or the process
/// cannot be placed in it.
#[cfg(windows)]
pub(super) fn hold(child: &Child, cap: ResidentCap) -> Result<win32job::Job, AxError> {
    use std::os::windows::io::AsRawHandle;

    let _ = (MIN_RESIDENT, cap);
    let job = win32job::Job::create().map_err(|err| refused(&err.to_string()))?;
    let handle = isize::try_from(child.as_raw_handle().addr())
        .map_err(|err| refused(&err.to_string()))?;
    job.assign_process(handle)
        .map_err(|err| refused(&err.to_string()))?;
    Ok(job)
}

/// Unix has no resident cap; see the module documentation.
#[cfg(unix)]
pub(super) fn hold(_child: &Child, _cap: ResidentCap) -> Result<(), AxError> {
    Ok(())
}

#[cfg(windows)]
fn refused(reason: &str) -> AxError {
    AxError::failure(
        kernel::AxCode::ToolUnavailable,
        "cap a command's memory",
        reason.to_owned(),
    )
    .with_recovery("check that this process may create a job object")
}

#[cfg(all(test, windows))]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn a_dispatched_process_holds_at_most_its_cap_resident() {
        let cap = ResidentCap::of_machine(1 << 30).unwrap();
        let mut child = std::process::Command::new("ping")
            .args(["-n", "30", "127.0.0.1"])
            .stdout(std::process::Stdio::null())
            .spawn()
            .unwrap();
        let job = hold(&child, cap);
        let members = job.as_ref().map(|job| job.query_process_id_list());
        let limits = job.as_ref().map(|job| format!("{:?}", job.query_extended_limit_info()));
        child.kill().unwrap();
        child.wait().unwrap();
        let pid = usize::try_from(child.id()).unwrap();
        assert!(members.unwrap().unwrap().contains(&pid));
        let limits = limits.unwrap();
        assert!(
            limits.contains(&format!("MaximumWorkingSetSize: {}", cap.bytes())),
            "the job holds no working-set cap: {limits}"
        );
    }
}
