// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The spine documents' grammar: how a row of `Roadmap.md` is written
//! and read, and the six fields a Memo outline carries.
//!
//! Shape and truth are two doors. `check_*` answers "is this written
//! like a roadmap"; what the rows then *mean* — how they hang together,
//! what each is worth, what may be started — is `crate::plan`'s, and it
//! is separate because a mistyped row is repaired by editing the line
//! while a circular dependency is repaired by rethinking the work.
//!
//! **Six columns, and the index is a path.** `2.3.1` hangs under `2.3`,
//! so the table states a tree without a second file to say so; `Weight`
//! is a ratio among the rows that share a parent, never a quantity; and
//! `Needs` names the rows that must finish first. The three columns the
//! old four-column table lacked are what let one plan carry multi-level
//! progress, a dependency graph and a ready set instead of pointing at a
//! diagram file nobody parses.
//!
//! **Every write goes through this file.** Assembling a row of Markdown
//! anywhere else would be a second opinion on what a row looks like, and
//! a grammar may only have one.

mod grammar;
mod memo;
mod rewrite;
mod row;

pub use grammar::check_roadmap_shape;
pub use memo::{MEMO_OUTLINE_FIELDS, MemoShape, ScopeChange, WriteMoment, check_memo_shape};
pub use rewrite::{insert_children, set_roadmap_status};
pub use row::{
    EvidenceCell, NewChild, ROADMAP_COLUMNS, ROADMAP_STATUS_SPELLINGS, RoadmapRow, RoadmapShape,
    RoadmapStatus,
};
