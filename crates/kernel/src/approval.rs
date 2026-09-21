// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The Approval Inbox holds design questions and nothing else (9.2).
//!
//! An action is either allowed by a rule or refused by one, and both
//! answers are the gates' (§8-27); what is left for a person is the
//! question a run cannot answer by reading the rules. So this module
//! carries one class of item, two autonomy states, and the rule for who
//! may answer. There is no standing waiver here: a waiver waives an
//! escalation, and no gate escalates any more.

use serde::{Deserialize, Serialize};

use crate::event::{RunId, Seq, TimeMs};
use crate::locator::Locator;
use crate::registry::ResidentId;

/// Non-empty item identity, derived from the run that raises the item
/// and that run's own position counter — never from a clock. Four lanes
/// drive at once by default, so two runs reach their first question in
/// one millisecond routinely; a clock-shaped identity makes those two
/// items one key, the inbox keeps the later one, and the earlier one
/// disappears from an append-only history that cannot afterwards tell a
/// lost item from an item that was never raised.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct ApprovalId(String);

impl ApprovalId {
    /// Mints the identity of the question `run` may raise at `seq`.
    ///
    /// `seq` is the run's own monotonic position — the counter the run
    /// already keeps to derive [`crate::IdemKey`], counted inside one run
    /// and never across runs, so a position is only ever an identity
    /// together with the run it was counted in. Replay re-derives the
    /// identical id because neither input is sampled: this is what keeps
    /// `just replay` self-proving over the approval records.
    ///
    /// The position is written zero-padded to the width of `u64::MAX`, so
    /// the derived `Ord` on two ids of one run reads the order the run
    /// raised them in.
    pub fn of(run: &RunId, seq: Seq) -> ApprovalId {
        ApprovalId(format!("ap-{run}-{:020}", seq.value()))
    }

    /// Mints the identity of the one question a drive's own sweep may
    /// raise, which happens at most once per run.
    ///
    /// The sweep sits at the highest position, which no call position can
    /// reach, because the sweep counts nothing and the run's call counter
    /// must stay free to hand out every position it reaches.
    pub fn of_sweep(run: &RunId) -> ApprovalId {
        ApprovalId::of(run, Seq::new(u64::MAX))
    }

    /// Adopts an id that already exists — one echoed back over the wire,
    /// one read from the ledger, one written into a fixture. Minting is
    /// [`ApprovalId::of`]; this path only refuses the empty string.
    pub fn new(raw: impl Into<String>) -> Option<ApprovalId> {
        let raw = raw.into();
        if raw.is_empty() {
            None
        } else {
            Some(ApprovalId(raw))
        }
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// What kind of decision this is. Wire data — the cluster keys
/// serialize — and one arm, because a run asks a person exactly one
/// kind of thing: a question about the design it cannot settle by
/// reading the rules.
///
/// The enum survives its own single arm on purpose. The cluster key is
/// wire data, and a class named in the payload keeps the day a second
/// kind of question appears a compile error at every reader rather than
/// a silent change of meaning for a field that used to say one thing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum ApprovalClass {
    Question,
}

/// The clustering key: class + free detail. One answer covers the
/// cluster a person was shown, and it expires with the process.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct ClusterKey {
    pub class: ApprovalClass,
    pub detail: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct ApprovalItem {
    pub id: ApprovalId,
    pub actor: String,
    pub action_desc: String,
    pub artifact: Locator,
    pub cluster_key: ClusterKey,
    pub created: TimeMs,
    /// Whether the run that raised this question started from content
    /// that came in from outside. Shown to the person answering, and
    /// decided on by nobody: what taint refuses is an effect, at
    /// [`crate::gate::undoable`], and a question is not an effect.
    pub tainted: bool,
}

/// How a person answers one item.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum Ruling {
    Allow,
    Deny,
}

/// Who answers the Approval Inbox (9.2). Two states, because a question
/// either waits for the person or goes straight to the resident they
/// appointed; there is no third state in which nobody answers, since
/// "nobody answered yet" is what an unanswered item already says.
///
/// Never touches gate decisions: the gates answer from the rules, and
/// who reads the inbox cannot change what a rule says.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum Autonomy {
    Owner,
    Delegate(ResidentId),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Answerer {
    Human,
    Resident(ResidentId),
}

/// Deliberately exhaustive.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnswerVerdict {
    May,
    /// Only the appointed delegate answers; nobody else's ruling counts.
    NotTheDelegate,
    /// Self-approval is no approval.
    SelfApprovalBarred,
}

/// The answering rule. A person answers everything; a resident answers
/// only as the appointed delegate, and never its own question.
pub fn may_answer(autonomy: &Autonomy, item: &ApprovalItem, answerer: &Answerer) -> AnswerVerdict {
    match answerer {
        Answerer::Human => AnswerVerdict::May,
        Answerer::Resident(resident) => {
            let appointed = matches!(autonomy, Autonomy::Delegate(d) if d == resident);
            if !appointed {
                return AnswerVerdict::NotTheDelegate;
            }
            if item.actor == resident.as_str() {
                return AnswerVerdict::SelfApprovalBarred;
            }
            AnswerVerdict::May
        }
    }
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
