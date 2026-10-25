// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The prompt documents the next run at an address is frozen with, in
//! the order they are sent, and the layer each one comes from
//! (`crates/wire/spec/Answer/Prefix.lean` §8-98).
//!
//! Which document is sent and which a nearer one replaces is decided
//! where the prefix is frozen; this answer carries that decision so a
//! page does not work it out a second time.

use kernel::{Address, B3Hash};
use serde::{Deserialize, Serialize};

use super::prefix::PrefixSlot;

/// Every prompt document of the next run at `addr`, in send order.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct LayersAnswer {
    pub addr: Address,
    pub documents: Vec<LayerDocument>,
}

/// One prompt document: where it is, or would be when it is absent;
/// which slot of the prefix it goes into; which layer it belongs to;
/// and whether the next run receives it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct LayerDocument {
    pub at: Address,
    pub slot: PrefixSlot,
    pub kind: PromptKind,
    pub layer: PromptLayer,
    pub state: LayerState,
    /// Present for the three documents whose default text this build
    /// carries.
    pub shipped: Option<ShippedDefault>,
}

/// What a prompt document is.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum PromptKind {
    /// How the city works: `City.md`.
    City,
    /// A project's own conventions: an `AGENTS.md`.
    Conventions,
    /// Who the resident at this address is.
    Identity,
    /// The note a person keeps for one model.
    ModelNote { provider: String, model: String },
}

/// Which layer a prompt document belongs to; a nearer layer replaces a
/// farther one where both answer the same question.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum PromptLayer {
    City,
    Building,
    Room,
}

/// Whether the next run receives a document.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum LayerState {
    /// Sent, at this length.
    Sent { bytes: u64 },
    /// There, and replaced by the nearer document at `by`.
    Replaced { by: Address },
    /// Not there.
    Absent,
    /// There, and not sent: the read bound refused where it leads, or it
    /// is not a file.
    Withheld { why: String },
}

/// This build's default for one of the three documents it carries, and
/// what the copy in the city is measured against it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct ShippedDefault {
    pub version: B3Hash,
    pub standing: DefaultStanding,
}

/// A copy measured against this build's default (city `CopyStanding`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum DefaultStanding {
    Absent,
    Current,
    Earlier,
    Edited,
}
