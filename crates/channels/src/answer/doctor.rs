// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! What this machine has, as the city found it when it started.
//!
//! Every state here is an enum rather than a sentence. The terminal
//! report is one machine's prose and a browser is two languages, so a
//! wire that carried the wording would make the page's words the
//! server's to choose. The one exception is `enables`, which is the
//! requirement table's own clause about what an item is for: no reader
//! can derive it, and nothing else on this answer can carry it.

use serde::{Deserialize, Serialize};

/// This machine, item by item, with a verdict for each tier.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct DoctorAnswer {
    /// Every item this city needs, in the order the requirement table
    /// states them.
    pub items: Vec<DoctorItem>,
    /// One verdict per tier: a person who only wants to run a city is
    /// not told about what changing this code would need.
    pub tiers: Vec<DoctorVerdict>,
}

/// Who needs an item, and therefore which verdict it counts towards.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum DoctorTier {
    /// Enough to run a city on this machine.
    Use,
    /// Enough to change this code and close its checks.
    Develop,
}

/// Whether a tier can be reached without the item.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum DoctorNeed {
    Required,
    /// Absent is a fact rather than a fault; `enables` says what having
    /// it would add.
    Optional,
}

/// One item, and this machine's answer about it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct DoctorItem {
    pub name: String,
    pub tier: DoctorTier,
    pub need: DoctorNeed,
    /// What having it lets a person do, in one clause, in the
    /// requirement table's own words.
    pub enables: String,
    pub state: DoctorState,
    pub install: DoctorInstall,
}

/// Whether the item is here, and in what condition.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum DoctorState {
    /// Here, and it started when asked its version.
    Present {
        at: String,
        version: DoctorVersion,
    },
    /// Here, and unusable. Counts as missing for a verdict, and as its
    /// own fault for a person.
    Broken {
        at: String,
        fault: DoctorFault,
    },
    Absent {
        absence: DoctorAbsence,
    },
}

/// What a present program said when asked its version. Only the first
/// arm carries text; the other three are facts about how it did not
/// answer, and none of them makes the program absent.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum DoctorVersion {
    Said {
        text: String,
    },
    /// It started, wrote nothing, and exited.
    Silent,
    /// Its first line was not text.
    Unreadable,
    /// It had not written a line when the deadline passed.
    Late,
}

/// Why a thing that is here cannot be used.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum DoctorFault {
    /// The operating system refused to start it.
    WillNotStart {
        said: String,
    },
    /// The component directory exists and the component file does not:
    /// an install that stopped half way.
    HalfWritten,
    Unreadable {
        said: String,
    },
}

/// Which kind of not being here.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum DoctorAbsence {
    NotOnSearchPath,
    /// A person pointed at a file, and the file is not there.
    VariableNamesNothing {
        variable: String,
        path: String,
    },
    NoComponent {
        dir: String,
    },
    NoHome,
    NotInThisBuild,
}

/// What getting the item would cost on this machine.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum DoctorInstall {
    /// A command this city may run once the person has agreed to it.
    Command { spelled: String },
    /// A command printed and never run: a script piped into a shell is
    /// code nobody read.
    Print { spelled: String },
    /// Nothing here can install it; the text says what a person does.
    Manual { how: String },
    /// This machine is not one of the three this project names recipes
    /// for, so nothing can be said about installing anything on it.
    UnknownPlatform,
}

/// Whether one tier is reachable on this machine.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct DoctorVerdict {
    pub tier: DoctorTier,
    /// The required items of this tier that are absent or broken, in
    /// table order. Empty means ready.
    pub missing: Vec<String>,
}
