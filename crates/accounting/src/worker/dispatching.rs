// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Who gets woken, and where the work lands.

use kernel::{Address, AxError, Locator, RunId};

/// A resident who was signalled and has no run open.
///
/// The second of the two ways a signal reaches somebody. The first
/// slips under the door of a run that is already working - a steer-kind
/// signal, landing at that run's next safe point with the sender's
/// address in front of it. This one knocks: it starts a run for a
/// resident who is not working, because a message nobody is there to
/// read is the same as no message.
/// What one dispatch is, as the caller asked for it.
///
/// Four of these reach every phase below together and are never chosen
/// independently: standing the run up, laying out its bench, freezing
/// its plan, settling its desks and concluding it all name the same
/// address, run policy and ceiling. Passed side by side they were four
/// parameters on six signatures, and the depth had to be derived twice.
///
/// The other two are spent in `stage_dispatch`'s prologue and never seen
/// again, and they are here rather than beside it because opening a room
/// is the first thing this city writes for a dispatch. A caller that
/// opened it would put the rule "agree before you write" in a second
/// place, and the entrance that dispatches without a person - a knock, a
/// schedule, a delegate - is the one that would not hold it.
pub(super) struct Assignment {
    /// Where the work was sent: a building when a room is still to be
    /// opened for it, a room when there already is one.
    pub(super) addr: Address,
    /// The room to open under that address, when the caller named a
    /// session. The prologue spends it, and that is the line where
    /// `addr` stops being what was asked for and becomes where the run
    /// works.
    pub(super) session: Option<kernel::SessionName>,
    /// How hard the runs in that room think from here on, when the
    /// caller said. It is written into the room's own configuration
    /// layer, so it cannot be answered before the room exists.
    pub(super) effort: Option<kernel::Effort>,
    /// The registered model this dispatch named by id, when it named
    /// one; `None` runs on the `main` tag's model. Spent by agreeing,
    /// which finds the tag that registered it (`crates/sprawling/Spec.lean` §8-10).
    pub(super) model: Option<String>,
    /// The run policy the dispatch chose (`crates/kernel/Spec.lean` §8-77): written
    /// into `run_started`, asked by the merge, and inherited whole by
    /// every run this one hands work to or knocks for.
    pub(super) policy: kernel::RunPolicy,
    /// The run that handed this work down, when somebody did.
    pub(super) parent: Option<RunId>,
    /// The run this one replaces, when it is a successor, and what that
    /// run answered the handoff probe. Distinct from `parent` on
    /// purpose: a successor inherits its predecessor's parent and
    /// therefore its depth, which is what makes succession a different
    /// verb from delegation.
    pub(super) succession: Option<Handover>,
    /// The outside sources this run began with, empty for work that
    /// began inside the city. Every door the run's bench asks reads it,
    /// and a non-empty set keeps its approvals from being waived by a
    /// policy (C15).
    ///
    /// Carried by the dispatch rather than held by the worker: a run is
    /// settled long after the entrance that started it returned, so a
    /// flag the worker set and cleared around one call described
    /// whichever run happened to be landing.
    pub(super) taint: kernel::TaintSet,
    /// What this session branched off, when it did. Set by the dispatch
    /// from the worker's own fold of `session_opened`, spent by the first
    /// run of that session and by no later one: a branch is a beginning,
    /// and the run that begins it is the one that inherits.
    pub(super) origin: Option<kernel::Origin>,
    /// Who sent this work, recorded in `run_started` because that
    /// line's author is always the city's desk.
    pub(super) dispatched_by: kernel::event::Who,
}

/// One succession, as the successor's dispatch receives it: who is
/// being replaced, and what they answered before handing over. The two
/// travel together because the probe's second reading is meaningless
/// without its first.
pub(super) struct Handover {
    pub(super) predecessor: RunId,
    pub(super) before: super::probing::probe::Answers,
}

impl Assignment {
    pub(super) fn predecessor(&self) -> Option<RunId> {
        self.succession.as_ref().map(|handed| handed.predecessor)
    }

    /// Derived from whether somebody handed this work down, rather than
    /// carried beside it. Two values that must agree are two chances to
    /// disagree, and the one that disagrees here is a grand-delegate.
    pub(super) fn depth(&self) -> kernel::Depth {
        match self.parent {
            None => kernel::Depth::Root,
            Some(_) => kernel::Depth::Delegated,
        }
    }
}

/// What the run was given: the brief on disk, the words it was written
/// from, and the pin of the bytes the run segment carried.
///
/// One value because the four are one act of writing: the brief is
/// rendered from the task and the goal, and the locator pins the brief's
/// own bytes. Two of the four reaching a phase without the others would
/// let the prompt and the history disagree about what was asked.
pub(super) struct Given {
    pub(super) brief: city::RunBrief,
    pub(super) task: String,
    pub(super) goal: String,
    pub(super) job: Locator,
}

impl super::RunWorker {
    /// Writes what the run was given: the brief on disk and its bytes
    /// pinned in the store.
    ///
    /// The task file exists first, then the run exists: the job on disk
    /// is what the agent reads, and the copy in the store is what the
    /// history keeps, so editing one cannot rewrite the other.
    ///
    /// # Errors
    /// Propagates a room that will not take the brief and a store that
    /// will not take its bytes.
    pub(super) fn give(
        &mut self,
        at: &Assignment,
        task: String,
        goal: String,
    ) -> Result<Given, AxError> {
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
        Ok(Given {
            job: Locator::cas(job_hash),
            brief,
            task,
            goal,
        })
    }
}

/// What the city agreed to before it wrote anything for this dispatch.
///
/// Every refusal a dispatch can owe cheaply is answered to produce this
/// value, and none of it touches the city: the four fields are read from
/// files a person wrote and from the endpoint book the ledger folds. It
/// exists so that the answer is computed once - `stand_up` takes it
/// apart into the `Site` rather than asking the same questions again,
/// which would put a second authority behind a credential renewal that
/// may reach the network.
pub(super) struct Agreed {
    pub(super) building: city::Building,
    pub(super) rules: city::BuildingRules,
    pub(super) model: gateway::ModelEntry,
    /// The name of the endpoint the model is reached through, which is
    /// the provider a model's note is filed under (`crates/sprawling/Spec.lean` §8-85).
    pub(super) provider: String,
    pub(super) adapter: super::keeping_warm::Door,
    /// How many times this run may make a failed call again, as the
    /// person set it on the endpoint that was chosen. Read here, where
    /// the endpoint is chosen, because nothing downstream sees the book.
    pub(super) retries: kernel::Retries,
}

/// Who the city agreed to seat in the room: decided once, in
/// `agree_to_work`, before anything is written (`crates/sprawling/Spec.lean` §8-124).
///
/// Every later phase that differs between the two residents matches
/// this, and nothing downstream asks the configuration again.
pub(super) enum Seat {
    /// A model this city calls, chosen and credentialed.
    Model(Agreed),
    /// An official harness that takes the turn itself.
    Harness(HarnessSeat),
}

/// What the city agreed to for a room whose resident is a harness.
pub(super) struct HarnessSeat {
    pub(super) building: city::Building,
    pub(super) rules: city::BuildingRules,
    pub(super) harness: agent_protocols::Harness,
}

impl Seat {
    /// The building the run stands in, whoever its resident is.
    pub(super) fn building(&self) -> &city::Building {
        match self {
            Seat::Model(agreed) => &agreed.building,
            Seat::Harness(seat) => &seat.building,
        }
    }
}

pub(super) struct Knock {
    pub(super) addr: Address,
    /// Who spoke, as they will be named in the woken run's own brief.
    pub(super) from: String,
    /// The run policy of the run that spoke. Carried rather than
    /// defaulted: an answer belongs to the same piece of work as the
    /// question, and a knock must not widen what the speaker could
    /// write or lift what it had to prove.
    pub(super) policy: kernel::RunPolicy,
    /// Where the run that spoke stood in its conversation. The woken run
    /// is one hop further on, and both ceilings are read there
    /// (`crates/sprawling/Spec.lean` §8-46-12).
    pub(super) chain: super::KnockChain,
}

/// What one dispatch left behind. Carried rather than re-derived,
/// because the run that asked for the work has to be told how it ended
/// and the ledger is not a thing this layer reads back mid-command.
pub(crate) struct Dispatched {
    pub(super) run: RunId,
    pub(super) addr: Address,
    pub(super) who: String,
    pub(super) completion: kernel::Completion,
}

pub(super) mod agreeing;
pub(super) mod custody;
pub(super) mod handback;
pub(super) mod harness;
pub(super) mod preparing;
pub(super) mod running;
pub(super) mod session;
pub(super) mod session_shape;
pub use agreeing::acp_dispatch;
pub(super) use agreeing::run_id_for;
#[cfg(test)]
mod tests;
