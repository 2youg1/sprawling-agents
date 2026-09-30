// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Who gets woken, and where the work lands.

use kernel::AxError;
use kernel::Locator;

use super::super::{
    Desks, Driven, Ending, Landed, Owing, QueueTenure, RunWorker, Settling, Site, Sweep, held,
};
use super::preparing::{Flown, LaneHalf, Staged};
use super::{Assignment, Given};

/// What the city built for a dispatch before the drive, and needs
/// again once the drive is home.
///
/// It exists because a drive may happen somewhere else. Where the
/// dispatch runs on this thread it is a local living across one call;
/// where a lane runs it, it waits in the pursuit that started it until
/// that run comes home (sprawling-SPEC.md 8-46-2). Either way there is
/// one per run, and nothing in it is shared.
pub(in crate::assembly) struct Continuation {
    desks: Desks,
    job_locator: Locator,
    /// This run's place in the backlog, given back where the run ends.
    member: Option<runtime::BacklogId>,
}

impl RunWorker {
    /// Everything the city decides for a dispatch before anything waits
    /// on the disk or on another process: agreeing to the work, opening
    /// the room, writing the brief, standing the run up, lending its
    /// desks, rebuilding the conversation it inherits.
    ///
    /// It hands back what the lane prepares and drives, and what the
    /// landing will need afterwards. Every line it writes goes through
    /// this worker on this thread; the review tree, the bench with its
    /// MCP servers and the frozen plan are the lane's
    /// (sprawling-SPEC.md 8-113).
    ///
    /// # Errors
    /// Propagates every refusal a dispatch can owe before it costs
    /// anything, and the failures of the phases that follow it.
    pub(in crate::assembly) fn stage_dispatch(
        &mut self,
        mut at: Assignment,
        task: String,
        goal: String,
    ) -> Result<(Staged, Continuation), AxError> {
        // The reading `tools/xtask/budgets.toml [prepare_dispatch_ms]` states:
        // what a dispatch spends on the accounting thread before a lane
        // takes it. Read from the city's own clock rather than
        // from a profiler, because the figure that matters is the one
        // taken on the thread no append is served on.
        let began = self.clock.now()?;
        // Nothing is written before the city agrees to take the work:
        // a halted city that laid a job file down would leave a task in
        // a room no run ever opened.
        let agreed = self.agree_to_work(&at)?;
        // A key pasted into the work goes to the vault before the text
        // is sent to be named, written to a room or recorded.
        let task = self.take_custody(task)?;
        let goal = self.take_custody(goal)?;
        // What this run will stand under reaches the history before it
        // governs anybody, and only once the city has agreed: a refused
        // dispatch still writes nothing (sprawling-SPEC.md 8-40).
        self.book_rules(&agreed.building)?;
        let session = super::session::session_for(&at.addr, at.session.take(), &task)?;
        // The first thing this city writes for a dispatch, and the line
        // where `addr` stops being where the work was sent and becomes
        // where the run works.
        at.addr = self.room_for(at.addr, session.as_ref())?;
        // What this session already froze, and the one dispatch that is
        // allowed to choose: the model and the effort are written into
        // the room's own layer at its first run and only read back at
        // every later one, because a shape that moved mid-session makes
        // every turn after it pay full price for a prefix that never
        // changed. A dispatch that would move it is refused here,
        // before the brief is written, so a refusal leaves the session
        // exactly as it was.
        self.choose_shape(&at, &agreed.model)?;
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
            .map_err(storage::StorageError::into_ax)?;
        let given = Given {
            job: Locator::cas(job_hash),
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
        // The run id is derived from the job locator, so the brief's
        // origin can only be recorded once the run stands; the bytes are
        // already durable and this put adds the origin record alone.
        self.cas
            .put_for(
                given.brief.segment_text().as_bytes(),
                &storage::BlockOrigin {
                    run: site.run_id,
                    building: at.addr.clone(),
                },
            )
            .map_err(storage::StorageError::into_ax)?;
        // The branch is the tree's name, so the desks opened here know it
        // while the lane still places the tree.
        site.name_tree(&at.addr)?;
        let desks = self.open_desks(&site, &at.addr)?;
        // The conversation this run opens with is rebuilt here, where
        // the ledger's index is held and its lineage line is written.
        let inherited = self.inherited(&at, site.run_id)?;
        let carried_from = self.origins.carried_from(&at.addr);
        // The run begins the room's session: what a carried session was
        // owed is spent, and this run is the room's last.
        self.origins.started(&at.addr, site.run_id);
        // A run somebody handed down stands in the backlog while it
        // drives, so a halt on its building reaches it; a root run is
        // ended by `Cancel`, which is a different verb.
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
        let spent = self.clock.now()?.value().saturating_sub(began.value());
        self.note(
            runtime::diagnostics::Level::Trace,
            "bin::assembly",
            &format!("prepare_dispatch took {spent} ms for {}", at.addr.as_str()),
        );
        Ok((
            Staged::new(at, site, lane),
            Continuation {
                desks,
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
    /// returning early at `driven?` would let a disk that went wrong take
    /// the room's mail and a tree's lease with it (sprawling-SPEC.md
    /// 8-46-9).
    ///
    /// The plan step closes each node of `open_claims` as that node's
    /// closing line reaches the ledger; the caller still owes the history
    /// a put-back line for every node it did not close.
    ///
    /// # Errors
    /// Propagates the drive's own failure to open a checkpoint, and
    /// every failure of settling the desks, the requests and the ending.
    pub(in crate::assembly) fn land(
        &mut self,
        continuation: Continuation,
        flown: Flown,
        owing: Owing,
        open_claims: &mut crate::assembly::booking::OpenClaims,
    ) -> Result<Landed, AxError> {
        let Continuation {
            desks,
            job_locator,
            member,
        } = continuation;
        let Flown {
            at,
            mut site,
            driven,
        } = flown;
        // Read before the obligation moves on: this run's place in the
        // conversation is what a signal it sends carries forward, and
        // `settle_desks` below is where those signals are spoken.
        let chain = owing.knock_chain().clone();
        // Both loans go back before either failure is propagated: a
        // backlog that would not take its member back must not cost the
        // room its mail too (sprawling-SPEC.md 8-46-9).
        let returned = self.return_borrowed(&at, &mut site, &desks);
        let left = member.map_or(Ok(()), |id| self.flight.backlog.leave(id));
        returned?;
        left?;
        // A lane that placed the tree and then failed leaves no sweep to
        // read it, so the tree goes back before the failure does; kept,
        // it would answer every later dispatch to the room with
        // WorktreeBusy until the worker restarts (sprawling-SPEC.md 8-113).
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
            checkpointed,
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
                    checkpointed: &checkpointed,
                    raised: &mut raised,
                    job_locator: &job_locator,
                },
                chain,
                open_claims,
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
                workshop: &desks.workshop,
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

    /// Gives the borrowed worktree back, once nothing else needs to
    /// read it.
    ///
    /// Later than the queue on purpose. The queue is a value this
    /// worker holds and comes home on every path; the worktree is a
    /// directory on disk that the sweep still has to diff against its
    /// own checkpoint, and releasing it first turns that diff into "object
    /// not found".
    ///
    /// # Errors
    /// Propagates a repository that will not give the tree back.
    fn release_lease(&mut self, site: &mut Site) -> Result<(), AxError> {
        let Some(held) = site.lease.take() else {
            return Ok(());
        };
        storage::Worktrees::open(&self.city_root)
            .map_err(storage::StorageError::into_ax)?
            .release(held)
            .map_err(storage::StorageError::into_ax)
    }
}
