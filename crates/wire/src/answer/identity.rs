// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! What the city calls the person and the Mayor, as a page reads it
//! (wire-SPEC.md 8-59).

use kernel::B3Hash;
use serde::{Deserialize, Serialize};

use crate::command::GovernedDocument;

/// The two identity areas as they stand, or where one stopped reading.
///
/// An area that does not read is its own answer rather than a default
/// name: a page that drew the default would let its next save write over
/// the line the person got wrong.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum IdentityAnswer {
    Stated(StatedIdentity),
    Unreadable {
        document: GovernedDocument,
        /// The line of the document, counted from its first.
        line: u32,
        why: String,
    },
}

/// The names a new session would freeze, and the two texts a save names
/// as its `base`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct StatedIdentity {
    /// What the city calls the person; absent until somebody says, and
    /// the page then says "you" in its own language.
    pub user_id: Option<String>,
    /// The host whose `gh` the user id was imported from; absent for a
    /// name typed by hand.
    pub imported_from: Option<String>,
    /// What the person wants every agent to know: `PREFERENCES.md`
    /// below its identity area.
    pub about: String,
    /// What the Mayor is called; absent until somebody says, and the
    /// page then draws the name its language table ships.
    pub mayor: Option<String>,
    /// The version a new session freezes; a `run_started` line naming
    /// another version ran under other names.
    #[cfg_attr(feature = "schema", schemars(with = "String"))]
    pub version: B3Hash,
    pub preferences_text: String,
    pub mayor_text: String,
}
