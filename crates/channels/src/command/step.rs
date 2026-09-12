// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The closed sets a Command carries: which step of a login, which
//! scope a halt applies to, which governed document is being written,
//! and what is being done to a pursuit.
//!
//! Each of them is the protocol's own vocabulary with no upstream
//! owner, which is what separates them from the carried names in
//! `channels::carried_name`: those defer to whoever owns the value set,
//! and these four have no one to defer to.

use kernel::Address;
use serde::{Deserialize, Serialize};

/// Which step of a subscription login a `Login` frame carries.
///
/// The authorization code arrives by hand: the provider shows it to the
/// person after they approve, and the person brings it back. That is
/// the flow the profile table describes, and it needs no listening port
/// of its own.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum LoginStep {
    /// Mint the authorization URL for a person to open.
    Begin,
    /// Redeem the code that person brought back.
    Code { code: String },
}

/// What a Halt, Release or Autonomy change applies to. Unlike modes and
/// providers, this set is the protocol's own and has no upstream owner.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum HaltScope {
    City,
    Building(Address),
    Workshop(Address),
}

/// Which of the three documents that govern a city a `PutDocument`
/// frame carries.
///
/// A closed set rather than a path, because where these files live is
/// the city's answer and not the sender's: all three sit in the city's
/// own reserved subtree, which no write domain reaches. A frame naming
/// its own path would be a way to write anywhere inside the one place a
/// resident may not edit.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum GovernedDocument {
    /// Who the Mayor is: `MAYOR.md`.
    Mayor,
    /// What the clerk answers by: `CLERK.md`.
    Clerk,
    /// How this person wants their city run: `PREFERENCES.md`. It
    /// belongs to no resident, which is why it sits beside the other two
    /// rather than at somebody's address.
    Preferences,
}

/// What a `Pursue` command does to a pursuit.
///
/// `Clear` and `Pause` are different actions and both exist: pausing
/// keeps the goal so it can be taken up again, and clearing throws it
/// away. Cancelling a *run* is a third thing again, and it has its own
/// command.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum PursuitStep {
    /// Declare one, replacing any goal this building already had.
    Set {
        goal: String,
    },
    Pause,
    Resume,
    Clear,
}
