// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::io;
use std::process::{Child, ExitStatus};
use std::time::Duration;

pub(crate) enum TerminateError {
    Kill(io::Error),
    Wait(io::Error),
}

pub(crate) fn terminate(child: &mut Child) -> Result<(), TerminateError> {
    match child.kill() {
        Ok(()) => child.wait().map(|_| ()).map_err(TerminateError::Wait),
        Err(kill) => match child.try_wait() {
            Ok(Some(_)) => Ok(()),
            Ok(None) => Err(TerminateError::Kill(kill)),
            Err(check) => Err(TerminateError::Kill(io::Error::other(format!(
                "{kill}. Checking whether the child exited also failed: {check}"
            )))),
        },
    }
}

#[cfg(windows)]
pub(crate) fn wait(child: &mut Child, timeout: Duration) -> io::Result<Option<ExitStatus>> {
    use wait_timeout::ChildExt as _;

    let rounding = Duration::from_millis(1).saturating_sub(Duration::from_nanos(1));
    child.wait_timeout(timeout.saturating_add(rounding))
}

#[cfg(not(windows))]
#[expect(
    clippy::disallowed_methods,
    reason = "the portable child exit adapter measures its own deadline"
)]
fn wait_with_backoff(child: &mut Child, timeout: Duration) -> io::Result<Option<ExitStatus>> {
    let started = std::time::Instant::now();
    let mut delay = Duration::from_millis(1);
    loop {
        if let Some(status) = child.try_wait()? {
            return Ok(Some(status));
        }
        let remaining = timeout.saturating_sub(started.elapsed());
        if remaining.is_zero() {
            return Ok(None);
        }
        std::thread::sleep(delay.min(remaining));
        delay = delay.saturating_mul(2).min(Duration::from_millis(50));
    }
}

#[cfg(not(any(windows, target_os = "linux")))]
pub(crate) fn wait(child: &mut Child, timeout: Duration) -> io::Result<Option<ExitStatus>> {
    wait_with_backoff(child, timeout)
}

#[cfg(target_os = "linux")]
#[expect(
    clippy::disallowed_methods,
    reason = "the pidfd wait measures the deadline across interrupted system calls"
)]
pub(crate) fn wait(child: &mut Child, timeout: Duration) -> io::Result<Option<ExitStatus>> {
    use rustix::event::{PollFd, PollFlags, Timespec, poll};
    use rustix::io::Errno;
    use rustix::process::{Pid, PidfdFlags, pidfd_open};

    let started = std::time::Instant::now();
    if let Some(status) = child.try_wait()? {
        return Ok(Some(status));
    }
    let raw = i32::try_from(child.id()).map_err(io::Error::other)?;
    let pid = Pid::from_raw(raw).ok_or_else(|| io::Error::other("invalid child pid"))?;
    let fd = match pidfd_open(pid, PidfdFlags::empty()) {
        Ok(fd) => fd,
        Err(Errno::NOSYS | Errno::PERM | Errno::ACCESS) => {
            return wait_with_backoff(child, timeout.saturating_sub(started.elapsed()));
        }
        Err(Errno::SRCH) => return child.try_wait(),
        Err(err) => return Err(err.into()),
    };
    let mut fds = [PollFd::new(&fd, PollFlags::IN)];
    loop {
        let remaining = timeout.saturating_sub(started.elapsed());
        if remaining.is_zero() {
            return child.try_wait();
        }
        let budget = Timespec::try_from(remaining).map_err(io::Error::other)?;
        match poll(&mut fds, Some(&budget)) {
            Ok(_) => return child.try_wait(),
            Err(Errno::INTR) => continue,
            Err(err) => return Err(err.into()),
        }
    }
}
