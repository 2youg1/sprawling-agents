// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The machine budget heavy work reserves its memory from, and the queue
//! it waits in past that budget (`crates/runtime/spec/Reserve.lean`, D103
//! and D104). The model there is the authority on what must hold: held
//! bytes within the budget unless one reservation holds alone, starts in
//! exactly the order of asking, and a queue that waits only behind a
//! holder. [`Book`] is that model's state and steps; [`MachineBudget`] is
//! the blocking handle the exec tool reserves through.

use std::collections::VecDeque;
use std::sync::{Arc, Condvar, Mutex, MutexGuard, PoisonError};

/// One machine's budget for heavy work. Clones share one book, so every
/// run's exec tool queues in the same line.
#[derive(Clone)]
pub struct MachineBudget {
    shared: Arc<Shared>,
}

struct Shared {
    book: Mutex<Book>,
    advanced: Condvar,
}

/// Bytes held until dropped. Dropping gives them back and wakes whoever
/// waits behind them.
pub struct Reservation {
    budget: MachineBudget,
    ticket: u64,
    waited: Option<Waited>,
}

/// What stood ahead of a reservation when it asked and did not fit at
/// once: how many asks were ahead in line, the bytes held, and the
/// budget, so a result can say whom it waited for and for how much.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Waited {
    pub ahead: usize,
    pub held_bytes: u64,
    pub budget_bytes: u64,
}

impl MachineBudget {
    #[must_use]
    pub fn new(budget_bytes: u64) -> MachineBudget {
        MachineBudget {
            shared: Arc::new(Shared {
                book: Mutex::new(Book::new(budget_bytes)),
                advanced: Condvar::new(),
            }),
        }
    }

    /// A budget everything fits in, for a table no machine budget was set
    /// on.
    #[must_use]
    pub fn unmetered() -> MachineBudget {
        MachineBudget::new(u64::MAX)
    }

    /// Takes a ticket for `bytes` and blocks until the book starts it.
    /// Never fails: the book's every step is made whole under its lock, so
    /// a poisoned lock still holds a consistent book and is used as such.
    #[must_use]
    pub fn reserve(&self, bytes: u64) -> Reservation {
        let mut book = self.book();
        let (ticket, waited) = book.ask(bytes);
        while !book.holds(ticket) {
            book = self
                .shared
                .advanced
                .wait(book)
                .unwrap_or_else(PoisonError::into_inner);
        }
        Reservation {
            budget: self.clone(),
            ticket,
            waited,
        }
    }

    fn book(&self) -> MutexGuard<'_, Book> {
        self.shared
            .book
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
    }
}

impl Reservation {
    /// What stood ahead when this reservation asked, if it did not start
    /// at once.
    #[must_use]
    pub fn waited(&self) -> Option<Waited> {
        self.waited
    }
}

impl Drop for Reservation {
    fn drop(&mut self) {
        self.budget.book().give_back(self.ticket);
        self.budget.shared.advanced.notify_all();
    }
}

/// The model's book (`Reserve.lean`, `Book`): the budget, what is held,
/// and who waits, in ticket order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Book {
    budget: u64,
    held: Vec<(u64, u64)>,
    waiting: VecDeque<(u64, u64)>,
    next: u64,
}

impl Book {
    pub(crate) fn new(budget: u64) -> Book {
        Book {
            budget,
            held: Vec::new(),
            waiting: VecDeque::new(),
            next: 0,
        }
    }

    /// The model's `ask`: a new ticket at the back of the line, then the
    /// line advances. Answers the ticket, and what stood ahead when it
    /// did not start at once.
    pub(crate) fn ask(&mut self, bytes: u64) -> (u64, Option<Waited>) {
        let ticket = self.next;
        self.next = self.next.wrapping_add(1);
        let ahead = Waited {
            ahead: self.waiting.len(),
            held_bytes: self.held_bytes(),
            budget_bytes: self.budget,
        };
        self.waiting.push_back((ticket, bytes));
        self.advance();
        let waited = (!self.holds(ticket)).then_some(ahead);
        (ticket, waited)
    }

    /// The model's `giveBack`: the ticket leaves what is held, then the
    /// line advances. A ticket not held changes nothing.
    pub(crate) fn give_back(&mut self, ticket: u64) {
        self.held.retain(|&(held, _)| held != ticket);
        self.advance();
    }

    pub(crate) fn holds(&self, ticket: u64) -> bool {
        self.held.iter().any(|&(held, _)| held == ticket)
    }

    /// The bytes held, summed saturating: a sum past `u64::MAX` reads as
    /// not fitting, as the model's natural numbers would have it.
    pub(crate) fn held_bytes(&self) -> u64 {
        self.held
            .iter()
            .fold(0_u64, |sum, &(_, bytes)| sum.saturating_add(bytes))
    }

    pub(crate) fn held_count(&self) -> usize {
        self.held.len()
    }

    pub(crate) fn waiting_count(&self) -> usize {
        self.waiting.len()
    }

    pub(crate) fn budget(&self) -> u64 {
        self.budget
    }

    /// The model's `advance`: from the head of the line, start each ask
    /// that fits.
    fn advance(&mut self) {
        while let Some(&(ticket, bytes)) = self.waiting.front() {
            self.held.push((ticket, bytes));
            self.waiting.pop_front();
        }
    }
}

#[cfg(test)]
mod tests;
