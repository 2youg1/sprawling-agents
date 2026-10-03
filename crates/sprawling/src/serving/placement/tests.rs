// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The two properties of `crates/sprawling/spec/Serving/Placement.lean`,
//! checked on the Rust seat table over every ranked table of up to four
//! processors in three ranks, given in any order, and every sequence of
//! five starts and exits of three threads. Bounded and exhaustive rather
//! than sampled: the space is small enough to walk whole, so a defect in
//! it cannot hide behind a seed.

#![allow(clippy::unwrap_used, clippy::indexing_slicing, clippy::panic)]

use super::{Core, Cores, Holder, SeatTable};

const THREADS: u64 = 3;
const RANKS: u8 = 3;
const MOST_CORES: usize = 4;
const EVENTS: usize = 5;

#[derive(Debug, Clone, Copy)]
enum Event {
    Start(Holder),
    Exit(Holder),
}

#[test]
fn placement_keeps_both_properties_on_every_trace() {
    for ranks in every_rank_table() {
        let cores = Cores::ranked(
            ranks
                .iter()
                .zip(0u32..)
                .map(|(&rank, processor)| Core { rank, processor })
                .collect(),
        );
        for trace in every_trace() {
            check_trace(&cores, &trace);
        }
    }
}

fn check_trace(cores: &Cores, trace: &[Event]) {
    let mut table = SeatTable::new(cores.clone());
    for &event in trace {
        let before: Vec<Option<usize>> = holders().map(|h| table.seat_of(h)).collect();
        match event {
            Event::Start(holder) => {
                table.start(holder);
                check_start(cores, &table, &before, holder, trace);
            }
            Event::Exit(holder) => {
                table.exit(holder);
                assert_eq!(
                    table.seat_of(holder),
                    None,
                    "an exit gives the seat back: {trace:?}"
                );
            }
        }
        for (other, was) in holders().zip(&before) {
            let lives = !matches!(event, Event::Exit(h) if h == other);
            if lives && let Some(seat) = was {
                assert_eq!(
                    table.seat_of(other),
                    Some(*seat),
                    "a seat is kept while its thread lives: {cores:?} {trace:?}"
                );
            }
        }
    }
}

/// A start of a thread with no seat takes one when the table has a core,
/// and takes a worse core only when every better core was already seated.
fn check_start(
    cores: &Cores,
    table: &SeatTable,
    before: &[Option<usize>],
    holder: Holder,
    trace: &[Event],
) {
    let index = usize::try_from(holder.0).unwrap();
    if before[index].is_some() {
        return;
    }
    let Some(seat) = table.seat_of(holder) else {
        assert!(
            cores.get(0).is_none(),
            "a non-empty table seats every start: {trace:?}"
        );
        return;
    };
    let rank = cores.get(seat).unwrap().rank;
    for better in (0..MOST_CORES).filter(|&p| cores.get(p).is_some_and(|c| c.rank < rank)) {
        assert!(
            before.contains(&Some(better)),
            "a worse core is taken only when every better one is seated: {cores:?} {trace:?}"
        );
    }
}

fn holders() -> impl Iterator<Item = Holder> {
    (0..THREADS).map(Holder)
}

fn every_rank_table() -> Vec<Vec<u8>> {
    let mut tables = vec![Vec::new()];
    let mut shorter = vec![Vec::new()];
    for _ in 0..MOST_CORES {
        shorter = shorter
            .iter()
            .flat_map(|table: &Vec<u8>| {
                (0..RANKS).map(move |rank| {
                    let mut longer = table.clone();
                    longer.push(rank);
                    longer
                })
            })
            .collect();
        tables.extend(shorter.iter().cloned());
    }
    tables
}

fn every_trace() -> Vec<Vec<Event>> {
    let events: Vec<Event> = holders()
        .flat_map(|h| [Event::Start(h), Event::Exit(h)])
        .collect();
    (0..EVENTS).fold(vec![Vec::new()], |traces, _| {
        traces
            .iter()
            .flat_map(|trace| {
                events.iter().map(move |&event| {
                    let mut longer = trace.clone();
                    longer.push(event);
                    longer
                })
            })
            .collect()
    })
}
