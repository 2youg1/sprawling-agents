// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The moments the plan and the record are written.
//!
//! `Memo.md` itself carries no shape: it is the notepad for what needs
//! recording and has no other home, so the form of an entry is the
//! writer's to choose. What the city does fix is *when* the plan and the
//! memo are written, and that is this module's [`WriteMoment`].

use serde::{Deserialize, Serialize};

/// Scope-change vocabulary: requirements move by KEEP/ADD/DROP, never by
/// piling replacements into a bigger project.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScopeChange {
    Keep,
    Add,
    Drop,
}

/// The three moments the plan may be written.
///
/// Its consumer is the projection that holds the parsed tree: because
/// the set of moments is closed, a reader can hold the tree between them
/// instead of parsing every building's file for every question.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WriteMoment {
    BeforeReport,
    AfterFeedback,
    OnPlanChange,
}
