// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::collections::BTreeMap;

use crate::node_id::NodeId;

mod blocking;
mod node;
mod share;
mod tree;

pub use node::{Held, PlanExit, PlanNode, StopCause};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlanTree {
    pub(crate) nodes: BTreeMap<NodeId, PlanNode>,
}
