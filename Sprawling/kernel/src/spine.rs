// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

mod grammar;
mod memo;
mod rewrite;
mod row;

pub use grammar::check_roadmap_shape;
pub use memo::{ScopeChange, WriteMoment};
pub use rewrite::{append_top_level, insert_children, set_roadmap_status};
pub use row::{
    EvidenceCell, NewChild, ROADMAP_COLUMNS, ROADMAP_FILE, ROADMAP_STATUS_SPELLINGS, RoadmapRow,
    RoadmapShape, RoadmapStatus,
};
