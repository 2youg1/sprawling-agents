// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Who gets woken, and where the work lands.

use kernel::{Address, Locator, Model, RunId};

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
/// address, mode and ceiling. Passed side by side they were four
/// parameters on six signatures, and the depth had to be derived twice.
///
/// The other two are spent in `dispatch_in`'s prologue and never seen
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
    pub(super) mode: runtime::Mode,
    pub(super) budget: kernel::BudgetCap,
    /// The run that handed this work down, when somebody did.
    pub(super) parent: Option<RunId>,
}

impl Assignment {
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
    pub(super) adapter: Box<dyn Model + Send>,
}

pub(super) struct Knock {
    pub(super) addr: Address,
    /// Who spoke, as they will be named in the woken run's own brief.
    pub(super) from: String,
    /// The mode and the spending ceiling of the run that spoke. Carried
    /// rather than defaulted: an answer belongs to the same piece of
    /// work as the question, and a run with no ceiling is the one
    /// failure with no floor under it.
    pub(super) mode: runtime::Mode,
    pub(super) budget: kernel::BudgetCap,
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

/// What the digest model is told when it is asked to name a piece of
/// work.
///
/// Short on purpose, and it states the shape of a legal answer rather
/// than trusting one: `SessionName::parse` is the authority and refuses
/// anything with a separator in it, so a prompt that did not say "one
/// segment" would spend a call to be refused.
pub(super) const NAME_THE_WORK: &str = "Name this piece of work in two to four words, joined by hyphens, in \
     lowercase ASCII. Answer with the name alone: no path, no quotes, no \
     explanation. Example: refactor-ledger-reads";

/// How many tokens a name is worth. Four words do not need more, and a
/// ceiling is what stops a model that decided to explain itself from
/// costing a person real money for a filename.
pub(super) const NAME_TOKENS: u64 = 32;

/// How many turns one dispatch may take before it freezes at `Limit`.
/// A budget the caller cannot set yet is still a budget: an unbounded
/// loop against a paid provider is the one failure with no ceiling.
pub(super) const DISPATCH_TURN_BUDGET: u32 = 24;

pub(super) mod agreeing;
pub(super) mod running;
pub(crate) use agreeing::acp_dispatch;
pub(super) use agreeing::run_id_for;
#[cfg(test)]
mod tests;
