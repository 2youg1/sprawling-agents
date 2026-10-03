// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The seat model of `crates/sprawling/spec/Serving/Placement.lean`,
//! checked on the Rust seat table over every plan of up to five seats and
//! every sequence of six starts and exits of four threads, the two
//! serial ones and the two lanes. Bounded and exhaustive rather than
//! sampled: the space is small enough to walk whole, so a defect in it
//! cannot hide behind a seed.

#![allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    clippy::panic,
    clippy::arithmetic_side_effects
)]

use std::collections::BTreeSet;

use super::plan::{Left, Plan, Processor, Topology};
use super::reading::Unread;
use super::seats::{Holder, Role, SERIAL_SEATS, Seats};
use super::describe;

const THREADS: u64 = 4;
const MOST_SEATS: u32 = 5;
const EVENTS: usize = 6;

/// The role of the thread `holder` names: the city has `SERIAL_SEATS`
/// serial threads, and every thread after them is a run's lane.
fn role(holder: Holder) -> Role {
    match usize::try_from(holder.0).unwrap_or(usize::MAX) < SERIAL_SEATS {
        true => Role::Serial,
        false => Role::Lane,
    }
}

#[derive(Debug, Clone, Copy)]
enum Event {
    Start(Holder),
    Exit(Holder),
}

#[test]
fn the_seat_table_keeps_every_property_on_every_trace() {
    for count in 0..=MOST_SEATS {
        let plan: Vec<Processor> = (0..count)
            .map(|number| Processor {
                group: 0,
                number: number * 2,
            })
            .collect();
        for trace in every_trace() {
            check_trace(&plan, &trace);
        }
    }
}

fn check_trace(plan: &[Processor], trace: &[Event]) {
    let mut table = Seats::new(plan.to_vec());
    for &event in trace {
        let before: Vec<Option<Processor>> = holders().map(|h| table.seat_of(h)).collect();
        match event {
            Event::Start(holder) => {
                table.start(holder, role(holder));
                check_start(&table, plan, &before, holder);
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
        // every_trace_keeps_the_table_good: one holder per seat, every seat in the plan.
        let seated: Vec<Processor> = holders().filter_map(|h| table.seat_of(h)).collect();
        let distinct: BTreeSet<Processor> = seated.iter().copied().collect();
        assert_eq!(
            distinct.len(),
            seated.len(),
            "one seat holds one thread: {plan:?} {trace:?}"
        );
        assert!(seated.iter().all(|seat| plan.contains(seat)));
        // a_seat_is_kept_while_its_thread_lives
        for (other, was) in holders().zip(&before) {
            let lives = !matches!(event, Event::Exit(h) if h == other);
            if lives && let Some(seat) = was {
                assert_eq!(
                    table.seat_of(other),
                    Some(*seat),
                    "a seat is kept: {plan:?} {trace:?}"
                );
            }
        }
    }
}

/// a_start_takes_the_first_free_seat, and
/// a_start_is_left_to_the_os_only_when_every_seat_is_taken: the seat
/// comes from the holder's own pool, and the first free one there.
fn check_start(table: &Seats, plan: &[Processor], before: &[Option<Processor>], holder: Holder) {
    let index = usize::try_from(holder.0).unwrap();
    if before[index].is_some() {
        return;
    }
    let pool = pool_of(plan, role(holder));
    let first_free = pool
        .iter()
        .find(|seat| !before.contains(&Some(**seat)))
        .copied();
    assert_eq!(
        table.seat_of(holder),
        first_free,
        "the first free seat of its own pool or none: {plan:?} {before:?}"
    );
}

/// The seats of `role`'s pool, in the order the table holds them: lanes
/// take the plan's processors from the front, serial threads its last
/// `SERIAL_SEATS`.
fn pool_of(plan: &[Processor], role: Role) -> Vec<Processor> {
    let split = plan.len().saturating_sub(SERIAL_SEATS);
    match role {
        Role::Serial => plan.get(split..).unwrap_or_default().to_vec(),
        Role::Lane => plan.get(..split).unwrap_or_default().to_vec(),
    }
}

/// The property the pools exist for: lanes cannot take the seats the
/// city's serial threads need, however many lanes start first.
#[test]
fn lanes_never_take_the_seats_the_serial_threads_need() {
    for count in 0..=MOST_SEATS {
        let plan: Vec<Processor> = (0..count)
            .map(|number| Processor {
                group: 0,
                number: number * 2,
            })
            .collect();
        let mut table = Seats::new(plan.clone());
        // Every lane starts first, in order, twice over.
        for holder in holders().filter(|h| role(*h) == Role::Lane) {
            for _ in 0..2 {
                table.start(holder, Role::Lane);
            }
        }
        for holder in holders().filter(|h| role(*h) == Role::Serial) {
            table.start(holder, Role::Serial);
            let wanted = usize::try_from(holder.0).unwrap_or(usize::MAX);
            let seated = table.seat_of(holder).is_some();
            let seats = usize::try_from(count).unwrap_or(usize::MAX);
            assert_eq!(
                seated,
                wanted < seats,
                "every serial thread up to the plan's size is seated: {plan:?}"
            );
        }
    }
}

fn holders() -> impl Iterator<Item = Holder> {
    (0..THREADS).map(Holder)
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

#[test]
fn the_doctor_line_names_the_classes_and_what_the_plan_did() {
    use super::plan::{Access, Cpu};
    let cpu = |number: u32, class: u8, core: u32, cache: u32| Cpu {
        processor: Processor { group: 0, number },
        class,
        core,
        cache,
        access: Access::Allowed,
    };
    let hybrid = Topology::new(
        (0..8)
            .map(|n| cpu(n, 1, n / 2 * 2, 0))
            .chain((8..16).map(|n| cpu(n, 0, n, 0)))
            .collect(),
    );
    let line = describe(&Ok(hybrid.clone()));
    let tail = if cfg!(windows) {
        "hot threads prefer the 4 fastest cores, one thread each"
    } else {
        "this platform has no placement call; its scheduler places threads"
    };
    assert_eq!(
        line,
        format!(
            "CPU: 2 classes, 4 performance cores (8 threads), 8 efficiency cores (8 threads); {tail}"
        )
    );
    let dual_ccd = Topology::new((0..32).map(|n| cpu(n, 0, n / 2 * 2, n / 16)).collect());
    assert_eq!(
        describe(&Ok(dual_ccd)),
        "CPU: one class, 16 cores (32 threads) in 2 cache groups; left to the operating system and its cache steering"
    );
    assert_eq!(
        describe(&Err(Unread("sysfs is absent".to_owned()))),
        "CPU: topology unread (sysfs is absent); left to the operating system"
    );
    assert_eq!(
        super::plan::plan(&hybrid),
        Plan::Seats(
            (0..4)
                .map(|n| Processor {
                    group: 0,
                    number: n * 2
                })
                .collect()
        )
    );
    assert_ne!(super::plan::plan(&hybrid), Plan::LeftToOs(Left::OneClass));
}
