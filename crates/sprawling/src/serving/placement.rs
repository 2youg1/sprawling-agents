// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The seat each hot thread takes from the placement plan: the first
//! free one, or none when every seat is taken, kept until the thread
//! exits, and handed to the platform as a soft ideal processor so the
//! scheduler stays in charge and moves the thread on at once when that
//! core is busy (`crates/sprawling/spec/Serving/Placement.lean`, D41 and
//! D45-D47). The seats are the plan `plan` makes of the topology
//! `reading` reports; `tests` checks this table against the seat model.
//! This module is also the one place that decides what each arm of the
//! person's `[core] placement` turns on (D47), the runs' shares included.

mod pinned;
pub(crate) mod plan;
pub(crate) mod reading;
pub(crate) mod seats;

use std::num::NonZeroU64;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock, PoisonError};

use accounting::person::CorePlacement;
use plan::{Left, Plan, Processor, Shape, Topology};
use reading::Unread;
use runtime::{PlatformShares, Shares, platform_shares};
pub(crate) use seats::Role;
use seats::{Holder, Seats};

/// The one seat table every hot thread of this process sits at, built
/// from the topology read at the first seat.
static TABLE: Mutex<Option<Seats>> = Mutex::new(None);
static NEXT_HOLDER: AtomicU64 = AtomicU64::new(0);

/// A hot thread's seat; dropping it, which a thread does as it exits,
/// gives the seat back.
#[derive(Debug)]
pub(crate) struct Seat(Option<Holder>);

impl Drop for Seat {
    fn drop(&mut self) {
        if let Some(holder) = self.0 {
            let mut table = TABLE.lock().unwrap_or_else(PoisonError::into_inner);
            if let Some(table) = table.as_mut() {
                table.exit(holder);
            }
        }
    }
}

/// Takes a seat of `role`'s pool for the calling thread and makes its
/// processor the thread's soft ideal processor. A refusal is told on
/// stderr, here, and the thread runs where the scheduler puts it.
pub(crate) fn seat_this_thread(name: &'static str, role: Role) -> Seat {
    let holder = Holder(NEXT_HOLDER.fetch_add(1, Ordering::Relaxed));
    let seat = {
        let mut table = TABLE.lock().unwrap_or_else(PoisonError::into_inner);
        let table = table.get_or_insert_with(first_table);
        table.start(holder, role);
        table.seat_of(holder)
    };
    if let Some(seat) = seat
        && let Err(reason) = prefer(seat)
    {
        eprintln!(
            "thread {name} runs without an ideal processor (processor {}:{}): {reason}",
            seat.group, seat.number
        );
    }
    Seat(Some(holder))
}

/// A driving lane's seat, through the hook `accounting` calls at the top
/// of every lane (D46); the seat goes back when the lane drops it, so
/// the lane pool turns over from one run to the next.
pub(crate) fn seat_lane() -> Box<dyn std::any::Any> {
    Box::new(seat_this_thread("sprawling-drive", Role::Lane))
}

/// The table of the first seat. With placement off it has no seat;
/// otherwise power throttling is lifted for the whole process (D40), the
/// topology read, and the plan's seats kept where the platform has a soft
/// call to give them to (D41).
fn first_table() -> Seats {
    match setting() {
        CorePlacement::Off => Seats::new(Vec::new()),
        CorePlacement::Soft | CorePlacement::SoftShares => soft_table(),
        CorePlacement::Pinned => pinned_table(),
    }
}

/// The person's `[core] placement`, read once for the process. A setting
/// that does not read is told on stderr once, here, and placement stays
/// on (D47).
fn setting() -> CorePlacement {
    static ARM: OnceLock<CorePlacement> = OnceLock::new();
    *ARM.get_or_init(|| {
        accounting::person::core_placement().unwrap_or_else(|err| {
            eprintln!("CPU placement stays on: {err}");
            CorePlacement::Soft
        })
    })
}

/// The shares each run's commands ask for under the person's arm (D47),
/// handed to the runtime through `Hands`.
pub(crate) fn run_shares() -> Shares {
    shares_of(setting(), memory_ceiling())
}

/// The pinned arm's remaining processors, saved before the core joins
/// its job. Initialise the same table the first seat uses, so assembly
/// before that seat still receives the saved mask.
pub(crate) fn run_affinity() -> runtime::backlog::RunAffinity {
    if setting() != CorePlacement::Pinned {
        return runtime::backlog::RunAffinity::Os;
    }
    TABLE
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .get_or_insert_with(first_table);
    pinned::run_affinity()
}

/// Reads only a User-entered ceiling; invalid settings are reported.
fn memory_ceiling() -> Option<NonZeroU64> {
    match accounting::person::read() {
        Ok(answer) => answer.core.memory_bytes,
        Err(err) => {
            eprintln!("run memory ceiling cannot be read: {err}");
            None
        }
    }
}

/// Only soft_shares requests the explicitly entered memory ceiling.
fn shares_of(arm: CorePlacement, limit: Option<NonZeroU64>) -> Shares {
    match (arm, limit) {
        (CorePlacement::Off, _) => Shares::Unset,
        (CorePlacement::Soft | CorePlacement::Pinned, _) | (CorePlacement::SoftShares, None) => {
            Shares::Cpu
        }
        (CorePlacement::SoftShares, Some(limit)) => Shares::CpuAndMemory { limit },
    }
}

/// Lifts this process out of the throttling background work gets,
/// telling the reason once when the platform refuses (D40).
fn lift() {
    if let Err(reason) = full_speed() {
        eprintln!("the city may be power-throttled as background work: {reason}");
    }
}

/// The seats of a reading: empty when the plan is left to the operating
/// system.
fn seats_of(read: &Result<Topology, Unread>) -> Vec<Processor> {
    match read.as_ref().map(plan::plan) {
        Ok(Plan::Seats(seats)) => seats,
        Ok(Plan::LeftToOs(_)) | Err(_) => Vec::new(),
    }
}

/// The plan for a topology read now, with the reason there is none told
/// once when the topology cannot be read (D45).
fn planned() -> Vec<Processor> {
    let read = reading::read();
    if let Err(Unread(reason)) = &read {
        eprintln!("the hot threads are placed by the operating system: {reason}");
    }
    seats_of(&read)
}

/// The seats a hot thread may prefer: none where the platform has no soft
/// ideal-processor call to hand one to (D41, Windows alone).
fn soft_table() -> Seats {
    lift();
    Seats::new(if SOFT_CALL { planned() } else { Vec::new() })
}

/// The pinned arm's table: the soft seats where the platform has them,
/// and the hard affinity taking this process (D41, D49). Where the
/// platform has no hard call, or the plan names no processor, the arm says
/// so and the seats stay soft. One reading serves both, so the mask and
/// the seats cannot name different processors.
fn pinned_table() -> Seats {
    lift();
    let seats = planned();
    pinned::take(&seats);
    Seats::new(if SOFT_CALL { seats } else { Vec::new() })
}

/// The doctor's line: the topology this machine reports, what the plan
/// does with it, and what each run's commands share, in the User's words
/// (D47).
pub(crate) fn report() -> String {
    let (arm, unread) = match accounting::person::core_placement() {
        Ok(arm) => (arm, String::new()),
        Err(err) => (
            CorePlacement::Soft,
            format!(" ([core] placement does not read, so placement stays on: {err})"),
        ),
    };
    let threads = match arm {
        CorePlacement::Off => {
            "CPU: placement is off ([core] placement = \"none\"); the operating system places every thread"
                .to_owned()
        }
        CorePlacement::Soft | CorePlacement::SoftShares => describe(&reading::read()),
        CorePlacement::Pinned => {
            let read = reading::read();
            format!("{} ({})", describe(&read), pinned::clause(&seats_of(&read)))
        }
    };
    let runs = runs_share(shares_of(arm, memory_ceiling()), platform_shares());
    format!("{threads}; {runs}{unread}")
}

/// What each run's commands share on this machine, given what the arm
/// asks for and what the platform can set: Windows sets both halves on
/// the run's job, macOS has the CPU half alone (`taskpolicy`), and
/// Linux sets both where its cgroup is delegated, nothing where it is
/// not (D29, D33).
fn runs_share(asked: Shares, platform: PlatformShares) -> String {
    const ALONE: &str = "runs' commands compete thread by thread";
    const NOT_DELEGATED: &str = "runs' commands compete thread by thread and run below the core: \
                                 the cgroup is not delegated";
    match (asked, platform) {
        (Shares::Unset, _) => ALONE.to_owned(),
        (_, PlatformShares::None) if cfg!(target_os = "linux") => NOT_DELEGATED.to_owned(),
        (_, PlatformShares::None) => ALONE.to_owned(),
        (Shares::Cpu, PlatformShares::Cpu | PlatformShares::CpuAndMemory) => {
            "each run's commands share the processors evenly".to_owned()
        }
        (Shares::CpuAndMemory { .. }, PlatformShares::Cpu) => {
            "each run's commands share the processors evenly; this platform sets no memory limit"
                .to_owned()
        }
        (Shares::CpuAndMemory { limit }, PlatformShares::CpuAndMemory) => format!(
            "each run's commands share the processors evenly and commit at most {} MiB",
            limit.get() / (1 << 20)
        ),
    }
}

/// [`report`] for a reading already made.
pub(crate) fn describe(read: &Result<Topology, Unread>) -> String {
    let topology = match read {
        Ok(topology) => topology,
        Err(Unread(reason)) => {
            return format!("CPU: topology unread ({reason}); left to the operating system");
        }
    };
    let shape = topology.shape();
    let outcome = match plan::plan(topology) {
        Plan::Seats(seats) if SOFT_CALL => format!(
            "hot threads prefer the {} fastest cores, one thread each",
            seats.len()
        ),
        Plan::Seats(_) => {
            "this platform has no placement call; its scheduler places threads".to_owned()
        }
        Plan::LeftToOs(Left::OneClass) if shape.caches > 1 => {
            "left to the operating system and its cache steering".to_owned()
        }
        Plan::LeftToOs(Left::OneClass) => "left to the operating system".to_owned(),
        Plan::LeftToOs(Left::Inconsistent) => {
            "the processors report an inconsistent topology; left to the operating system"
                .to_owned()
        }
        Plan::LeftToOs(Left::NothingUsable) => {
            "this process may use none of them; left to the operating system".to_owned()
        }
    };
    format!("CPU: {}; {outcome}", classes(&shape))
}

fn classes(shape: &Shape) -> String {
    let caches = match shape.caches {
        0 | 1 => String::new(),
        many => format!(" in {many} cache groups"),
    };
    let usable = if shape.usable == shape.logical {
        String::new()
    } else {
        format!(
            " (this process may use {} of {} logical processors)",
            shape.usable, shape.logical
        )
    };
    let kinds: Vec<String> = shape
        .classes
        .iter()
        .zip(class_names(shape.classes.len()))
        .map(|(class, name)| format!("{} {name}cores ({} threads)", class.cores, class.logical))
        .collect();
    match kinds.len() {
        0 | 1 => format!("one class, {}{caches}{usable}", kinds.join("")),
        count => format!("{count} classes, {}{caches}{usable}", kinds.join(", ")),
    }
}

/// The words for each class, fastest first.
fn class_names(count: usize) -> Vec<&'static str> {
    match count {
        0 | 1 => vec![""],
        2 => vec!["performance ", "efficiency "],
        3 => vec!["performance ", "efficiency ", "low-power "],
        many => std::iter::once("fastest ")
            .chain(std::iter::repeat_n("slower ", many.saturating_sub(1)))
            .collect(),
    }
}

/// Whether the platform has a soft ideal-processor call to hand a seat
/// to (D41): Windows alone.
const SOFT_CALL: bool = cfg!(windows);

/// A Windows without power throttling has nothing to lift (D40).
#[cfg(windows)]
fn full_speed() -> Result<(), String> {
    desktop_ffi::cpu::full_speed()
        .map(|_lifted_or_absent| ())
        .map_err(|err| format!("Windows refused to lift it ({err:?})"))
}

/// macOS and Linux throttle no process for running in the background,
/// so there is nothing to lift (D40).
#[cfg(not(windows))]
fn full_speed() -> Result<(), String> {
    Ok(())
}

/// The seat as the calling thread's ideal processor, when the thread is
/// in the seat's processor group: an ideal processor is named inside
/// the thread's own group.
#[cfg(windows)]
fn prefer(seat: Processor) -> Result<(), String> {
    let group = desktop_ffi::cpu::thread_group()
        .map_err(|err| format!("its processor group is unreadable ({err:?})"))?;
    if group.group != seat.group {
        return Err(format!("it runs in processor group {}", group.group));
    }
    thread_priority::windows::set_current_thread_ideal_processor(seat.number)
        .map(|_previous| ())
        .map_err(|err| err.to_string())
}

#[cfg(not(windows))]
fn prefer(_seat: Processor) -> Result<(), String> {
    Ok(())
}

#[cfg(test)]
mod tests;
