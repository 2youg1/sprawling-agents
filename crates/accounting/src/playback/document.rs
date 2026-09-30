// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The playback bundle's schema (accounting-SPEC.md 8-12): every field,
//! in the order `encode` writes it. A field added here is a field the
//! bundle carries, so a change to this file moves `PROJECTION_RULES`.
//!
//! Every u64 travels as a [`Decimal`] string, so no reader parses a seq,
//! a moment or an amount through a floating-point number.

use kernel::{Address, B3Hash, EventKind, GitOid, Locator, RunId};
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Checkpoint {
    pub(super) seq: Decimal,
    pub(super) run: RunId,
    pub(super) holds: Holds,
}

/// The two facts `checkpoint_committed` carries, kept apart.
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
    },
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
