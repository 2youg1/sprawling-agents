// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

mod pinned;
pub(crate) mod plan;
pub(crate) mod reading;
pub(crate) mod seats;

use std::num::NonZeroU64;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock, PoisonError};

use accounting::person::{CorePlacement, MachineMemory};
use plan::{Left, Plan, Processor, Shape, Topology};
use reading::Unread;
use runtime::{PlatformShares, Shares, platform_shares};
pub(crate) use seats::Role;
use seats::{Holder, Seats};

use super::journal::notice;

const MODULE: &str = "bin::serving::placement";

static TABLE: Mutex<Option<Seats>> = Mutex::new(None);
static NEXT_HOLDER: AtomicU64 = AtomicU64::new(0);

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
        notice(
            MODULE,
            &format!(
                "thread {name} runs without an ideal processor (processor {}:{}): {reason}",
                seat.group, seat.number
            ),
        );
    }
    Seat(Some(holder))
}

pub(crate) fn seat_lane() -> Box<dyn std::any::Any> {
    Box::new(seat_this_thread("sprawling-drive", Role::Lane))
}

fn first_table() -> Seats {
    match setting() {
        CorePlacement::Off => Seats::new(Vec::new()),
        CorePlacement::Soft | CorePlacement::SoftShares => soft_table(),
        CorePlacement::Pinned => pinned_table(),
    }
}

fn setting() -> CorePlacement {
    static ARM: OnceLock<CorePlacement> = OnceLock::new();
    *ARM.get_or_init(|| {
        accounting::person::core_placement().unwrap_or_else(|err| {
            notice(MODULE, &format!("CPU placement stays on: {err}"));
            CorePlacement::Soft
        })
    })
}

pub(crate) fn run_shares() -> Shares {
    shares_of(setting(), memory_ceiling())
}

const SYSTEM_KEEPS_BYTES: u64 = 2 * 1024 * 1024 * 1024;

const VIEW_KEEPS_BYTES: u64 = 400 * 1024 * 1024;

pub(crate) fn machine_budget() -> runtime::MachineBudget {
    static BUDGET: OnceLock<runtime::MachineBudget> = OnceLock::new();
    BUDGET
        .get_or_init(|| {
            let setting = accounting::person::machine_memory().unwrap_or_else(|err| {
                notice(
                    MODULE,
                    &format!("the machine's memory budget is sized by the city: {err}"),
                );
                MachineMemory::Auto
            });
            runtime::MachineBudget::new(budget_bytes(
                setting,
                crate::monitor::memory::read().physical,
            ))
        })
        .clone()
}

fn budget_bytes(setting: MachineMemory, physical: u64) -> u64 {
    match setting {
        MachineMemory::Bytes(bytes) => bytes,
        MachineMemory::Auto => physical
            .saturating_sub(SYSTEM_KEEPS_BYTES)
            .saturating_sub(VIEW_KEEPS_BYTES)
            .saturating_sub(u64::try_from(storage::RESIDENT_TOTAL_BYTES).unwrap_or(u64::MAX)),
    }
}

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

fn memory_ceiling() -> Option<NonZeroU64> {
    entered_ceiling().unwrap_or_else(|err| {
        notice(MODULE, &format!("run memory ceiling cannot be read: {err}"));
        None
    })
}

fn entered_ceiling() -> Result<Option<NonZeroU64>, String> {
    accounting::person::read()
        .map(|answer| answer.core.memory_bytes)
        .map_err(|err| err.to_string())
}

fn ceiling_note(arm: CorePlacement, entered: &Result<Option<NonZeroU64>, String>) -> String {
    match (arm, entered) {
        (_, Err(err)) => format!("; the memory ceiling does not read, so none applies: {err}"),
        (CorePlacement::SoftShares, Ok(_)) | (_, Ok(None)) => String::new(),
        (CorePlacement::Off | CorePlacement::Soft | CorePlacement::Pinned, Ok(Some(limit))) => {
            format!(
                "; the entered memory ceiling of {} bytes is not applied: [core] placement is \
                 not \"soft_shares\"",
                limit.get()
            )
        }
    }
}

fn shares_of(arm: CorePlacement, limit: Option<NonZeroU64>) -> Shares {
    match (arm, limit) {
        (CorePlacement::Off, _) => Shares::Unset,
        (CorePlacement::Soft | CorePlacement::Pinned, _) | (CorePlacement::SoftShares, None) => {
            Shares::Cpu
        }
        (CorePlacement::SoftShares, Some(limit)) => Shares::CpuAndMemory { limit },
    }
}

fn lift() {
    if let Err(reason) = full_speed() {
        notice(
            MODULE,
            &format!("the city may be power-throttled as background work: {reason}"),
        );
    }
}

fn seats_of(read: &Result<Topology, Unread>) -> Vec<Processor> {
    match read.as_ref().map(plan::plan) {
        Ok(Plan::Seats(seats)) => seats,
        Ok(Plan::LeftToOs(_)) | Err(_) => Vec::new(),
    }
}

fn planned() -> Vec<Processor> {
    let read = reading::read();
    if let Err(Unread(reason)) = &read {
        notice(
            MODULE,
            &format!("the hot threads are placed by the operating system: {reason}"),
        );
    }
    seats_of(&read)
}

fn soft_table() -> Seats {
    lift();
    Seats::new(if SOFT_CALL { planned() } else { Vec::new() })
}

fn pinned_table() -> Seats {
    lift();
    let seats = planned();
    pinned::take(&seats);
    Seats::new(if SOFT_CALL { seats } else { Vec::new() })
}

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
    let entered = entered_ceiling();
    let limit = entered.as_ref().ok().copied().flatten();
    let runs = runs_share(shares_of(arm, limit), platform_shares());
    let ceiling = ceiling_note(arm, &entered);
    format!("{threads}; {runs}{ceiling}{unread}")
}

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
            "each run's commands share the processors evenly and commit at most {} bytes",
            limit.get()
        ),
    }
}

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

const SOFT_CALL: bool = cfg!(windows);

#[cfg(windows)]
fn full_speed() -> Result<(), String> {
    desktop_ffi::cpu::full_speed()
        .map(|_lifted_or_absent| ())
        .map_err(|err| format!("Windows refused to lift it ({err:?})"))
}

#[cfg(not(windows))]
fn full_speed() -> Result<(), String> {
    Ok(())
}

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
