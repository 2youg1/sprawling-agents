// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The values a Command carries beside its scalars, and the closed sets: which scope a halt applies to, which governed document is being written,
//! what is being done to a pursuit, and what a new session keeps from
//! the one before it.
//!
//! Each of them is the protocol's own vocabulary with no upstream
//! owner, which is what separates them from the carried names in
//! `wire::carried_name`: those defer to whoever owns the value set,
//! and these have no one to defer to.

use kernel::event::record::SliceVerdict;
use kernel::{Address, B3Hash, Effort, IdemKey, KeepWarm};
use serde::{Deserialize, Serialize};

/// A page's save of one document: edits made on the version `baseline`,
/// refused once that version has moved (`crates/wire/spec/Command/Step.lean` §8-72).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct RangeWrite {
    pub doc: Address,
    pub baseline: B3Hash,
    pub edits: Vec<documents::TextEdit>,
    pub idem: IdemKey,
}

/// A display name for the session of `room` that began at `began`; an
/// empty name takes it back (`crates/wire/spec/Answer/Sessions.lean` D27).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct SessionNaming {
    pub room: Address,
    pub began: kernel::Seq,
    pub name: String,
    pub idem: IdemKey,
}

/// A room's new run policy, which the run under way takes at its next
/// safe point (`crates/wire/spec/Answer/Sessions.lean` D27).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct PolicyChange {
    pub room: Address,
    pub policy: kernel::model::RunPolicy,
    pub idem: IdemKey,
}

/// What `OpenRemoteDoor` asks for: the remote door open for `lasting_ms`, one minute to seven
/// days. The request does nothing by itself; the city prints a code at its own console and
/// refuses the frame with `E_APPROVAL_PENDING` (`crates/wire/spec/Command/Kind.lean`,
/// remote_access D4).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct DoorOpening {
    pub lasting_ms: u64,
    pub idem: IdemKey,
}

/// What `ConfirmRemoteDoor` carries: the code the city's console printed, read however it was
/// retyped. Every answer, right or wrong, ends the request it answers.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct DoorAnswer {
    pub code: String,
    pub idem: IdemKey,
}

/// A door verb with nothing to say but its key: `ReplaceCityKey`, guarded the way opening is,
/// and `CloseRemoteDoor`, unguarded because closing only takes access away (remote_access D5).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct DoorStep {
    pub idem: IdemKey,
}

/// A person's decision on proposal cards of one document, landed as one
/// save (`crates/wire/spec/Answer/Proposals.lean` §8-73).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct ProposalDecisions {
    pub doc: Address,
    pub decisions: Vec<ProposalDecision>,
    pub idem: IdemKey,
}

/// One card and the verdicts on its changed sentences; a sentence left
/// unnamed is rejected.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct ProposalDecision {
    pub proposal: B3Hash,
    pub verdicts: Vec<SliceVerdict>,
}

/// A building's whole `RULES.toml` from a page, and the text the page
/// read: the city evaluates `body` before it lands, and only over `base`
/// (`crates/wire/spec/Command/Step.lean` §8-60).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct RulesWrite {
    pub building: Address,
    pub base: String,
    pub body: String,
    pub idem: IdemKey,
}

/// The city's own layer from the settings page: `None` leaves a key as
/// it is (`crates/wire/spec/Command/Step.lean` §8-61).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct CitySettings {
    pub keep_warm: Option<KeepWarm>,
    pub effort: Option<Effort>,
    /// The city's whole `[search]` table, written before the other two
    /// so a value the city refuses lands nothing.
    pub search: Option<kernel::config::SearchConfiguration>,
    pub idem: IdemKey,
}

/// What a new session at an address keeps from the previous one.
///
/// An enum rather than a flag: the two states are named actions with
/// different results on disk, and `carry: true` at a call site says
/// neither of them. `Nothing` is the first variant and the default,
/// because that is what a person means by starting a new session — a
/// new one, here, not a continuation (`crates/sprawling/Spec.lean` §8-82). The
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

/// One identity card's values (`crates/wire/spec/Answer/Identity.lean` §8-59).
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
