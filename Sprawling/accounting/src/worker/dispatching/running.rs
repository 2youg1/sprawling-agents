// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use kernel::AxError;
use kernel::Locator;

use super::super::{
    Desks, Driven, Ending, Landed, Owing, QueueTenure, RunWorker, Settling, Site, Sweep, held,
};
use super::preparing::{Flown, LaneHalf, Staged};
use super::{Assignment, Seat};

pub(in crate::worker) enum Continuation {
    Model(Box<Lent>),

    Harness,
}

pub(in crate::worker) struct Lent {
    pub(in crate::worker) desks: Desks,
    job_locator: Locator,

    member: Option<runtime::BacklogId>,
}

impl RunWorker {
    pub(in crate::worker) fn stage_dispatch(
        &mut self,
        mut at: Assignment,
        task: String,
        goal: String,
    ) -> Result<(Staged, Continuation), AxError> {
        let began = (self.monotonic)();

        let seat = self.agree_to_work(&mut at)?;

        let task = self.take_custody(task)?;
        let goal = self.take_custody(goal)?;

        self.recover_project(seat.building().addr())?;
        self.book_rules(seat.building())?;
        let session = super::session::session_for(&at.addr, at.session.take(), &task)?;

        at.addr = self.room_for(at.addr, session.as_ref())?;
        let agreed = match seat {
            Seat::Model(mut agreed) => {
                use kernel::Model as _;

                let book = &self.credentials.book;
                let opening = book
                    .session_account(&at.addr, &agreed.provider)
                    .or_else(|| {
                        at.parent
                            .and_then(|mother| self.handing_room(mother))
                            .and_then(|room| book.session_account(room, &agreed.provider))
                    });
                if let Some(bound) = opening {
                    agreed.adapter.select_account(bound);
                }
                agreed
            }

            Seat::Harness(seat) => {
                let given = self.give(&at, task, goal)?;
                return self.stage_harness(at, seat, given, began);
            }
        };

        let generation = self.choose_shape(&at, &agreed.model, &agreed.provider)?;
        let given = self.give(&at, task, goal)?;

        let job_locator = given.job.clone();

        let mut site = self.stand_up(agreed, &at, &given, generation)?;

        self.cas
            .put_for(
                given.brief.segment_text().as_bytes(),
                &storage::BlockOrigin {
                    run: site.run_id,
                    building: at.addr.clone(),
                },
            )
            .map_err(storage::StorageError::into_ax)?;

        site.name_tree()?;
        let desks = self.open_desks(&site, &at.addr, at.depth())?;

        let inherited = self.inherited(&at, site.run_id)?;
        let carried_from = self.origins.carried_from(&at.addr);

        self.origins.started(&at.addr, site.run_id);

        let member = match at.parent {
            Some(_) => Some(self.flight.backlog.enrol_run(
                &at.addr,
                format!("run {} at {}", site.run_id, at.addr.as_str()),
            )?),
            None => None,
        };
        let lane = LaneHalf {
            given,
            job_locator: job_locator.clone(),
            desks: desks.for_bench(),
            laying: self.laying(&site.who)?,
            inherited,
            carried_from,
            member,
            command: self.doorstep.entrance.carrying(),
        };
        let spent = (self.monotonic)()
            .saturating_duration_since(began)
            .as_millis();
        self.note(
            runtime::diagnostics::Level::Trace,
            "accounting::worker",
            &format!("prepare_dispatch took {spent} ms for {}", at.addr.as_str()),
        );
        Ok((
            Staged::Model { at, site, lane },
            Continuation::Model(Box::new(Lent {
                desks,
                job_locator,
                member,
            })),
        ))
    }

    pub(in crate::worker) fn land(
        &mut self,
        continuation: Continuation,
        flown: Flown,
        owing: Owing,
        open_claims: &mut crate::worker::booking::OpenClaims,
    ) -> Result<Landed, AxError> {
        let (lent, at, mut site, driven, swept) = match (continuation, flown) {
            (
                Continuation::Model(lent),
                Flown::Model {
                    at,
                    site,
                    driven,
                    swept,
                },
            ) => (lent, at, site, driven, swept),
            (Continuation::Harness, Flown::Harness { at, driven }) => {
                return self.land_harness(at, driven, owing);
            }
            (Continuation::Harness, Flown::Model { mut site, .. }) => {
                self.release_lease(&mut site)?;
                return Err(crossed_continuation());
            }
            (Continuation::Model(_), Flown::Harness { driven, .. }) => {
                self.give_tree_back(driven.lease)?;
                return Err(crossed_continuation());
            }
        };
        let Lent {
            desks,
            job_locator,
            member,
        } = *lent;

        let chain = owing.knock_chain().clone();

        let returned = self.return_borrowed(&at, &mut site, &desks);
        let left = member.map_or(Ok(()), |id| self.flight.backlog.leave(id));
        returned?;
        left?;

        let driven = match driven {
            Ok(driven) => driven,
            Err(failure) => {
                self.release_lease(&mut site)?;
                return Err(failure);
            }
        };
        let Driven {
            outcome: driven,
            adapter: home,
            checkpointed: _,
            ran,
            mut raised,
            workbench,
        } = driven;
        site.adapter = Some(home);
        self.settle_desks(
            &site,
            &at,
            &desks,
            Settling {
                sweep: Sweep {
                    swept,
                    raised: &mut raised,
                    job_locator: &job_locator,
                },
                chain,
                open_claims,
            },
        )?;

        let produced = {
            let (ok, failed) = ran;
            runtime::Produced {
                tests_passed: if ok == 0 && failed == 0 {
                    None
                } else {
                    Some(failed == 0)
                },

                contract_moved: false,
                held_in: None,
                held_out: None,
            }
        };
        let settled = self
            .settle_requests(&site, &at, &desks.pr, &produced)
            .and_then(|()| {
                if driven.is_ok() {
                    self.finish_work(&site, &at)
                } else {
                    Ok(())
                }
            });
        let released = if settled.is_ok() && driven.is_ok() {
            self.retire_checkout(site.lease.take())
        } else {
            self.release_lease(&mut site)
        };
        settled?;
        released?;
        let landed = self.conclude(
            site,
            &at,
            Ending {
                driven,
                raised,
                succession: &workbench.succession,
                owing,
            },
        )?;

        self.answer_knocks();
        Ok(landed)
    }

    fn return_borrowed(
        &mut self,
        at: &Assignment,
        site: &mut Site,
        desks: &Desks,
    ) -> Result<(), AxError> {
        let returned = held(&desks.signals, "settle the signal desk")?.take_inbox();
        match desks.tenure {
            QueueTenure::TheRoomQueue => self.vacate(&at.addr, site.run_id, returned)?,
            QueueTenure::ASpare { held_by } => self.note(
                runtime::diagnostics::Level::Refuse,
                "collab::inbox",
                &format!(
                    "{} read no signals: {held_by} holds the queue of that room",
                    site.run_id
                ),
            ),
        }
        Ok(())
    }

    pub(super) fn retire_checkout(
        &self,
        lease: Option<storage::WorktreeLease>,
    ) -> Result<(), AxError> {
        if let Some(held) = lease {
            self.connectors.release_root(held.path());
            storage::Worktrees::open(held.owner())
                .and_then(|trees| trees.retire(held))
                .map_err(storage::StorageError::into_ax)?;
        }
        Ok(())
    }

    fn release_lease(&mut self, site: &mut Site) -> Result<(), AxError> {
        self.give_tree_back(site.lease.take())
    }

    pub(super) fn give_tree_back(
        &mut self,
        lease: Option<storage::WorktreeLease>,
    ) -> Result<(), AxError> {
        let Some(held) = lease else {
            return Ok(());
        };
        self.connectors.release_root(held.path());
        storage::Worktrees::open(held.owner())
            .map_err(storage::StorageError::into_ax)?
            .release(held)
            .map_err(storage::StorageError::into_ax)
    }
}

fn crossed_continuation() -> AxError {
    AxError::failure(
        kernel::AxCode::ConfigInvalid,
        "land a run",
        "the drive and what the city lent for it name two different residents",
    )
    .with_recovery("report this: one dispatch stages one resident and lands it")
}
