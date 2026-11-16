// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! What an official harness reported during a run and what it answered
//! the city, in this city's own words.
//!
//! The words are the city's and not ACP's (kernel D9): a spelling
//! the Ledger wrote cannot change, and ACP's words follow upstream. The
//! assembly maps each ACP reading onto these types with an exhaustive
//! `match`, so a report upstream adds stops the build at the mapping.
//!
//! Neither payload carries `#[serde(default)]`: both kinds were written
//! in this shape from their first line, so a line that lacks a key is
//! refused rather than guessed at.

use serde::{Deserialize, Serialize};

/// `harness_reported`: one thing the harness reported during a run.
///
/// The `report` key says which. A fold that answers "what did the city
/// do" never reads this kind; only a view that answers "what did the
/// harness say" does.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(tag = "report", rename_all = "snake_case")]
pub enum HarnessReported {
    /// A piece of the answer to the city.
    Said {
        /// The piece, as the harness sent it.
        text: String,
    },
    /// A piece of the harness's reasoning.
    Thought {
        /// The piece, as the harness sent it.
        text: String,
    },
    /// A tool call the harness started, in its own words.
    ToolCall {
        /// The harness's id for the call.
        call: String,
        /// The title the harness gave the call.
        title: String,
        /// The harness's own word for what kind of call it is.
        kind: String,
    },
    /// The status of a tool call the harness started changed.
    ToolCallStatus {
        /// The harness's id for the call.
        call: String,
        /// The harness's own word for the new status.
        status: String,
    },
    /// A report this city has no reading for; only its name is kept.
    Other {
        /// The name of the report as the harness spelled it.
        variant: String,
    },
    /// The harness asked the city for a permission.
    PermissionAsked {
        /// What the harness asked to do.
        title: String,
        /// The options it offered, in the order it offered them.
        options: Vec<HarnessPermit>,
    },
    /// The city answered a permission the harness asked for.
    PermissionAnswered {
        /// The id of the option the city chose; `None` is the answer
        /// `cancelled`.
        chosen: Option<String>,
    },
}

/// One option a harness offered when it asked for a permission.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct HarnessPermit {
    /// The harness's id for the option; the city answers with it.
    pub id: String,
    /// The name the harness shows for the option.
    pub name: String,
    /// What choosing the option grants or refuses.
    pub kind: HarnessPermitKind,
}

/// What choosing a permission option grants or refuses.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum HarnessPermitKind {
    /// Allowed this once.
    AllowOnce,
    /// Allowed from now on.
    AllowAlways,
    /// Refused this once.
    RejectOnce,
    /// Refused from now on.
    RejectAlways,
}

/// `harness_answered`: the harness's answer to the city's prompt.
///
/// Written when the stop reason arrives, before the run freezes, for
/// every stop reason. An `end_turn` answer with text is evidence of
/// `Completion::Done` (`crates/kernel/spec/Completion.lean` §8-20).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct HarnessAnswered {
    /// Why the harness stopped.
    pub stop: HarnessStop,
    /// The answer pieces of this turn, joined in order; written even
    /// when empty.
    pub text: String,
}

/// Why a harness stopped answering the city's prompt.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum HarnessStop {
    /// The harness finished its turn.
    EndTurn,
    /// The model ran out of output tokens.
    MaxTokens,
    /// The harness reached its limit of model requests for one turn.
    MaxTurnRequests,
    /// The model refused.
    Refusal,
    /// The city cancelled the turn.
    Cancelled,
}

#[cfg(test)]
#[allow(clippy::unwrap_used, reason = "test code")]
mod tests;
