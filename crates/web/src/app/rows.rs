// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! One run as a row, and what the model calls consumed.

use channels::{Address, ApprovalItem, RunId, Seq, Tokens, UsdMicros};

use crate::lang::Msg;
use crate::phase::Phase;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ProviderHealth {
    #[default]
    Unknown,
    Healthy,
    Degraded,
    Lost,
}

impl ProviderHealth {
    /// The word shown to a person.
    ///
    /// A `Msg` and not a `&str`, which is the whole point: the doc here
    /// used to say "microcopy has one authority" while being a second one,
    /// in English only. Both callers put the result in a slot, so a
    /// Chinese page rendered `provider 状态：unknown` - the variant's own
    /// name, straight out of the enum. `web::lang` makes a missing
    /// translation unrepresentable, and a slot filled with a `&'static
    /// str` is how that guarantee was got around.
    #[must_use]
    pub fn word(self) -> Msg {
        match self {
            Self::Unknown => Msg::ProviderUnknown,
            Self::Healthy => Msg::ProviderHealthy,
            Self::Degraded => Msg::ProviderDegraded,
            Self::Lost => Msg::ProviderLost,
        }
    }
}

/// What a Run looks like in a list. Progress is carried as the two counts
/// the Ledger reports rather than a percentage: the ratio is the view's to
/// render, and a stored percentage would be a second place for it to be
/// wrong.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunRow {
    pub addr: Option<Address>,
    /// What a person called this piece of work. Folded from the address
    /// the run opened in, because the room *is* the name: `Dispatch`
    /// takes a session name and the city opens a room of that name, so
    /// the last segment of the address is the word a person typed.
    pub session: Option<String>,
    /// The run that handed this work down, when one did. Folded from
    /// `run_started`, so a page and an offline replay draw the same
    /// tree.
    pub parent: Option<RunId>,
    pub phase: Phase,
    pub steps_done: u32,
    pub steps_planned: Option<u32>,
    pub started_at_seq: Seq,
    /// The last record this client folded for this run. What the session
    /// page needs in order to say how long ago something happened, and
    /// what `Fork` needs in order to name a point a person can mean.
    pub last_seq: Seq,
    /// How many turns have closed. A turn is a model call and whatever
    /// it caused, so this counts `model_returned` rather than records.
    pub turns: u32,
    /// The turn at which a handoff was last written, if one ever was.
    pub handoff_at_turn: Option<u32>,
    /// Which gate is holding this run, when one is. Named by the gate,
    /// not by the request: a person asked "what is it stuck on" and the
    /// answer is a door, not an identifier.
    pub gate: Option<String>,
    /// What this run has cost so far, in the two halves that are
    /// separately knowable. `None` where no call has settled an amount.
    pub spent: Option<UsdMicros>,
    /// The last thing the model said, trimmed to one line. What a row in
    /// the list shows so that a person can tell two sessions apart
    /// without opening either.
    pub said: Option<String>,
    /// What the person asked for, as they typed it.
    ///
    /// Folded from `run_started`, where it has been on the wire all
    /// along and no page read it. A session page that shows what a run
    /// spent, which door holds it and what it last said, and never the
    /// sentence it was given, is asking a reader to judge an answer
    /// without the question - and the question is the one thing on that
    /// page a person wrote themselves.
    pub task: Option<String>,
}

/// What the model calls consumed, and what the city may not claim to know
/// about their price.
///
/// Money and tokens are separate fields because they are separately
/// knowable: every call reports tokens, and only a call whose provider or
/// price sheet settled it reports money. A subscription reports neither
/// price nor bill, and rendering that as `$0.00` would be the interface
/// inventing a fact - zero and unknown are different, and only one of
/// them is true.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Usage {
    pub input: Tokens,
    pub output: Tokens,
    pub cache_read: Tokens,
    /// Calls that came back with a settled amount.
    pub priced_calls: u32,
    /// Calls that came back with tokens and no amount.
    pub unpriced_calls: u32,
}

/// The name a person gave this piece of work, out of the address the run
/// opened in.
///
/// `Dispatch` carries a session name and the city opens a room of that
/// name, so the last segment of a room address is the word somebody
/// typed. A bare building is not a session: work sent to `lab` with no
/// name is answered in a room the city named, and reporting the
/// building as the session name would put every unnamed run under one
/// heading.
pub(crate) fn session_named_by(addr: &Address) -> Option<String> {
    let raw = addr.as_str();
    let (building, room) = raw.rsplit_once('/')?;
    (!building.is_empty() && !room.is_empty()).then(|| room.to_owned())
}

/// Which door a run is stopped at, named by the door.
///
/// A person who asks what a session is stuck on wants the gate, not the
/// identifier of the request: `ApprovalId` is unique and says nothing,
/// while the class is the one word that tells them whether this is
/// theirs to answer now or later.
pub(crate) fn gate_named_by(item: &ApprovalItem) -> String {
    match item.cluster_key.class {
        channels::ApprovalClass::Commitment => "commit",
        channels::ApprovalClass::BudgetLimit => "budget",
        channels::ApprovalClass::DiscardEscalate => "discard",
        channels::ApprovalClass::AgentQuestion => "question",
        channels::ApprovalClass::Governance => "governance",
        channels::ApprovalClass::Delegation => "delegation",
        // The class set is open on the wire, and a class this build has
        // no word for is still a door this run is stopped at. Saying so
        // beats saying nothing: the person learns the session needs them
        // and the page they open names the request in full.
        _ => "a gate this build cannot name",
    }
    .to_owned()
}
