// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! One Windows run's job: the shares it was given, the watch on its
//! memory ceiling and the refusals that watch counted
//! (`crates/runtime/spec/Tools/Exec.lean` D29, D95). [`super::jobs`]
//! routes each command to its run's job; this module owns what the job
//! holds.

use std::num::NonZeroU64;

use super::ceiling::{Mark, Unapplied};
use super::jobs::{RunAffinity, Shares};

/// The weight every run's job holds, from 1 to 9: the same for every
/// run, so the runs share the processors evenly among themselves.
pub(super) const RUN_CPU_WEIGHT: u32 = 5;

/// One run's job, created when its first command joins.
#[derive(Default)]
pub(super) struct RunJob {
    pub(super) retiring: Vec<super::native_windows::RetiringNative>,
    pub(super) job: Option<win32job::Job>,
    pub(super) unjoined: u32,
    pub(super) share: Shares,
    pub(super) affinity: RunAffinity,
    /// The port that counts the job's refusals at its memory ceiling;
    /// present exactly when the ceiling was set (D95).
    pub(super) watch: Option<desktop_ffi::cpu::JobWatch>,
    /// Refusals counted so far: each read takes its messages off the port.
    counted: u64,
    /// Set once a read failed, because the messages it took are gone and
    /// every later count would be short.
    lost: bool,
}

impl RunJob {
    /// The mark of a command that joined this run's job, given the
    /// ceiling `asked` for (D95).
    pub(super) fn mark(&mut self, asked: Option<NonZeroU64>) -> Mark {
        match (asked, self.share, self.watch.is_some()) {
            (None, ..) => Mark::NotAsked,
            (Some(_), Shares::CpuAndMemory { limit }, true) => {
                Mark::entered(limit, self.refusals())
            }
            (Some(limit), ..) => Mark::Unapplied {
                limit,
                why: Unapplied::Refused,
            },
        }
    }

    /// The refusals at the memory ceiling counted since the job was
    /// made; `None` with no watch, or once a read of the port failed.
    pub(super) fn refusals(&mut self) -> Option<u64> {
        let watch = self.watch.as_ref()?;
        if self.lost {
            return None;
        }
        match watch.memory_hits() {
            Ok(hits) => {
                self.counted = self.counted.saturating_add(u64::from(hits));
                Some(self.counted)
            }
            Err(failure) => {
                eprintln!("a run's memory ceiling can no longer be read: {failure:?}");
                self.lost = true;
                None
            }
        }
    }

    /// Puts `child` into this run's job, creating the job on first use
    /// with the shares asked for. `Refused` when the job cannot be made,
    /// `Unjoined` when the process cannot join it.
    pub(super) fn join(
        &mut self,
        child: &std::process::Child,
        shares: Shares,
        affinity: RunAffinity,
    ) -> Result<(), Unapplied> {
        use std::os::windows::io::AsRawHandle;
        let handle =
            isize::try_from(child.as_raw_handle().addr()).map_err(|_beyond| Unapplied::Unjoined)?;
        let job = match self.job.take() {
            Some(job) => job,
            None => {
                let job = win32job::Job::create().map_err(|_refused| Unapplied::Refused)?;
                (self.share, self.watch) = given(&job, shares);
                if let RunAffinity::Mask(mask) = affinity {
                    // Read after the memory limit was set: an empty
                    // record here would erase that limit.
                    let applied = job.query_extended_limit_info().and_then(|mut info| {
                        info.limit_affinity(mask.get());
                        job.set_extended_limit_info(&info)
                    });
                    self.affinity = match applied {
                        Ok(()) => affinity,
                        Err(_refused) => RunAffinity::Os,
                    };
                }
                job
            }
        };
        let joined = job
            .assign_process(handle)
            .map(drop)
            .map_err(|_refused| Unapplied::Unjoined);
        self.job = Some(job);
        joined
    }
}

/// The shares `job` takes of those `asked`, with the watch on its memory
/// ceiling. A job that refuses them still follows the run's processes,
/// and the run is read as unshared. A limit past this process's address
/// space limits nothing, and a ceiling whose refusals cannot be watched
/// is not set (D95): in both cases the job takes the weight alone, and
/// the run's commands report the ceiling as refused (D29).
fn given(job: &win32job::Job, asked: Shares) -> (Shares, Option<desktop_ffi::cpu::JobWatch>) {
    let memory = match asked {
        Shares::Unset => return (Shares::Unset, None),
        Shares::Cpu => return (weigh(job, 0, Shares::Cpu), None),
        Shares::CpuAndMemory { limit } => match usize::try_from(limit.get()) {
            Ok(bytes) => bytes,
            Err(_beyond) => return (weigh(job, 0, Shares::Cpu), None),
        },
    };
    match desktop_ffi::cpu::JobWatch::attach(job.handle()) {
        Ok(watch) => {
            let held = weigh(job, memory, asked);
            let watch = matches!(held, Shares::CpuAndMemory { .. }).then_some(watch);
            (held, watch)
        }
        Err(failure) => {
            eprintln!(
                "a run's memory ceiling is not set, because its refusals cannot be watched: {failure:?}"
            );
            (weigh(job, 0, Shares::Cpu), None)
        }
    }
}

/// Sets the run weight and `memory` (none when zero) on `job`: `held`
/// when the platform takes them, `Unset` when it refuses.
fn weigh(job: &win32job::Job, memory: usize, held: Shares) -> Shares {
    match desktop_ffi::cpu::job_share(job.handle(), RUN_CPU_WEIGHT, memory) {
        Ok(()) => held,
        Err(_refused) => Shares::Unset,
    }
}
