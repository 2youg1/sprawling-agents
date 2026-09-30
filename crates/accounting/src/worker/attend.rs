// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The accounting thread's loop, and the counts it keeps for other
//! threads to read.
//!
//! The loop looks at three things in this order: every relay request a
//! lane is blocked on, then at most one run home from a lane, then the
//! desk (sprawling-SPEC.md 8-42-4). The crossing is first because a run
//! that has already been paid for must not queue behind one that has
//! not started.
//!
//! Which properties the loop must hold - every message served in
//! finitely many steps, append order equal to seq order, no wake
//! without work - is decided by the Lean model
//! `crates/accounting/spec/Worker/Attend.lean`; this loop polls and so does not
//! yet hold the third (sprawling-SPEC.md 8-42-4).

use std::time::Duration;

use super::RunWorker;
use super::desk::{CommandDesk, DeskWait, SCHEDULE_TICK_MS};
use super::health::Health;
use super::relay::Patience;

impl RunWorker {
    /// The accounting queue's counts, readable from another thread: the
    /// monitor samples them (sprawling-SPEC.md 8-98).
    pub fn health(&self) -> Health {
        self.flight.gate.health()
    }

    /// Hands this worker's session slices to the thread that will file
    /// them from now on, so this thread files none (sprawling-SPEC.md
    /// 8-123). `None` when they were handed off already, or the ledger
    /// is not a city's.
    pub fn hand_off_session_slices(&mut self) -> Option<storage::Sessions> {
        self.ledger.hand_off_session_slices()
    }
}

/// The accounting thread's loop: every relay request the one queue
/// holds, then at most one run home, then the desk, in that order
/// (sprawling-SPEC.md 8-42-4), until the desk closes.
///
/// The thread sleeps on the one queue and on nothing else, so a relay
/// request, a run home, a posted command and a close each wake it the
/// moment they are queued; an idle city wakes only at the schedule's
/// deadline (`crates/accounting/spec/Worker/Attend.lean`).
///
/// A function of its own rather than the body of the thread's closure,
/// because the instruments that time a relay round trip and the gap two
/// dispatches leave in a run drive this loop and not a copy of it
/// (sprawling-SPEC.md 8-84): a copy that waited differently would be
/// measured instead of the city.
pub fn attend(worker: &mut RunWorker, desk: &CommandDesk) {
    desk.ring_through(worker.bell());
    // When the schedule was last read against. Kept by the loop rather
    // than measured from it, because a city with lanes driving comes
    // back here on every relay request, and one that opened its
    // schedule file that often would spend its time opening a file
    // (sprawling-SPEC.md 8-46-2).
    let mut read_schedule_at = kernel::TimeMs::new(0);
    // The desk is read before the first wait, because what was posted
    // before this thread attended rang nobody.
    let mut patience = Patience::Now;
    loop {
        // The first two of the three mouths: every relay request
        // queued, then at most one run home. The crossing is served
        // first, because a run that has already been paid for must not
        // queue behind one that has not started.
        if let Err(err) = worker.serve_flight(patience) {
            eprintln!("a run could not be landed: {err}");
        }
        // The third mouth, which also decides how long the next look
        // may sleep.
        patience = match desk.next(|run| worker.drives(run)) {
            // `carrying` holds this command's key in flight for the
            // length of the arm, so a frame that repeats it while the
            // work is going adds no second run. The desk may hold more,
            // so the next look does not sleep.
            DeskWait::Command(posted, carrying) => {
                worker.serve_one(*posted);
                drop(carrying);
                Patience::Now
            }
            // The refusal is written inside `tick`; a schedule that
            // cannot be read must not stop the city from answering the
            // person.
            DeskWait::Idle => {
                let since = match worker.clock.now() {
                    Ok(now) => {
                        let since = now.value().saturating_sub(read_schedule_at.value());
                        if since >= SCHEDULE_TICK_MS {
                            read_schedule_at = now;
                            drop(worker.tick(now));
                            0
                        } else {
                            since
                        }
                    }
                    Err(_) => 0,
                };
                // A kept keep-warm door that falls due before the next
                // schedule read wakes the loop for itself
                // (sprawling-SPEC.md 8-112); none is kept by default.
                let until_warm = match (worker.warm_due(), worker.clock.now()) {
                    (Some(_), Ok(now)) => {
                        worker.renew_warm(now);
                        worker
                            .warm_due()
                            .map_or(u64::MAX, |due| due.saturating_sub(now.value()))
                    }
                    (None, _) | (Some(_), Err(_)) => u64::MAX,
                };
                Patience::For(Duration::from_millis(
                    SCHEDULE_TICK_MS.saturating_sub(since).min(until_warm),
                ))
            }
            DeskWait::Close(why) => {
                // A city closed before the proof of its history finished
                // writes its handoff once the proof has a verdict, rather
                // than having it refused (sprawling-SPEC.md 8-90).
                worker.await_proof();
                // The lanes are waited for rather than abandoned: a lane
                // left blocked on an append loses lines this city had
                // already told it were durable.
                if let Err(err) = worker.land_the_rest() {
                    eprintln!("a run could not be landed as the city closed: {err}");
                }
                if let Err(err) = worker.close_city(why) {
                    eprintln!("the city could not write its handoff: {err}");
                }
                break;
            }
            DeskWait::Gone => break,
        };
    }
}
