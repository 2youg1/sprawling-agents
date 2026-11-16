// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::time::Duration;

use super::RunWorker;
use super::desk::{CommandDesk, DeskWait, SCHEDULE_TICK_MS};
use super::health::Health;
use super::relay::Patience;

impl RunWorker {

    pub fn health(&self) -> Health {
        self.flight.gate.health()
    }

    pub fn relay(&self) -> super::Relay {
        self.flight.gate.issue()
    }

    pub fn hand_off_session_slices(&mut self) -> Option<storage::Sessions> {
        self.ledger.hand_off_session_slices()
    }
}

pub fn attend(worker: &mut RunWorker, desk: &CommandDesk) {
    desk.ring_through(worker.bell());

    let mut read_schedule_at = kernel::TimeMs::new(0);

    let mut patience = Patience::Now;
    loop {

        if let Err(err) = worker.serve_flight(patience) {
            eprintln!("a run could not be landed: {err}");
        }

        patience = match desk.next(|run| worker.drives(run)) {

            DeskWait::Command(posted, carrying) => {
                worker.serve_one(*posted);
                drop(carrying);
                Patience::Now
            }

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

                worker.await_proof();

                if desk.interrupting() {
                    worker.stop_the_backlog();
                }

                if let Err(err) = worker.land_the_rest() {
                    worker.note(
                        runtime::diagnostics::Level::Refuse,
                        "accounting::worker",
                        &format!("a run could not be landed as the city closed: {err}"),
                    );
                }
                if let Err(err) = worker.close_city(why) {
                    worker.note(
                        runtime::diagnostics::Level::Refuse,
                        "accounting::worker",
                        &format!("the city could not write its handoff: {err}"),
                    );
                }
                break;
            }
            DeskWait::Gone => break,
        };
    }
}
