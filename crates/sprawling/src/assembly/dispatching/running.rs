// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Who gets woken, and where the work lands.

use kernel::Locator;
use kernel::{Address, AxCode, AxError, EventKind};

use crate::effect;

use super::super::{
    CITY_VERIFIER, Driven, Driving, Ending, RunWorker, Sweep, artifact_of, new_inbox, now_ms,
};
use super::{Assignment, Dispatched, Given};

impl RunWorker {
    pub(in crate::assembly) fn dispatch_in(
        &mut self,
        mut at: Assignment,
        task: String,
        goal: String,
    ) -> Result<Dispatched, AxError> {
        // Nothing is written before the city agrees to take the work:
        // a halted city that laid a job file down would leave a task in
        // a room no run ever opened.
        let agreed = self.agree_to_work(&at.addr)?;
        // Naming the work costs one call to the digest model, so it is
        // asked after the city has agreed rather than before: a person
        // does not pay a provider to name work this city was never going
        // to take.
        let session = self.session_for(&at.addr, at.session.take(), &task)?;
        // The first thing this city writes for a dispatch, and the line
        // where `addr` stops being where the work was sent and becomes
        // where the run works.
        at.addr = self.room_for(at.addr, session.as_ref())?;
        // Written into the session's own layer rather than held beside
        // the run: the ladder that resolves city -> building -> room is
        // already the authority on how hard a run thinks, and a second
        // store would be a second answer. Chosen once, it holds for
        // every later run in that room - and for this one, which is why
        // it is written before the configuration is frozen.
        if let Some(effort) = at.effort {
            city::write_effort(&self.city_root, &at.addr, effort)?;
        }
        // The task file exists first, then the run exists: the job on
        // disk is what the agent reads, and the copy in the store is
        // what the history keeps, so editing one cannot rewrite the
        // other.
        let brief = city::write_brief(
            &self.city_root,
            &at.addr,
            &city::JobBrief {
                task: &task,
                goal: &goal,
            },
        )?;
        // What the run was given, pinned whichever arm it is: for a
        // session nobody assigned, the pin holds the words that said so,
        // so the ledger's `job` locator resolves to the bytes the run
        // segment actually carried rather than to a file that was never
        // written.
        let job_hash = self
            .cas
            .put(brief.segment_text().as_bytes())
            .map_err(memory::MemoryError::into_ax)?;
        let given = Given {
            job: Locator::parse(&format!("cas:b3-{job_hash}"))?,
            brief,
            task,
            goal,
        };
        // Kept for the post-drive sweep: an escalation names the work it
        // interrupted, and by then the plan has consumed the original.
        let job_locator = given.job.clone();

        // Where this run stands, and the desks it works at: two phases,
        // two values, both carried whole rather than taken apart into a
        // row of locals every phase below would then have to be handed
        // one at a time.
        let mut site = self.stand_up(agreed, &at, &given)?;
        let desks = self.open_desks(&site, &at.addr)?;

        // What the model may see, what routes the call it makes, and
        // who it may hand work down to: one phase, one value.
        let mut workbench = self.lay_out_workbench(&site, &desks, &at, &job_locator)?;
        // The plan this run is frozen with, and the handoff that says
        // what to read to pick it up again: one phase, because the
        // handoff quotes the plan and the plan is what the prefix was
        // assembled for.
        let (plan, handoff) = self.freeze_plan(&site, &workbench, &at, given)?;
        // The probe's second reading, over what this successor was
        // handed and before it takes a turn: the comparison with the
        // predecessor's answers is what says whether the handoff lost
        // something.
        if let Some(handed) = at.succession.as_ref() {
            self.probe_after(&mut site, &plan, handed)?;
        }

        let fence_scope = site.fence_scope(&at.addr);
        // The adapter moves into the drive and comes home in `Driven`:
        // `Site` gains and loses no field over one dispatch. `Option`
        // is the move's vehicle, not a new state: it is `Some` on both
        // sides of the call, and `None` between is unobservable. The
        // refusal names the invariant rather than panicking on it:
        // a site without an adapter is a defect in `stand_up`, and
        // the person who dispatched deserves that address.
        let Some(adapter) = site.adapter.take() else {
            return Err(AxError::failure(
                AxCode::ConfigInvalid,
                "carry the adapter into the drive",
                "the site arrived without one",
            )
            .with_recovery("report this: stand_up always seats an adapter"));
        };
        // A run somebody handed down stands in the backlog while it
        // drives, so a halt on its building reaches it; a root run is
        // ended by `Cancel`, which is a different verb.
        let member = match at.parent {
            Some(_) => Some(self.backlog.enrol_run(
                &at.addr,
                format!("run {} at {}", site.run_id, at.addr.as_str()),
            )?),
            None => None,
        };
        let driven = self.drive_dispatch(
            plan,
            &handoff,
            Driving {
                adapter,
                bench: &mut workbench.bench,
                signals: std::sync::Arc::clone(&desks.signals),
                write_root: &site.write_root,
                fence_scope: fence_scope.clone(),
                who: &site.who,
                run_id: site.run_id,
                of: site.provenance(&self.city_root, &at.addr)?,
                sieving: self.sieving_for(&site, &at.addr)?,
                member,
            },
        );
        if let Some(id) = member {
            self.backlog.leave(id)?;
        }
        let Driven {
            outcome: driven,
            adapter: home,
            fenced,
            ran,
            mut raised,
        } = driven?;
        site.adapter = Some(home);
        self.settle_desks(
            &site,
            &at,
            &desks,
            Sweep {
                fenced: &fenced,
                raised: &mut raised,
                job_locator: &job_locator,
            },
        )?;
        // What the run can show for itself. `None` is not `Some(false)`:
        // "nothing ran" and "something ran and failed" are different
        // facts, and the modes that care refuse them differently.
        let produced = {
            let (ok, failed) = ran;
            runtime::Produced {
                tests_passed: if ok == 0 && failed == 0 {
                    None
                } else {
                    Some(failed == 0)
                },
                // The city cannot see a contract move from here; a run
                // that renovates says so through its evidence, and until
                // it can, SC admits on the honest default.
                contract_moved: false,
                held_in: None,
                held_out: None,
            }
        };
        self.settle_requests(&site, &at, &desks.pr, &produced)?;
        self.conclude(
            site,
            &at,
            Ending {
                driven,
                raised,
                delegates: &workbench.delegates,
                succession: &workbench.succession,
            },
        )
    }

    /// Where a building keeps the plan its residents claim rows from.
    /// One dispatch, run to its frozen end on this thread.
    pub(in crate::assembly) fn dispatch(
        &mut self,
        addr: Address,
        task: String,
        goal: String,
    ) -> Result<(), AxError> {
        self.dispatch_in(
            Assignment {
                addr,
                // The address is already where the work happens: every
                // caller of this one is the city dispatching into a room
                // that exists, rather than a person opening a session.
                session: None,
                effort: None,
                mode: runtime::Mode::PlanGoal,
                parent: None,
                succession: None,
            },
            task,
            goal,
        )
        .map(drop)
    }

    /// Tells the run that asked for the work how it came back.
    ///
    /// The child's account is pinned in the store before it is judged,
    /// so the locator the parent is handed resolves to bytes rather than
    /// to a sentence this process happened to build. The city verifies:
    /// `Completion::Done` is something the city observed, and a producer
    /// verifying itself is what `Claim::verified` refuses.
    pub(in crate::assembly) fn deliver_handback(
        &mut self,
        parent: &Address,
        child: &Dispatched,
    ) -> Result<(), AxError> {
        let account = format!(
            "room: {}\nby: {}\nending: {}\n",
            child.addr.as_str(),
            child.who,
            child.completion.name()
        );
        let digest = self
            .cas
            .put(account.as_bytes())
            .map_err(memory::MemoryError::into_ax)?;
        let claim = collab::Claim::new(
            collab::NodeId::parse(child.addr.as_str())?,
            Locator::parse(&format!("cas:b3-{digest}"))?,
            digest,
            child.who.clone(),
        );
        let back = collab::Handback::of(
            claim,
            matches!(child.completion, kernel::Completion::Done(_)),
            CITY_VERIFIER,
        );
        let signal = back.signal(
            collab::SignalId::parse(&format!("handback-{}", child.run))?,
            parent.clone(),
            now_ms()?,
        )?;
        // Recorded, then delivered - the same order every other signal
        // takes, so the queue only ever changes as a consequence of a
        // line the history already has.
        self.record_for(
            child.run,
            effect::Line {
                who: child.who.to_owned(),
                addr: parent.clone(),
                kind: EventKind::SignalEnqueued,
                data: signal.enqueued_payload()?,
            },
        )?;
        self.inboxes
            .entry(parent.clone())
            .or_insert_with(new_inbox)
            .deliver(&signal)?;
        // And into the room's join, by the same reading a restart would
        // do: one function decides what a handback signal means.
        if let Some(artifact) = artifact_of(&signal) {
            self.joins
                .entry(parent.clone())
                .or_default()
                .accept(artifact);
        }
        Ok(())
    }
}
