// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Who gets woken, and where the work lands.

use kernel::{Address, Locator};
use kernel::{AxCode, AxError, EventKind};

use crate::effect;

use super::super::{
    CITY_VERIFIER, Desks, Driven, Driving, Ending, Holding, Landed, Owing, RunWorker, Site, Sweep,
    Workbench, artifact_of, held, now_ms,
};
use super::{Assignment, Dispatched, Given};

/// What the city built for a dispatch before the drive, and needs
/// again once the drive is home.
///
/// It exists because a drive may happen somewhere else. Where the
/// dispatch runs on this thread it is a local living across one call;
/// where a lane runs it, it waits in the pursuit that started it until
/// that run comes home (sprawling-SPEC.md 8-46-2). Either way there is
/// one per run, and nothing in it is shared.
pub(in crate::assembly) struct Continuation {
    at: Assignment,
    site: Site,
    desks: Desks,
    workbench: Workbench,
    job_locator: Locator,
    /// This run's place in the backlog, given back where the run ends.
    member: Option<runtime::BacklogId>,
}

impl RunWorker {
    /// Everything the city does for a dispatch before anything is
    /// driven: agreeing to the work, opening the room, writing the
    /// brief, standing the run up, laying out its bench, freezing its
    /// plan.
    ///
    /// It hands back the drive and what that drive will need afterwards.
    /// Every line it writes goes through this worker on this thread,
    /// before any lane exists.
    ///
    /// # Errors
    /// Propagates every refusal a dispatch can owe before it costs
    /// anything, and the failures of the phases that follow it.
    pub(in crate::assembly) fn prepare_dispatch(
        &mut self,
        mut at: Assignment,
        task: String,
        goal: String,
    ) -> Result<(Driving, Continuation), AxError> {
        // Nothing is written before the city agrees to take the work:
        // a halted city that laid a job file down would leave a task in
        // a room no run ever opened.
        let agreed = self.agree_to_work(&at.addr)?;
        // Naming the work costs one call to the digest model, so it is
        // asked after the city has agreed rather than before: a person
        // does not pay a provider to name work this city was never going
        // to take. The rules read a moment ago carry the building's
        // policy into that call, so the task text of a confidential
        // building reaches only a model on this machine.
        let session =
            self.session_for(&at.addr, at.session.take(), &task, agreed.rules.policy())?;
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

        let fence_scope = site.fence_scope()?;
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
        let driving = Driving {
            adapter,
            bench: workbench.take_bench()?,
            signals: std::sync::Arc::clone(&desks.signals),
            write_root: site.write_root.clone(),
            fence_scope,
            run_id: site.run_id,
            of: site.provenance(self.city_hash()?, &at.addr),
            sieving: self.sieving_for(&site, &at.addr)?,
            member,
            plan,
            handoff,
        };
        Ok((
            driving,
            Continuation {
                at,
                site,
                desks,
                workbench,
                job_locator,
                member,
            },
        ))
    }

    /// Puts what a drive left onto the history, and concludes the run.
    ///
    /// Everything here happens on the accounting thread, whichever
    /// thread drove: settling is where a run's last lines are written,
    /// and a city has one writer.
    ///
    /// **What the dispatch borrowed is given back before the drive's
    /// own outcome is read.** A drive that failed is exactly when the
    /// room's queue and the worktree lease are most likely to be lost:
    /// the old order returned early at `driven?`, so a disk that went
    /// wrong took the room's mail and a tree's lease with it
    /// (sprawling-SPEC.md 8-46-9).
    ///
    /// # Errors
    /// Propagates the drive's own failure to open a checkpoint, and
    /// every failure of settling the desks, the requests and the ending.
    pub(in crate::assembly) fn land(
        &mut self,
        continuation: Continuation,
        driven: Result<Driven, AxError>,
        owing: Owing,
    ) -> Result<Landed, AxError> {
        let Continuation {
            at,
            mut site,
            desks,
            workbench,
            job_locator,
            member,
        } = continuation;
        // Both loans go back before either failure is propagated: a
        // backlog that would not take its member back used to cost the
        // room its mail too (sprawling-SPEC.md 8-46-9).
        let returned = self.return_borrowed(&at, &mut site, &desks);
        let left = member.map_or(Ok(()), |id| self.backlog.leave(id));
        returned?;
        left?;
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
        self.release_lease(&mut site)?;
        let landed = self.conclude(
            site,
            &at,
            Ending {
                driven,
                raised,
                delegates: &workbench.delegates,
                succession: &workbench.succession,
                owing,
            },
        )?;
        // Whoever this run spoke to answers next, and whoever they
        // speak to after that. Drained here rather than on the person's
        // path alone, because a delegate and a scheduled job start
        // conversations the same way a typed dispatch does.
        self.answer_knocks();
        Ok(landed)
    }

    /// Gives back everything this dispatch borrowed: the room's queue
    /// and, under review, the worktree it wrote in.
    ///
    /// Called before the drive's outcome is read, so a failed drive
    /// returns what a finished one returns. A lease left out is a tree
    /// no later run can claim, and a queue left in a dropped desk is
    /// mail the ledger says arrived and no room holds.
    ///
    /// # Errors
    /// Propagates a desk left locked, a queue this run was not holding,
    /// and a tree the worktree table will not take back.
    fn return_borrowed(
        &mut self,
        at: &Assignment,
        site: &mut Site,
        desks: &Desks,
    ) -> Result<(), AxError> {
        let returned = held(&desks.signals, "settle the signal desk")?.take_inbox();
        match desks.holding {
            Holding::TheRoomQueue => self.rooms.give_back(&at.addr, site.run_id, returned)?,
            Holding::ASpare { held_by } => self.note(
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

    /// Gives the borrowed worktree back, once nothing else needs to
    /// read it.
    ///
    /// Later than the queue on purpose. The queue is a value this
    /// worker holds and comes home on every path; the worktree is a
    /// directory on disk that the sweep still has to diff against its
    /// own fence, and releasing it first turns that diff into "object
    /// not found".
    ///
    /// # Errors
    /// Propagates a repository that will not give the tree back.
    fn release_lease(&mut self, site: &mut Site) -> Result<(), AxError> {
        let Some(held) = site.lease.take() else {
            return Ok(());
        };
        memory::Worktrees::open(&self.city_root)
            .map_err(memory::MemoryError::into_ax)?
            .release(held)
            .map_err(memory::MemoryError::into_ax)
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
        // Through the room table rather than into a queue of its own:
        // the parent room may have another run reading in it, and a
        // handback delivered beside that reader is one nobody collects.
        self.rooms.deliver(&signal)?;
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
