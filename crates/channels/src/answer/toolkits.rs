// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Which outside applications the broker offers, and where each one
//! stands for this city.
//!
//! This is a reading of right now and it is never folded from the
//! Ledger, on the same grounds as `McpHealth` (channels-SPEC section
//! 8-31): whether an account is connected this minute is a fact about
//! this minute, and a recorded copy would still call it connected an
//! hour after the person revoked it.
//!
//! One shape answers both the question and the act. Asking what is on
//! the shelf and asking to connect one of its rows come back as the
//! same list, so a client draws one rendering rather than two that have
//! to agree.

use kernel::AxError;
use serde::{Deserialize, Serialize};

use crate::carried_name::ToolkitSlug;

/// The shelf, or the reason there is no shelf to show.
///
/// Exhaustive rather than a list that is empty in three different
/// senses: "you have not given this city a key yet", "the broker is
/// unreachable" and "the broker has nothing for you" are three
/// different things for a person to do next, and an empty `Vec` says
/// all three at once.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum ToolkitsAnswer {
    /// No project key has been enrolled, so there is nobody to ask. The
    /// page shows the key field, and this is not a failure: it is the
    /// first step, and drawing it as an error would tell a new person
    /// that something broke.
    Unenrolled,
    /// What the broker published, in the order it published it.
    ///
    /// The broker's order rather than a best-first one: somebody
    /// scanning this list twice finds the same application in the same
    /// place.
    Shelf { toolkits: Vec<ToolkitLine> },
    /// The broker was asked and did not answer. The whole refusal
    /// travels, so the page draws the city's wording rather than
    /// inventing a second one.
    Refused { refusal: Box<AxError> },
}

/// One application on the shelf.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct ToolkitLine {
    pub slug: ToolkitSlug,
    /// What the broker calls it in prose, which is what a person
    /// recognises it by.
    pub name: String,
    /// How this application authenticates, in the broker's own word -
    /// `oauth2`, `api_key`, `bearer_token`. Carried rather than reduced
    /// to a flag: the word decides what the person is about to be
    /// shown, and this city is not the authority on which schemes
    /// exist.
    pub auth: String,
    pub standing: Standing,
}

/// Where one application stands for this city.
///
/// Four states, each a different next action: connect it, finish the
/// consent page that is already open, nothing, or read why the last
/// attempt failed. Every one of them is the broker's own reading rather
/// than something this city remembered - the page a person left open
/// yesterday is not evidence that they finished with it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum Standing {
    /// Nothing here has connected it.
    Absent,
    /// A consent page exists and the person has not finished with it.
    ///
    /// The url travels so the client can open it, and keeps travelling
    /// so the client can offer it again: a blocked popup and a closed
    /// tab both leave a person who needs this link a second time, and a
    /// url held only at the moment the command answered would be gone.
    Awaiting { consent_url: String },
    /// Connected, under the account name the broker returned. The name
    /// is shown because one person can hold two accounts on the same
    /// application and "connected" alone would not say which.
    Connected { alias: String },
    /// The last attempt failed and the broker still says so.
    Refused { refusal: Box<AxError> },
}
