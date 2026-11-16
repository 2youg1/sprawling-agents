// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::collections::VecDeque;
use std::sync::{Arc, Condvar, Mutex, MutexGuard, PoisonError};

#[derive(Clone)]
pub struct MachineBudget {
    shared: Arc<Shared>,
}

struct Shared {
    book: Mutex<Book>,
    advanced: Condvar,
}

pub struct Reservation {
    budget: MachineBudget,
    ticket: u64,
    waited: Option<Waited>,
}

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

    #[must_use]
    pub fn unmetered() -> MachineBudget {
        MachineBudget::new(u64::MAX)
    }

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

    pub(crate) fn give_back(&mut self, ticket: u64) {
        self.held.retain(|&(held, _)| held != ticket);
        self.advance();
    }

    pub(crate) fn holds(&self, ticket: u64) -> bool {
        self.held.iter().any(|&(held, _)| held == ticket)
    }

    pub(crate) fn held_bytes(&self) -> u64 {
        self.held
            .iter()
            .fold(0_u64, |sum, &(_, bytes)| sum.saturating_add(bytes))
    }

    #[cfg(test)]
    pub(crate) fn held_tickets(&self) -> impl Iterator<Item = u64> + '_ {
        self.held.iter().map(|&(ticket, _)| ticket)
    }

    #[cfg(test)]
    pub(crate) fn held_count(&self) -> usize {
        self.held.len()
    }

    #[cfg(test)]
    pub(crate) fn waiting_count(&self) -> usize {
        self.waiting.len()
    }

    #[cfg(test)]
    pub(crate) fn budget(&self) -> u64 {
        self.budget
    }

    fn advance(&mut self) {
        while let Some(&(ticket, bytes)) = self.waiting.front() {
            if !self.fits(bytes) {
                return;
            }
            self.held.push((ticket, bytes));
            self.waiting.pop_front();
        }
    }

    fn fits(&self, bytes: u64) -> bool {
        self.held.is_empty() || self.held_bytes().saturating_add(bytes) <= self.budget
    }
}

#[cfg(test)]
mod tests;
