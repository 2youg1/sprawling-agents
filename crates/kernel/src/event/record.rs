// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! One serde struct per [`EventKind`](super::EventKind): the single
//! authority for what a kind's payload is called and what it holds.
//!
//! Writers reach a payload through [`Payload::of`](super::Payload::of)
//! and readers through [`Payload::read`](super::Payload::read). Those
//! two doors plus these structs replace the hand-written
//! `insert("key")` at the writer and `get("key")` at each reader, which
//! is how one key came to be spelled two ways in two crates.
//!
//! Three rules hold for every struct here, because a ledger already
//! written cannot be re-spelled:
//!
//! 1. **The bytes do not move.** Field names are the keys the previous
//!    hand-written writer used, an absent key is spelled
//!    `#[serde(skip_serializing_if = ...)]` exactly where the previous
//!    writer omitted it, and a key that writer emitted unconditionally
//!    stays unconditional even when it is empty. Keys sort on the way
//!    out (serde_json's `Map` is a `BTreeMap`), so declaration order
//!    here changes nothing.
//! 2. **Reading is tolerant, writing is not.** Every field a record
//!    written by an older build may lack carries `#[serde(default)]`,
//!    because the fixtures hold a `run_started` whose payload is `{}`.
//!    Unknown keys are ignored rather than refused, so a newer writer
//!    never breaks an older reader.
//! 3. **Values keep their kernel types.** A run is a [`RunId`](super::RunId) and not
//!    a `String`, so the display form and the parse live where they
//!    already lived. Types with a hand-written `Serialize` carry
//!    `schemars(with = "String")`, which states on the schema what
//!    their wire form has always been.
//!
//! A kind whose struct is not here yet is still read by hand at its
//! call sites; the migration moves one family at a time, and each
//! family's closing condition is that a replay produces identical
//! bytes.

mod checkpoint;
mod log;
mod run;
mod tool;
mod turn;

pub use checkpoint::{CheckpointCommitted, Commit, CommitAttribution};
pub use log::LogTruncated;
pub use run::{EvidenceCite, RunForked, RunFrozen, RunStarted, SkillPin};
pub use tool::{ToolAnswer, ToolCalled, ToolResult};
pub use turn::{
    ModelCalled, ModelReturned, PromptAssembled, PromptSegment, PromptSkip, PromptSource,
    SkipReason, SteerReceived,
};
