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

pub(crate) mod plan;
pub(crate) mod reading;

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, PoisonError};

use accounting::person::CorePlacement;
use plan::{Left, Plan, Processor, Shape, Topology};
use reading::Unread;

/// Who holds a seat: one per hot thread for as long as it lives.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct Holder(pub(crate) u64);

/// The seats of one plan (the model's `Seats` and `step`): one holder per
/// seat at most, so seated threads never outnumber the plan.
#[derive(Debug)]
pub(crate) struct SeatTable {
    seats: Vec<(Processor, Option<Holder>)>,
}

impl SeatTable {
    pub(crate) fn new(seats: Vec<Processor>) -> Self {
        Self {
            seats: seats.into_iter().map(|seat| (seat, None)).collect(),
        }
    }

    /// Seats `holder` on the first free seat. A holder already seated
    /// keeps its seat; with no free seat the holder gets none.
    pub(crate) fn start(&mut self, holder: Holder) {
        if self.seat_of(holder).is_some() {
            return;
        }
        if let Some((_, taken)) = self.seats.iter_mut().find(|(_, taken)| taken.is_none()) {
            *taken = Some(holder);
        }
    }

    /// Gives `holder`'s seat back; no other seat moves.
    pub(crate) fn exit(&mut self, holder: Holder) {
        for (_, taken) in &mut self.seats {
            if *taken == Some(holder) {
                *taken = None;
            }
        }
    }

    /// The processor `holder` sits at.
    pub(crate) fn seat_of(&self, holder: Holder) -> Option<Processor> {
        self.seats
            .iter()
            .find(|(_, taken)| *taken == Some(holder))
            .map(|(seat, _)| *seat)
    }
}

/// The one table every hot thread of this process sits at, built from
/// the topology read at the first seat.
static TABLE: Mutex<Option<SeatTable>> = Mutex::new(None);
static NEXT_HOLDER: AtomicU64 = AtomicU64::new(0);

/// A hot thread's seat; dropping it, which a thread does as it exits,
/// gives the seat back.
#[derive(Debug)]
pub(crate) struct Seat(Option<Holder>);

impl Seat {
    /// A seat that holds nothing, for a thread built in a test.
    #[cfg(test)]
    pub(crate) fn none() -> Self {
        Self(None)
    }
}

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

/// Takes a seat for the calling thread and makes its processor the
/// thread's soft ideal processor. A refusal is told on stderr, here, and
/// the thread runs where the scheduler puts it.
pub(crate) fn seat_this_thread(name: &'static str) -> Seat {
    let holder = Holder(NEXT_HOLDER.fetch_add(1, Ordering::Relaxed));
    let seat = {
        let mut table = TABLE.lock().unwrap_or_else(PoisonError::into_inner);
        let table = table.get_or_insert_with(first_table);
        table.start(holder);
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
/// of every lane (D46); the seat goes back when the lane drops it.
pub(crate) fn seat_lane() -> Box<dyn std::any::Any> {
    Box::new(seat_this_thread("sprawling-drive"))
}

/// The table of the first seat. With placement off it has no seat;
/// otherwise power throttling is lifted for the whole process (D40), the
/// topology read, and the plan's seats kept where the platform has a soft
/// call to give them to (D41).
fn first_table() -> SeatTable {
    match setting() {
        CorePlacement::Off => SeatTable::new(Vec::new()),
        CorePlacement::Soft => soft_table(),
    }
}

/// The person's `[core] placement`. A setting that does not read is told
/// on stderr once, here, and placement stays on (D47).
fn setting() -> CorePlacement {
    accounting::person::core_placement().unwrap_or_else(|err| {
        eprintln!("CPU placement stays on: {err}");
        CorePlacement::Soft
    })
}

fn soft_table() -> SeatTable {
    if let Err(reason) = full_speed() {
        eprintln!("the city may be power-throttled as background work: {reason}");
    }
    let read = reading::read();
    if let Err(Unread(reason)) = &read {
        eprintln!("the hot threads are placed by the operating system: {reason}");
    }
    let seats = match read.as_ref().map(plan::plan) {
        Ok(Plan::Seats(seats)) if SOFT_CALL => seats,
        Ok(Plan::Seats(_) | Plan::LeftToOs(_)) | Err(_) => Vec::new(),
    };
    SeatTable::new(seats)
}

/// The doctor's line: the topology this machine reports and what the
/// plan does with it, in the User's words (D47).
pub(crate) fn report() -> String {
    match accounting::person::core_placement() {
        Ok(CorePlacement::Off) => {
            "CPU: placement is off ([core] placement = \"none\"); the operating system places every thread"
                .to_owned()
        }
        Ok(CorePlacement::Soft) => describe(&reading::read()),
        Err(err) => format!(
            "{} ([core] placement does not read, so placement stays on: {err})",
            describe(&reading::read())
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
