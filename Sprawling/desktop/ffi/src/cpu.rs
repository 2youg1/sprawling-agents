// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use winsafe::co;

use crate::cpu_set::{self, CpuSet, Malformed};
use crate::ended::{self, Failure};
use crate::leaf;
use crate::step::Step;

const FIRST_ROOM: usize = 2_048;

const ATTEMPTS: u8 = 4;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Unread {
    Leaf(Failure),

    Malformed(Malformed),
}

pub fn sets() -> Result<Vec<CpuSet>, Unread> {
    let mut room = FIRST_ROOM;
    for _attempt in 0..ATTEMPTS {
        let mut into = vec![0_u8; room];
        let mut found = 0;
        let mut code = co::ERROR::SUCCESS;

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Group {
    pub group: u16,
    pub mask: u64,
}

pub fn thread_group() -> Result<Group, Failure> {
    let mut group = 0;
    let mut mask = 0;
    let mut code = co::ERROR::SUCCESS;

    #[expect(unsafe_code, reason = "the one call that reads the thread's group")]
    let raw = unsafe {
        leaf::sprawling_desktop_thread_group(&raw mut group, &raw mut mask, &raw mut code)
    };
    ended::finished(raw, code)?;
    Ok(Group { group, mask })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Throttling {
    Lifted,

    Absent,
}

pub fn full_speed() -> Result<Throttling, Failure> {
    let mut code = co::ERROR::SUCCESS;

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

pub fn job_share(job: isize, weight: u32, memory: usize) -> Result<(), Failure> {
    let mut code = co::ERROR::SUCCESS;
    let handle = std::ptr::without_provenance_mut::<std::ffi::c_void>(job.cast_unsigned());

    #[expect(unsafe_code, reason = "the one call that sets a job's CPU share")]
    let raw = unsafe { leaf::sprawling_desktop_job_share(handle, weight, memory, &raw mut code) };
    ended::finished(raw, code)
}

#[derive(Debug)]
pub struct JobWatch {
    port: usize,
}

impl JobWatch {
    pub fn attach(job: isize) -> Result<JobWatch, Failure> {
        let mut port = 0;
        let mut code = co::ERROR::SUCCESS;
        let handle = std::ptr::without_provenance_mut::<std::ffi::c_void>(job.cast_unsigned());

        #[expect(unsafe_code, reason = "the one call that attaches a job's port")]
        let raw =
            unsafe { leaf::sprawling_desktop_job_watch(handle, &raw mut port, &raw mut code) };
        ended::finished(raw, code)?;
        Ok(JobWatch { port })
    }

    pub fn memory_hits(&self) -> Result<u32, Failure> {
        let mut hits = 0;
        let mut code = co::ERROR::SUCCESS;

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
