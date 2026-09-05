// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use crate::approval::{ApprovalClass, ApprovalItem, ApprovalSource, ClusterKey};
use crate::locator::Locator;

use super::GateContext;

pub(crate) fn item(
    ctx: &GateContext,
    class: ApprovalClass,
    detail: String,
    action_desc: String,
    artifact: Locator,
    tainted: bool,
) -> ApprovalItem {
    ApprovalItem {
        id: ctx.item_id.clone(),
        source: ApprovalSource::Gate,
        actor: ctx.actor.clone(),
        action_desc,
        artifact,
        cluster_key: ClusterKey { class, detail },
        created: ctx.now,
        tainted,
    }
}
