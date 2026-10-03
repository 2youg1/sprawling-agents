// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! What a dispatch, a fork and a freeze record about a run.

use serde::{Deserialize, Serialize};

use crate::completion::Completion;
use crate::event::identity::{RunId, Seq};
use crate::event::kind::EventKind;
use crate::event::who::Who;
use crate::locator::{B3Hash, Locator};
use crate::model::{Effort, RunPolicy};
use crate::origin::Origin;

/// One skill a run was dispatched with, pinned to the bytes it read.
///
/// The hash is the point: a page answers "which runs read these exact
/// bytes" from it, and a pin naming a name alone cannot answer that.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct SkillPin {
    pub name: String,
    #[cfg_attr(feature = "schema", schemars(with = "String"))]
    pub hash: B3Hash,
}

/// `run_started`: the work a run was given, and where it came from.
///
/// Every field defaults, because the golden fixtures hold a
/// `run_started` whose payload is `{}` and a projection that refused it
/// would refuse a ledger this repository ships.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct RunStarted {
    /// What the person asked for, in their words.
    #[serde(default)]
    pub task: String,
    /// What finishing looks like, in their words.
    #[serde(default)]
    pub goal: String,
    /// The job this dispatch came out of. Absent only in a record
    /// written before the key existed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "schema", schemars(with = "Option<String>"))]
    pub job: Option<Locator>,
    /// The run that forked this one, when one did.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent: Option<RunId>,
    /// The run this one replaced, when it is a successor. Spelled the
    /// same way here and on `checkpoint_committed`, because it is one
    /// fact with one spelling.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub predecessor: Option<RunId>,
    /// Written even when empty, and deliberately: a key that comes and
    /// goes is a shape a reader has to guess at, and "this building
    /// admits no skills" is a fact worth recording rather than an
    /// absence to infer.
    #[serde(default)]
    pub skills: Vec<SkillPin>,
    /// Who dispatched this run: the person, the city's own desk (a
    /// plan, the schedule, an arrival), or the resident that delegated,
    /// succeeded itself or knocked. The line's author cannot say it,
    /// because the city's desk writes every `run_started`. Absent only
    /// in a record written before the key existed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "schema", schemars(with = "Option<String>"))]
    pub dispatched_by: Option<Who>,
    /// The run policy this run was dispatched under (`crates/kernel/spec/Model.lean` §8-77).
    /// Absent in a record written before the policy was recorded; the
    /// six mode words before it never reached a typed payload, so
    /// there is no older shape to read.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub policy: Option<RunPolicy>,
    /// The identity version this run's session froze: the key a page
    /// reads the names the request carried back by, through the content
    /// store (`crates/kernel/spec/Event/Record.lean` §8-79). Absent in a record written before the
    /// key existed, which a page answers with the address rather than
    /// with today's names.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "schema", schemars(with = "Option<String>"))]
    pub naming: Option<B3Hash>,
    /// How the run's first user message was written (`crates/kernel/spec/Event/Record.lean` §8-82-1):
    /// what a fork reads to rebuild that message the way the mother sent
    /// it. Absent in a record written before the key existed, and on a
    /// harness run, whose first words are the harness's own prompt.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub opening: Option<Opening>,
    /// The effort the run's requests froze (`crates/kernel/spec/Event/Record.lean`
    /// §8-85): the session's, which no turn may change. Absent when the
    /// dispatch stated none and the provider chooses - which is not
    /// [`Effort::None`], asking it not to think - on a harness run, and
    /// in a record written before the key existed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub effort: Option<Effort>,
}

/// How a run's first user message opens.
///
/// Exhaustive, and the choice is made once by the city that wrote (or
/// did not write) the job file. It is not a formatting preference: a
/// session working from an assignment and a session talking with the
/// person want different first words, and inferring which from an empty
/// string would make the emptiness of a goal mean two things.
///
/// It lives here rather than beside the conversation that writes it
/// because `run_started` records it, and the ledger's payloads are this
/// crate's; `runtime::conversation` re-exports it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum Opening {
    /// Somebody wrote the task down; the job file's text is in the prefix.
    FromJob,
    /// Somebody wrote the task down for a mother, and a branch is
    /// rebuilding her conversation: her job file lives in her room, not
    /// in the branch's prefix, so the task travels in the message.
    Inherited,
    /// Nobody did; the person is on the other side of this message.
    WithPerson,
}

/// `run_frozen`: how a run ended, and what it cites for having ended
/// that way.
///
/// The three endings are spelled by [`Completion::name`], which stays
/// the authority for the words: this struct carries whichever word that
/// method printed rather than an enum that would have to agree with it.
/// A run that ended any other way cites nothing, and the key is absent
/// rather than empty, exactly as the hand-written writer left it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct RunFrozen {
    pub completion: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub evidence: Option<Vec<EvidenceCite>>,
    /// Why the line was written by something other than the run's own
    /// driver (`crates/kernel/spec/Event/Record.lean` §8-82-2). Absent on every freeze a run wrote
    /// for itself, so those lines keep the bytes they always had.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cause: Option<FreezeCause>,
}

/// Why a run was frozen by something other than its own driver.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum FreezeCause {
    /// The process driving it died; the next `resume` froze it.
    ProcessDied,
}

impl RunFrozen {
    /// The one projection from a verdict to the line that records it.
    ///
    /// Reading is deliberately not the inverse: a [`Completion::Done`]
    /// holds [`EventRef`](crate::EventRef)s, and minting those from a
    /// line would forge the evidence the type exists to guarantee.
    pub fn of(completion: &Completion) -> RunFrozen {
        let evidence = match completion {
            Completion::Done(evidence) => Some(
                evidence
                    .refs()
                    .iter()
                    .map(|cited| EvidenceCite {
                        seq: cited.seq(),
                        kind: cited.kind(),
                    })
                    .collect(),
            ),
            Completion::Limit | Completion::Cancelled => None,
        };
        RunFrozen {
            completion: completion.name().to_owned(),
            evidence,
            cause: None,
        }
    }

    /// The freeze of a run whose process died before it froze itself:
    /// the one place that says which ending such a run has.
    ///
    /// `cancelled`, because the run stopped without finishing - `done`
    /// needs evidence the city recorded and it has none - and no ceiling
    /// cut it, which is what `limit` says. The cause tells it apart from
    /// a cancel somebody asked for (kernel D14).
    pub fn lost() -> RunFrozen {
        RunFrozen {
            completion: Completion::Cancelled.name().to_owned(),
            evidence: None,
            cause: Some(FreezeCause::ProcessDied),
        }
    }
}

/// One line a finished run cites as the evidence it finished.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct EvidenceCite {
    pub seq: Seq,
    pub kind: EventKind,
}

/// `session_opened`: a new session began at this address.
///
/// The room is the subject rather than the run, because what changed is
/// what the *next* run there is governed by: the model and the effort
/// the previous session froze are gone, and whether the summary it left
/// travels with this one is the one thing the caller chose. A replay
/// reads the answer to "why did this run start fresh" from here;
/// neither fact is visible in the room's files afterwards, because both
/// were removals.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct SessionOpened {
    /// What the session branched from, when it is a branch.
    ///
    /// `#[serde(default)]` and absent when there is none, so a line
    /// written before this key existed reads as a session that began
    /// without one - which is what it was.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub from: Option<Origin>,
    /// Whether the previous session's handoff travelled with it.
    ///
    /// **The room itself is the record's own `addr` field**, not a copy
    /// here: a line that belongs to an address says so in the envelope,
    /// and a payload copy would be a second place the same address is
    /// spelled (`crates/kernel/spec/Event.lean` §8-4).
    pub carried: bool,
}

/// `run_forked`: which run this one continues, and from where in it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct RunForked {
    /// The run whose history this one resumes.
    pub from: RunId,
    /// The last sequence number of `from` that this run inherits.
    pub at_seq: Seq,
}

/// `eval_run`: the handoff probe asked of a successor, compared with
/// what its predecessor answered.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct EvalRun {
    /// The probe's name and version, which together fix its questions.
    pub probe: String,
    pub version: u32,
    /// The run whose answers are the reference.
    pub predecessor: RunId,
    /// How many answers the successor kept.
    pub kept: u32,
    /// The question indices whose answers differ, in order.
    pub lost: Vec<u32>,
    /// The predecessor's answers and the successor's, one per question.
    pub before: Vec<String>,
    pub after: Vec<String>,
}

/// `run_policy_changed`: the room's run policy, changed while a session
/// is open (`crates/kernel/spec/Event/Record.lean` D21).
///
/// The run under way reads it at its next safe point and passes the
/// gate under it from that step on; the next run's `run_started.policy`
/// is the room's last change. The model and the effort stay what the
/// session froze.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct RunPolicyChanged {
    pub policy: RunPolicy,
    #[cfg_attr(feature = "schema", schemars(with = "String"))]
    pub by: Who,
}

/// `session_named`: the name a person gave one session of a room
/// (`crates/kernel/spec/Event/Record.lean` D22).
///
/// The address stays the session's identity; the name is what a page
/// shows. `began` is the seq that opened the session, the same value
/// `wire::SessionLine.began` carries; an empty name takes the name back,
/// and the last line for a session is the one that holds.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct SessionNamed {
    pub began: Seq,
    pub name: String,
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "test code"
)]
mod tests;
