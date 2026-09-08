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
use super::{Assignment, DISPATCH_TURN_BUDGET, Dispatched, Given, NAME_THE_WORK, NAME_TOKENS};
use crate::assembly::credentials::dialect_headers;

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
                budget: &format!("{DISPATCH_TURN_BUDGET} turns"),
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
        let Driven {
            outcome: driven,
            adapter: home,
            fenced,
            ran,
            mut raised,
        } = self.drive_dispatch(
            plan,
            &handoff,
            Driving {
                adapter,
                bench: &mut workbench.bench,
                signals: &desks.signals,
                write_root: &site.write_root,
                fence_scope: fence_scope.clone(),
                who: &site.who,
                run_id: site.run_id,
                of: site.provenance(&self.city_root, &at.addr)?,
            },
        )?;
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
                budget: kernel::BudgetCap::default(),
                parent: None,
            },
            task,
            goal,
        )
        .map(drop)
    }

    /// Where a dispatch works: the room a named session opens, or the
    /// address it was sent to.
    ///
    /// A named session opens a room of its own under the building, so
    /// two sessions started from the same screen do not write over each
    /// other's files. An unnamed one is a person continuing what is
    /// already at that address.
    /// What this piece of work should be called, when the person did not
    /// say.
    ///
    /// A person who writes one sentence has named the work in it, and
    /// making them name it twice is the ceremony this interface exists
    /// to remove. So an address that names a building with no session
    /// beside it is answered here, by asking the cheapest model this
    /// city has for a short name.
    ///
    /// **A refusal, never a guess.** When no model answers, when the
    /// answer is not a legal session name, or when the address already
    /// names a room, this returns what it was given. The failure a
    /// person then sees names the field they have to fill, and the
    /// composer opens that one control — which is the whole reason the
    /// fallback is a refusal rather than a name this city made up. A run
    /// living in a room somebody did not choose cannot be found again by
    /// the name they would look for.
    pub(super) fn session_for(
        &mut self,
        addr: &Address,
        session: Option<kernel::SessionName>,
        task: &str,
    ) -> Result<Option<kernel::SessionName>, AxError> {
        if session.is_some() {
            return Ok(session);
        }
        // An address with a room in it is already a session: this is the
        // shape a second dispatch into an open session takes, and naming
        // it again would open a room inside a room.
        if addr.as_str().contains('/') {
            return Ok(None);
        }
        let Some(named) = self.name_the_work(task) else {
            return Err(AxError::failure(
                AxCode::InvalidArgs,
                "work out what to call this work",
                addr.as_str().to_owned(),
            )
            .with_recovery(
                "name the room yourself: send the work to `building/name` rather than to \
                 `building`",
            ));
        };
        Ok(Some(named))
    }

    /// One cheap model call, turning a task into a short room name.
    ///
    /// The digest tag, which is this city's word for the small model
    /// that reads so the main one does not have to. Naming a piece of
    /// work is exactly that shape of job, and putting it on the main
    /// model would charge a person reasoning tokens for a filename.
    ///
    /// `None` for every failure this can have — no model registered for
    /// the tag, a call that did not come back, an answer that is not a
    /// legal session name. The caller turns that into a refusal naming
    /// the field, so a failure here costs a person one field rather than
    /// putting their work somewhere they will not look for it.
    fn name_the_work(&mut self, task: &str) -> Option<kernel::SessionName> {
        let chosen = self
            .book
            .select(kernel::ModelTag::Digest, &kernel::BuildingPolicy::default())
            .ok()?;
        let model_id = chosen.entry.id.clone();
        let mut adapter = gateway::adapter_for(
            &chosen,
            self.redemption(),
            dialect_headers(chosen.endpoint.dialect),
        )
        .ok()?;
        let answer = adapter
            .call(&kernel::ModelRequest {
                policy: kernel::BuildingPolicy::default(),
                segments: [kernel::B3Hash::digest(b""); 4],
                chat: kernel::ChatRequest {
                    model: model_id,
                    max_tokens: NAME_TOKENS,
                    system: vec![kernel::SystemBlock {
                        text: NAME_THE_WORK.to_owned(),
                        cache: false,
                    }],
                    messages: vec![kernel::ChatMessage {
                        role: kernel::Role::User,
                        content: vec![kernel::ContentBlock::Text {
                            text: task.to_owned(),
                        }],
                    }],
                    tools: Vec::new(),
                    effort: None,
                },
            })
            .ok()?;
        // The model is asked for one word and sometimes writes a
        // sentence around it. The first line, stripped of the
        // punctuation an answer tends to arrive wrapped in, is what is
        // offered to the parser — and the parser decides, not this.
        let said = kernel::content_from_message(&answer.message)
            .ok()?
            .into_iter()
            .find_map(|block| match block {
                kernel::ContentBlock::Text { text } => Some(text),
                _ => None,
            })?;
        let candidate = said
            .lines()
            .next()?
            .trim()
            .trim_matches(|glyph: char| glyph == '`' || glyph == '"' || glyph == '.');
        kernel::SessionName::parse(candidate).ok()
    }

    pub(super) fn room_for(
        &self,
        addr: Address,
        session: Option<&kernel::SessionName>,
    ) -> Result<Address, AxError> {
        match session {
            None => Ok(addr),
            Some(name) => {
                let building = city::Building::of(&addr)?;
                city::open_room(&self.city_root, building.addr(), name)
            }
        }
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
