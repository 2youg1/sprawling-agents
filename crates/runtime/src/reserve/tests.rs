// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "test code"
)]

use std::collections::BTreeSet;

use proptest::prelude::*;

use super::*;

const BUDGET: u64 = 100;

#[derive(Debug, Clone)]
enum Event {
    Ask(u64),
    /// Gives back the n-th ticket asked so far, or a ticket never asked
    /// when n runs past them.
    GiveBack(usize),
}

fn event() -> impl Strategy<Value = Event> {
    prop_oneof![
        (0..2 * BUDGET).prop_map(Event::Ask),
        (0_usize..24).prop_map(Event::GiveBack),
    ]
}

proptest! {
    /// The three properties of `crates/runtime/spec/Reserve.lean`, after
    /// every step of any trace of asks and give-backs: held bytes within
    /// the budget or one holder alone (`held_within_budget`), tickets
    /// started in the order they asked (`started_in_ask_order`), and a
    /// line that waits only behind a holder (`waits_only_behind_a_holder`).
    #[test]
    fn the_book_keeps_the_lean_properties(events in prop::collection::vec(event(), 0..64)) {
        let mut book = Book::new(BUDGET);
        let mut asked: Vec<u64> = Vec::new();
        let mut started: BTreeSet<u64> = BTreeSet::new();
        for event in events {
            match event {
                Event::Ask(bytes) => asked.push(book.ask(bytes).0),
                Event::GiveBack(n) => book.give_back(asked.get(n).copied().unwrap_or(u64::MAX)),
            }
            started.extend(asked.iter().copied().filter(|&ticket| book.holds(ticket)));

            prop_assert!(book.held_bytes() <= book.budget() || book.held_count() == 1);
            let prefix: BTreeSet<u64> = asked.iter().copied().take(started.len()).collect();
            prop_assert_eq!(&started, &prefix);
            prop_assert!(book.waiting_count() == 0 || book.held_count() > 0);
        }
    }
}

/// A reservation that did not fit waits until the holder gives its bytes
/// back, and says what stood ahead of it when it asked.
#[test]
fn a_reservation_past_the_budget_waits_for_the_holder() {
    let budget = MachineBudget::new(10);
    let first = budget.reserve(6);
    assert_eq!(first.waited(), None);
    let waiter = {
        let budget = budget.clone();
        std::thread::spawn(move || budget.reserve(5).waited())
    };
    while budget.book().waiting_count() == 0 && !waiter.is_finished() {
        std::thread::yield_now();
    }
    drop(first);
    assert_eq!(
        waiter.join().unwrap(),
        Some(Waited {
            ahead: 0,
            held_bytes: 6,
            budget_bytes: 10,
        })
    );
}
