// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Which shelf a written skill lands on.

use kernel::Address;
use serde::{Deserialize, Serialize};

/// Where a [`Command::PutShelved`](crate::Command) writes.
///
/// Two places and no third, for the reason
/// [`GovernedDocument`](crate::GovernedDocument) is a closed set: both
/// of them sit under the city's reserved subtree, which no write
/// domain reaches, so a frame that named its own path would be a way
/// to write anywhere inside the one place a resident may not edit. The
/// city turns a shelf and a name into a path; the sender never spells
/// one.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum Shelf {
    /// The city's own library, which any building may admit from.
    Library,
    /// One building's private shelf, which only that building keeps.
    Building(Address),
}
