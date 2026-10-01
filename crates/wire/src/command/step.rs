// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The closed sets a Command carries: which scope a halt applies to, which governed document is being written,
//! what is being done to a pursuit, and what a new session keeps from
//! the one before it.
//!
//! Each of them is the protocol's own vocabulary with no upstream
//! owner, which is what separates them from the carried names in
//! `wire::carried_name`: those defer to whoever owns the value set,
//! and these have no one to defer to.

use kernel::Address;
use serde::{Deserialize, Serialize};

/// What a new session at an address keeps from the previous one.
///
/// An enum rather than a flag: the two states are named actions with
/// different results on disk, and `carry: true` at a call site says
/// neither of them. `Nothing` is the first variant and the default,
/// because that is what a person means by starting a new session — a
/// new one, here, not a continuation (`sprawling-SPEC.md` 8-82). The
/// handoff is the exception a person states.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum Carry {
    /// Forget the shape the last session froze, and carry no summary.
    #[default]
    Nothing,
    /// Forget the shape, keep the summary the last session wrote.
    ///
    /// The shape still goes, which is the point of the verb: a person
    /// who carries the summary changed the model, and a room that kept
    /// its frozen shape could not dispatch at all.
    Handoff,
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

/// One identity card's values (wire-SPEC.md 8-59).
///
/// Each key is the card's own: `None` removes it, which puts the default
/// name back. What a name may be is the city's answer (`city::Naming`),
/// so a value here is the text the person typed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum IdentityCard {
    /// `PREFERENCES.md`: what the city calls the person, the host the
    /// name was imported from, and, when `about` is `Some`, the body
    /// that says what every agent should know about them.
    Person {
        user_id: Option<String>,
        imported_from: Option<String>,
        about: Option<String>,
    },
    /// `MAYOR.md`: what the Mayor is called. The body under the area is
    /// left as it is.
    Mayor { name: Option<String> },
}

/// Which of a building's own spine documents a write carries.
///
/// Named rather than addressed, for the reason [`GovernedDocument`]
/// gives: where these files live is the city's answer and not the
/// sender's. Unlike the three documents that govern a city, these have
/// a second writer — a resident reaches `Roadmap.md` through `plan` and
/// the others through `edit` — so a write to one of them is a write that
/// can lose a race, and the caller says which text it started from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum SpineDocument {
    /// The plan and the only source of progress in a building.
    Roadmap,
    /// The notepad: whatever needs recording and has no other home.
    Memo,
    /// What the next session needs and cannot get from the files.
    Handoff,
    /// What this project is and the decisions it holds.
    Spec,
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
