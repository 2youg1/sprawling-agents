// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! This machine's CPU sets, the calling thread's processor group, this
//! process at full speed, a job's CPU weight and memory limit, and the
//! completion port that reads when the job refused memory, through the leaf (`crates/desktop/ffi/Spec.lean` D4). Nothing here
//! decides what to do with them: the placement plan and the job policy
//! live with their callers.

use winsafe::co;

use crate::cpu_set::{self, CpuSet, Malformed};
use crate::ended::{self, Failure};
use crate::leaf;
use crate::step::Step;

/// The room the first read lends, in bytes: 64 records.
const FIRST_ROOM: usize = 2_048;

/// How many times a short buffer is grown before the read is refused;
/// processors arriving between two reads is the only reason to grow twice.
const ATTEMPTS: u8 = 4;

/// Why the CPU sets were not read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Unread {
    /// The leaf stopped, or answered no step.
    Leaf(Failure),
    /// A record the operating system wrote does not walk.
    Malformed(Malformed),
}

/// Every CPU set of this machine, with the allocation flags read for this
/// process.
///
/// # Errors
/// [`Unread::Leaf`] when the call refused or the answer kept growing;
/// [`Unread::Malformed`] when its records do not walk.
pub fn sets() -> Result<Vec<CpuSet>, Unread> {
    let mut room = FIRST_ROOM;
    for _attempt in 0..ATTEMPTS {
        let mut into = vec![0_u8; room];
        let mut found = 0;
        let mut code = co::ERROR::SUCCESS;
        // SAFETY: `into` is `into.len()` initialised bytes lent to this
        // call alone, and that same length is the capacity the leaf hands
        // the operating system, which writes no further than it.
        #[expect(unsafe_code, reason = "the one call that reads the CPU sets")]
        let raw = unsafe {
            leaf::sprawling_desktop_cpu_sets(
                into.as_mut_ptr(),
                into.len(),
                &raw mut found,
                &raw mut code,
            )
        };
        let step = ended::step(raw).map_err(Unread::Leaf)?;
        if step == Step::Finished {
            into.truncate(found);
            return cpu_set::parse(&into).map_err(Unread::Malformed);
        }
        if step != Step::NoRoom {
            return Err(Unread::Leaf(Failure::At { step, code }));
        }
        room = found.max(room.saturating_add(1));
    }
    Err(Unread::Leaf(Failure::At {
        step: Step::NoRoom,
        code: co::ERROR::INSUFFICIENT_BUFFER,
    }))
}

/// A processor group and the processors of it a thread may run on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Group {
    pub group: u16,
    pub mask: u64,
}

/// The calling thread's group and the processors of it the thread may
/// run on, which a job's affinity limit has already narrowed.
///
/// # Errors
/// [`Failure::At`] `Affinity` with the operating system's reason.
pub fn thread_group() -> Result<Group, Failure> {
    let mut group = 0;
    let mut mask = 0;
    let mut code = co::ERROR::SUCCESS;
    // SAFETY: `group`, `mask` and `code` are live locals the leaf writes
    // one value each into, during this call only.
    #[expect(unsafe_code, reason = "the one call that reads the thread's group")]
    let raw = unsafe {
        leaf::sprawling_desktop_thread_group(&raw mut group, &raw mut mask, &raw mut code)
    };
    ended::finished(raw, code)?;
    Ok(Group { group, mask })
}

/// What lifting power throttling came to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Throttling {
    /// This process's execution speed is no longer throttled.
    Lifted,
    /// This system has no power throttling: it answers
    /// `ERROR_INVALID_FUNCTION` to the class, even to the matching read.
    Absent,
}

/// Opts this process out of EcoQoS: its execution speed is never
/// throttled, so a hybrid scheduler does not treat it as background work.
///
/// # Errors
/// [`Failure::At`] `Throttling` with the operating system's reason.
pub fn full_speed() -> Result<Throttling, Failure> {
    let mut code = co::ERROR::SUCCESS;
    // SAFETY: `code` is a live local the leaf writes once; the state the
    // leaf passes is its own, and the process it changes is this one.
    #[expect(unsafe_code, reason = "the one call that lifts power throttling")]
    let raw = unsafe { leaf::sprawling_desktop_full_speed(&raw mut code) };
    match ended::finished(raw, code) {
        Ok(()) => Ok(Throttling::Lifted),
        Err(Failure::At {
            step: Step::Throttling,
            code: co::ERROR::INVALID_FUNCTION,
        }) => Ok(Throttling::Absent),
        Err(failure) => Err(failure),
    }
}

/// Gives the job behind `job` a weighted CPU share (`weight` from 1 to 9,
/// 5 an even share) and, when `memory` is not zero, a limit on the bytes
/// all its processes commit together.
///
/// # Errors
/// [`Failure::At`] `JobShare` with the operating system's reason, or
/// with none when `job` is zero.
pub fn job_share(job: isize, weight: u32, memory: usize) -> Result<(), Failure> {
    let mut code = co::ERROR::SUCCESS;
    let handle = std::ptr::without_provenance_mut::<std::ffi::c_void>(job.cast_unsigned());
    // SAFETY: `job` is a job handle its owner keeps open for the length of
    // this call (`win32job::Job::handle`), or zero, which the leaf refuses
    // before any call; `code` is a live local the leaf writes once.
    #[expect(unsafe_code, reason = "the one call that sets a job's CPU share")]
    let raw = unsafe { leaf::sprawling_desktop_job_share(handle, weight, memory, &raw mut code) };
    ended::finished(raw, code)
}

/// A completion port attached to one job, read for the times a process
/// of the job was refused memory at the job's limit
/// (`crates/desktop/ffi/Spec.lean` D6). The port lives as long as this
/// value; dropping it closes the port.
#[derive(Debug)]
pub struct JobWatch {
    port: usize,
}

impl JobWatch {
    /// Attaches a new completion port to the job behind `job`.
    ///
    /// # Errors
    /// [`Failure::At`] `JobWatch` with the operating system's reason, or
    /// with none when `job` is zero; nothing is left open either way.
    pub fn attach(job: isize) -> Result<JobWatch, Failure> {
        let mut port = 0;
        let mut code = co::ERROR::SUCCESS;
        let handle = std::ptr::without_provenance_mut::<std::ffi::c_void>(job.cast_unsigned());
        // SAFETY: `job` is a job handle its owner keeps open for the length
        // of this call (`win32job::Job::handle`), or zero, which the leaf
        // refuses before any call; `port` and `code` are live locals the
        // leaf writes once each.
        #[expect(unsafe_code, reason = "the one call that attaches a job's port")]
        let raw =
            unsafe { leaf::sprawling_desktop_job_watch(handle, &raw mut port, &raw mut code) };
        ended::finished(raw, code)?;
        Ok(JobWatch { port })
    }

    /// How many refusals at the memory limit arrived since the last read.
    /// Each message is counted once: a read takes what it counts off the
    /// port.
    ///
    /// # Errors
    /// [`Failure::At`] `JobWatch` with the operating system's reason when
    /// the port could not be read.
    pub fn memory_hits(&self) -> Result<u32, Failure> {
        let mut hits = 0;
        let mut code = co::ERROR::SUCCESS;
        // SAFETY: `self.port` is the port `attach` received and this value
        // still owns, so it stays open for the call; `hits` and `code` are
        // live locals the leaf writes once each.
        #[expect(unsafe_code, reason = "the one call that reads a job's port")]
        let raw = unsafe {
            leaf::sprawling_desktop_job_memory_hits(self.port, &raw mut hits, &raw mut code)
        };
        ended::finished(raw, code)?;
        Ok(hits)
    }
}

impl Drop for JobWatch {
    fn drop(&mut self) {
        let mut code = co::ERROR::SUCCESS;
        // SAFETY: `self.port` is the port `attach` received; this drop is
        // its owner's last use, so no read can follow the close.
        #[expect(unsafe_code, reason = "the one call that closes a job's port")]
        let raw = unsafe { leaf::sprawling_desktop_job_unwatch(self.port, &raw mut code) };
        if let Err(failure) = ended::finished(raw, code) {
            eprintln!("a job's completion port did not close: {failure:?}");
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, reason = "test code")]
mod tests {
    use super::*;

    /// This machine reports at least one CPU set, each with a processor
    /// the calling thread's group can name.
    #[test]
    fn this_machine_reports_its_cpu_sets_and_the_thread_group() {
        let sets = sets().unwrap();
        assert!(!sets.is_empty());
        let group = thread_group().unwrap();
        assert!(group.mask != 0);
        assert!(sets.iter().any(|set| set.group == group.group));
    }

    #[test]
    fn this_process_may_lift_its_own_power_throttling() {
        full_speed().unwrap();
    }

    /// A process of the job asks for more than the limit, and the port
    /// counts that refusal once. The child waits for the gate file, so
    /// it allocates only after it joined the job.
    #[test]
    #[expect(clippy::disallowed_methods, reason = "test fixture (child D4)")]
    fn a_watched_job_counts_a_refusal_at_its_memory_limit() {
        use std::os::windows::io::AsRawHandle;
        let job = win32job::Job::create().unwrap();
        job_share(job.handle(), 5, 512 << 20).unwrap();
        let watch = JobWatch::attach(job.handle()).unwrap();
        assert_eq!(watch.memory_hits().unwrap(), 0);
        let open = std::env::temp_dir().join(format!("sprawling-job-watch-{}", std::process::id()));
        let mut child = std::process::Command::new("powershell.exe")
            .args([
                "-NoProfile",
                "-NonInteractive",
                "-Command",
                "while (-not (Test-Path -LiteralPath $env:GATE)) { Start-Sleep -Milliseconds 20 };                  try { $b = New-Object byte[] 1073741824; exit 0 } catch { exit 7 }",
            ])
            .env("GATE", &open)
            .spawn()
            .unwrap();
        job.assign_process(isize::try_from(child.as_raw_handle().addr()).unwrap())
            .unwrap();
        std::fs::write(&open, b"").unwrap();
        let status = child.wait().unwrap();
        std::fs::remove_file(&open).unwrap();
        assert_eq!(status.code(), Some(7));
        assert!(watch.memory_hits().unwrap() >= 1);
        assert_eq!(watch.memory_hits().unwrap(), 0);
        assert!(matches!(
            JobWatch::attach(0),
            Err(Failure::At {
                step: Step::JobWatch,
                code: co::ERROR::SUCCESS
            })
        ));
    }

    #[test]
    fn a_job_takes_a_weighted_share_and_a_memory_limit() {
        let job = win32job::Job::create().unwrap();
        job_share(job.handle(), 5, 1 << 30).unwrap();
        assert_eq!(
            job_share(0, 5, 0),
            Err(Failure::At {
                step: Step::JobShare,
                code: co::ERROR::SUCCESS
            })
        );
    }
}
