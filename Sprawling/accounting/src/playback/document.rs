// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The playback bundle's schema (`crates/accounting/spec/Playback.lean` §8-12): every field,
//! in the order `encode` writes it. A field added here is a field the
//! bundle carries, so a change to this file moves `PROJECTION_RULES`.
//!
//! Every u64 travels as a [`Decimal`] string, so no reader parses a seq,
//! a moment or an amount through a floating-point number.

use kernel::{Address, B3Hash, EventKind, GitOid, Locator, RunId, RunPolicy};
use serde::{Deserialize, Serialize};

/// A u64 written as its decimal digits, with no sign and no leading zero.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct Decimal(pub(super) u64);

impl Serialize for Decimal {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.collect_str(&self.0)
    }
}

impl<'de> Deserialize<'de> for Decimal {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let raw = String::deserialize(deserializer)?;
        let canonical =
            raw.bytes().all(|byte| byte.is_ascii_digit()) && (raw == "0" || !raw.starts_with('0'));
        match raw.parse::<u64>() {
            Ok(value) if canonical => Ok(Decimal(value)),
            Ok(_) | Err(_) => Err(serde::de::Error::custom(format!(
                "'{raw}' is not a decimal u64 written without sign or leading zero"
            ))),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Document {
    pub(super) schema: String,
    pub(super) source: Source,
    pub(super) events: Vec<Event>,
    pub(super) context: Vec<Event>,
    pub(super) unknown: Vec<Decimal>,
    pub(super) runs: Vec<Run>,
    pub(super) moments: Vec<Moment>,
    pub(super) messages: Vec<Message>,
    pub(super) calls: Vec<Call>,
    pub(super) checkpoints: Vec<Checkpoint>,
    pub(super) costs: Costs,
    pub(super) withheld: Withheld,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Source {
    pub(super) city: B3Hash,
    pub(super) selection: Chosen,
    pub(super) cutoff: CutoffLine,
    pub(super) rules: u32,
    pub(super) reader: ReaderName,
}

/// The selection exactly as it was given, before the cutoff bounded it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Chosen {
    pub(super) from: Option<Decimal>,
    pub(super) through: Option<Decimal>,
    pub(super) run: Option<RunId>,
    pub(super) building: Option<Address>,
    /// The span's start and end, in milliseconds; a day is written as
    /// its two ends.
    pub(super) since: Option<Decimal>,
    pub(super) until: Option<Decimal>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct CutoffLine {
    pub(super) seq: Decimal,
    pub(super) chain_hash: B3Hash,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub(super) enum ReaderName {
    Person(ConfidentialName),
    Resident(Address),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum ConfidentialName {
    Withheld,
    Included,
}

/// One Ledger line: its seq, its own moment when it records one, and the
/// line byte for byte.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Event {
    pub(super) seq: Decimal,
    pub(super) moment: Option<Decimal>,
    pub(super) line: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Run {
    pub(super) run: RunId,
    pub(super) addr: Option<Address>,
    pub(super) session: Option<Decimal>,
    pub(super) parent: Option<Related>,
    pub(super) forked_at: Option<Decimal>,
    pub(super) predecessor: Option<Related>,
    pub(super) first_seq: Decimal,
    pub(super) last_seq: Decimal,
    pub(super) state: Option<Phase>,
    pub(super) unanswered: Decimal,
    /// The policy its `run_started` recorded; `None` for a line written
    /// before policies were, or one the reader may not see.
    pub(super) policy: Option<RunPolicy>,
}

/// Another run one run points at.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub(super) enum Related {
    Run(RunId),
    Withheld,
    Missing,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum Phase {
    Active,
    Frozen,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum Family {
    Run,
    Approval,
    Pr,
}

/// One end of a key moment or a message.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub(super) enum End {
    At(Decimal),
    Outside(Decimal),
    Withheld,
    Pending,
    Missing,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Moment {
    pub(super) family: Family,
    pub(super) key: String,
    pub(super) opened: End,
    pub(super) closed: End,
    pub(super) seqs: Vec<Decimal>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Message {
    pub(super) id: String,
    pub(super) from: Option<String>,
    pub(super) room: Option<Address>,
    pub(super) sent: End,
    pub(super) consumed: End,
}

/// One call, a tool call or a model attempt, in the run its two lines
/// are paired in.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Call {
    pub(super) run: RunId,
    pub(super) callee: Callee,
    pub(super) called: End,
    pub(super) answered: End,
    pub(super) took: Took,
}

/// What was called, and its name when the calling line is visible.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub(super) enum Callee {
    /// A tool, by the id its call and its answer share.
    Tool { id: String, name: Option<String> },
    /// A model, by the model id the request spelled; the attempt itself
    /// is the call's `called` end.
    Model { name: Option<String> },
}

/// How long a call took: measured milliseconds, or not known. Never a
/// zero standing in for a moment nobody measured.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub(super) enum Took {
    Measured(Decimal),
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Checkpoint {
    pub(super) seq: Decimal,
    pub(super) run: RunId,
    pub(super) holds: Holds,
}

/// The two facts `checkpoint_committed` carries, and the commit a
/// `pr_merged` landed, kept apart.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub(super) enum Holds {
    Pinned {
        job: Locator,
    },
    Committed {
        oid: GitOid,
        scope: Vec<String>,
        files: Vec<String>,
        base: Base,
        diff: Vec<FileDiff>,
        trace: CallTrace,
    },
    Merged {
        oid: GitOid,
    },
}

/// What a commit is compared with: the same run's previous commit, its
/// one parent, or nothing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub(super) enum Base {
    Previous(GitOid),
    Parent(GitOid),
    #[serde(rename = "none")]
    Absent,
}

/// One path a commit names, and what became of it between the base and
/// the commit.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct FileDiff {
    pub(super) path: String,
    pub(super) change: Change,
}

/// The six things a file's diff can be, kept apart.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub(super) enum Change {
    Patch {
        lines: Vec<DiffLine>,
        credential: Vec<HiddenLine>,
    },
    /// The head of the patch that fit in what was left of the budget, and
    /// how many lines did not.
    Truncated {
        lines: Vec<DiffLine>,
        credential: Vec<HiddenLine>,
        cut: Decimal,
    },
    Empty,
    Binary,
    Missing,
    Withheld,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct DiffLine {
    pub(super) number: Decimal,
    pub(super) text: String,
}

/// A patch line the credential scan matched: where it was and why, never
/// its bytes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct HiddenLine {
    pub(super) number: Decimal,
    pub(super) reason: String,
}

/// The calls a commit is the result of, as `accounting::trace` answers.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub(super) enum CallTrace {
    Traced {
        calls: Vec<Cited>,
        nearby: Vec<Near>,
    },
    /// The trace knows no such commit, or answers another announcement of
    /// the same oid.
    Untraced,
    /// The trace failed, with this code.
    Unread(String),
}

/// One call a trace names, as the reader may see it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub(super) enum Cited {
    /// In `events`.
    At(Decimal),
    /// Visible, outside the selection; its line is not carried.
    Elsewhere(Decimal),
    Withheld,
}

/// Another run that called tools in the commit's building in the span.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Near {
    pub(super) run: Related,
    /// Where it called from, when the reader may see the run.
    pub(super) actor: Option<Address>,
    pub(super) calls: Decimal,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Costs {
    pub(super) billed_usd_micros: Decimal,
    pub(super) by_run: Vec<Billed>,
    pub(super) unpriced_calls: Decimal,
    pub(super) unpriced_tokens: Decimal,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Billed {
    pub(super) run: String,
    pub(super) usd_micros: Decimal,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Withheld {
    pub(super) events: Decimal,
    pub(super) kinds: Vec<KindCount>,
    pub(super) buildings: Vec<Closed>,
    pub(super) credential: Decimal,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct KindCount {
    pub(super) kind: EventKind,
    pub(super) count: Decimal,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Closed {
    pub(super) building: Address,
    pub(super) reason: Reason,
}

/// Why a building was closed to the reader: the two closing arms of
/// `kernel::ReadVerdict`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum Reason {
    Confidential,
    RulesUnreadable,
}
