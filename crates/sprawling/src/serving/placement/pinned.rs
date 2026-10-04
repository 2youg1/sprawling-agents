// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The fourth comparison arm: the plan's processors held by hard
//! affinity (`crates/sprawling/spec/Serving/Placement.lean`, D41 and
//! D49). It is built to be measured later, and it is honest about what
//! each platform can do: a Windows Job Object holds this process to the
//! plan's mask, Linux has the call but not through a safe Rust interface
//! so the whole binary is started under `taskset -c` from outside, and
//! macOS has no call at all.
//!
//! A job's affinity limit is inherited by every process in the job, so
//! the harness job lets children leave it: a run's own Job Object is
//! free to take the processors the plan did not.

use super::plan::Processor;

/// What the arm does with this machine, or why it does nothing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Did {
    /// This process is in a Job Object whose affinity limit is this
    /// mask of this processor group.
    #[cfg_attr(
        not(windows),
        expect(
            dead_code,
            reason = "only Windows takes the process into a job whose affinity limit this is"
        )
    )]
    Pinned {
        group: u16,
        mask: u64,
        runs: runtime::backlog::RunAffinity,
    },
    /// The call is external here: the whole binary is started under this
    /// `taskset -c` list by whoever runs the measurement.
    #[cfg_attr(
        not(target_os = "linux"),
        expect(dead_code, reason = "only Linux answers with a taskset list")
    )]
    External { list: String },
    /// No mechanism, or one that refused, and the reason the doctor's
    /// line carries.
    Nothing { reason: String },
}

/// The one taking of this process, and the answer every later reader
/// gets: the first seat takes it, and the doctor reads it back.
static TAKEN: std::sync::OnceLock<Did> = std::sync::OnceLock::new();

/// The Job Object this process was taken into, kept open for as long as
/// the process lives: closing the last handle to a job destroys it, and
/// with it the affinity limit.
#[cfg(windows)]
static JOB: std::sync::OnceLock<win32job::Job> = std::sync::OnceLock::new();

/// Takes this process into the arm's mechanism, once, and says on stderr
/// what happened when that is not [`Did::Pinned`]; the answer is kept for
/// [`clause`].
pub(crate) fn take(seats: &[Processor]) -> Did {
    let did = TAKEN.get_or_init(|| acted(seats)).clone();
    if !matches!(did, Did::Pinned { .. }) {
        eprintln!(
            "[core] placement = \"pinned\": {}",
            words(&did, seats.len())
        );
    }
    did
}

/// The run request saved before this process's affinity was narrowed.
pub(crate) fn run_affinity() -> runtime::backlog::RunAffinity {
    match TAKEN.get() {
        Some(Did::Pinned { runs, .. }) => *runs,
        Some(Did::External { .. } | Did::Nothing { .. }) | None => {
            runtime::backlog::RunAffinity::Os
        }
    }
}

/// The arm's clause for the doctor's line (D47): what was done, or what
/// would be done now.
pub(crate) fn clause(seats: &[Processor]) -> String {
    let did = match TAKEN.get() {
        Some(did) => did.clone(),
        None => would(seats),
    };
    words(&did, seats.len())
}

fn words(did: &Did, cores: usize) -> String {
    match did {
        Did::Pinned { .. } => format!(
            "hard affinity: the process is held to the {cores} planned cores by a Job Object"
        ),
        Did::External { list } => format!(
            "hard affinity: start the binary under `taskset -c {list}`; this arm sets nothing inside the process"
        ),
        Did::Nothing { reason } => format!("hard affinity: nothing is pinned ({reason})"),
    }
}

/// The taking that needs the platform, or the answer of [`would`] when
/// there is nothing to take.
#[cfg(windows)]
fn acted(seats: &[Processor]) -> Did {
    let (group, mask, runs) = match would(seats) {
        Did::Pinned { group, mask, runs } => (group, mask, runs),
        external @ Did::External { .. } | external @ Did::Nothing { .. } => return external,
    };
    let Ok(wide) = usize::try_from(mask) else {
        return Did::Nothing {
            reason: "the plan's mask does not fit this build's word".to_owned(),
        };
    };
    let mut info = win32job::ExtendedLimitInfo::default();
    info.limit_affinity(wide).limit_silent_breakaway_ok();
    let job = match win32job::Job::create_with_limit_info(&info) {
        Ok(job) => job,
        Err(err) => {
            return Did::Nothing {
                reason: format!("a job with that affinity limit was refused ({err})"),
            };
        }
    };
    if let Err(err) = job.assign_current_process() {
        return Did::Nothing {
            reason: format!("this process could not join the job ({err})"),
        };
    }
    if JOB.set(job).is_err() {
        return Did::Nothing {
            reason: "the process was already taken".to_owned(),
        };
    }
    if runs == runtime::backlog::RunAffinity::Os {
        eprintln!("[core] placement = \"pinned\": no remaining processors for run affinity");
    }
    Did::Pinned { group, mask, runs }
}

/// Nothing to take: the platform call is external (`taskset`) or absent.
#[cfg(not(windows))]
fn acted(seats: &[Processor]) -> Did {
    would(seats)
}

/// What the arm asks of this platform for these seats, without changing
/// anything.
fn would(seats: &[Processor]) -> Did {
    if seats.is_empty() {
        return Did::Nothing {
            reason: "the plan leaves every processor to the operating system".to_owned(),
        };
    }
    #[cfg(windows)]
    {
        let group = match desktop_ffi::cpu::thread_group() {
            Ok(group) => group,
            Err(err) => {
                return Did::Nothing {
                    reason: format!("this thread's processor group is unreadable ({err:?})"),
                };
            }
        };
        // An ideal processor and a job's affinity limit are both named
        // inside one group, and a limit cannot exceed what this process
        // may already use.
        let mask = seats
            .iter()
            .filter(|seat| seat.group == group.group)
            .filter_map(|seat| 1_u64.checked_shl(seat.number))
            .fold(0, |mask, bit| mask | bit)
            & group.mask;
        if mask == 0 {
            return Did::Nothing {
                reason: "the plan names no processor of this thread's group".to_owned(),
            };
        }
        Did::Pinned {
            group: group.group,
            mask,
            runs: remaining(group.mask, mask),
        }
    }
    #[cfg(target_os = "linux")]
    {
        Did::External {
            list: seats
                .iter()
                .map(|seat| seat.number.to_string())
                .collect::<Vec<_>>()
                .join(","),
        }
    }
    #[cfg(not(any(windows, target_os = "linux")))]
    {
        Did::Nothing {
            reason: "this platform has no hard-affinity call".to_owned(),
        }
    }
}

/// The available processors outside the core plan; an empty or
/// unrepresentable remainder asks for no affinity (D49).
#[cfg(any(windows, test))]
fn remaining(available: u64, core: u64) -> runtime::backlog::RunAffinity {
    match usize::try_from(available & !core) {
        Ok(mask) => match std::num::NonZeroUsize::new(mask) {
            Some(mask) => runtime::backlog::RunAffinity::Mask(mask),
            None => runtime::backlog::RunAffinity::Os,
        },
        Err(_beyond) => runtime::backlog::RunAffinity::Os,
    }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    reason = "test code"
)]
mod tests {
    use super::*;

    /// The core reads back its planned mask, and a run child reports the
    /// remaining mask after enrolment, independently of the run window.
    #[cfg(windows)]
    #[test]
    fn the_job_reports_the_mask_the_arm_asked_for() {
        let group = desktop_ffi::cpu::thread_group().unwrap();
        let seats: Vec<Processor> = (0..64)
            .filter(|&number| group.mask >> number & 1 == 1)
            .take(2)
            .map(|number| Processor {
                group: group.group,
                number,
            })
            .collect();
        assert_eq!(seats.len(), 2, "this machine lends two processors");
        let did = take(&seats);
        let Did::Pinned { mask, runs, .. } = did else {
            panic!("the arm pinned nothing: {did:?}");
        };
        assert_eq!(desktop_ffi::cpu::thread_group().unwrap().mask, mask);
        assert_eq!(run_affinity(), runs);
        let runtime::backlog::RunAffinity::Mask(run_mask) = runs else {
            panic!("this machine lends processors outside the two core seats");
        };
        let backlog =
            runtime::Backlog::with_window(runtime::PollBudget::new(1, 1)).with_affinity(runs);
        let scratch = tempfile::tempdir().unwrap();
        let gate = scratch.path().join("gate");
        let report = scratch.path().join("affinity");
        let mut child = std::process::Command::new("powershell.exe");
        child.args([
            "-NoProfile",
            "-NonInteractive",
            "-Command",
            "while (-not (Test-Path -LiteralPath $env:R05_GATE)) { Start-Sleep -Milliseconds 20 }; \
             [IO.File]::WriteAllText($env:R05_REPORT + '.pending', \
             [Diagnostics.Process]::GetCurrentProcess().ProcessorAffinity.ToInt64().ToString()); \
             [IO.File]::Move($env:R05_REPORT + '.pending', $env:R05_REPORT)",
        ]);
        child
            .env("R05_GATE", &gate)
            .env("R05_REPORT", &report)
            .current_dir(std::env::temp_dir());
        let owner = kernel::RunId::from_bytes([10; 16]);
        let scope = kernel::Address::parse("vault/room1").unwrap();
        let started = backlog.run(owner, &scope, "remaining processors".to_owned(), child);
        let polls = 3000;
        let interval = std::time::Duration::from_millis(20);
        let observed = (|| -> std::io::Result<(String, runtime::Finished)> {
            if let Err(err) = &started {
                return Err(std::io::Error::other(format!(
                    "start affinity reader: {err}"
                )));
            }
            std::fs::write(&gate, b"ready")?;
            let mut observed = None;
            for _ in 0..polls {
                match std::fs::read_to_string(&report) {
                    Ok(value) => {
                        observed = Some(value);
                        break;
                    }
                    Err(err) if err.kind() == std::io::ErrorKind::NotFound => {}
                    Err(err) => return Err(err),
                }
                std::thread::sleep(interval);
            }
            let observed = observed.ok_or_else(|| {
                std::io::Error::new(
                    std::io::ErrorKind::TimedOut,
                    format!("affinity reader published no report after {polls} polls: {started:?}"),
                )
            })?;
            for _ in 0..polls {
                if let Some(finished) = backlog
                    .harvest(owner)
                    .map_err(|err| std::io::Error::other(err.to_string()))?
                    .into_iter()
                    .next()
                {
                    return Ok((observed, finished));
                }
                std::thread::sleep(interval);
            }
            Err(std::io::Error::new(
                std::io::ErrorKind::TimedOut,
                format!("affinity reader did not exit after {polls} polls"),
            ))
        })();
        backlog.release(owner);
        let reaped = (|| -> Result<(), String> {
            for _ in 0..polls {
                backlog.harvest(owner).map_err(|err| err.to_string())?;
                if backlog
                    .standing(&scope)
                    .map_err(|err| err.to_string())?
                    .is_empty()
                {
                    return Ok(());
                }
                std::thread::sleep(interval);
            }
            Err(format!(
                "released affinity reader remains in the backlog after {polls} polls"
            ))
        })();
        let removed = scratch.close();
        reaped.expect("release and harvest the affinity reader");
        removed.expect("remove the affinity reader's gate and report files");
        let (observed, finished) =
            observed.expect("read the child process's affinity report and exit");
        assert_eq!(
            finished.exit,
            runtime::Exit::Ended { code: 0 },
            "{}",
            finished.stderr
        );
        assert_eq!(observed.trim().parse::<usize>().unwrap(), run_mask.get());
        assert_eq!(u64::try_from(run_mask.get()).unwrap() & mask, 0);
    }

    /// Linux has the call but not through a safe interface, so the arm
    /// asks to be started under `taskset` rather than doing nothing
    /// quietly.
    #[cfg(target_os = "linux")]
    #[test]
    fn linux_asks_to_be_started_under_taskset() {
        let seats = vec![
            Processor {
                group: 0,
                number: 2,
            },
            Processor {
                group: 0,
                number: 4,
            },
        ];
        assert_eq!(
            would(&seats),
            Did::External {
                list: "2,4".to_owned()
            }
        );
    }

    /// A platform with no mechanism says so rather than silently doing
    /// nothing.
    #[cfg(not(any(windows, target_os = "linux")))]
    #[test]
    fn a_platform_without_the_call_says_so() {
        let seats = vec![Processor {
            group: 0,
            number: 0,
        }];
        let Did::Nothing { reason } = would(&seats) else {
            panic!("this platform is expected to have no hard-affinity call");
        };
        assert!(reason.contains("no hard-affinity call"), "{reason}");
        assert!(clause(&seats).contains("nothing is pinned"));
    }

    /// The mask partitions every small available set, including an
    /// empty remainder and planned bits outside the available set.
    #[test]
    fn run_processors_are_available_and_outside_the_core_plan() {
        for available in 0_u64..256 {
            for core in 0_u64..256 {
                match remaining(available, core) {
                    runtime::backlog::RunAffinity::Os => assert_eq!(available & !core, 0),
                    runtime::backlog::RunAffinity::Mask(mask) => {
                        let runs = u64::try_from(mask.get()).unwrap();
                        assert_eq!(runs & core, 0);
                        assert_eq!(runs & !available, 0);
                        assert_eq!(runs | (available & core), available);
                    }
                }
            }
        }
    }

    /// No plan means nothing to pin, said rather than hidden.
    #[test]
    fn an_empty_plan_pins_nothing_and_says_so() {
        let Did::Nothing { reason } = would(&[]) else {
            panic!("an empty plan asks nothing of any platform");
        };
        assert!(reason.contains("leaves every processor"), "{reason}");
    }
}
