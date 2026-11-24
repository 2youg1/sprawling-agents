// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use kernel::{AxError, Locator};
use runtime::handoff::Handoff;
use runtime::run::RunPlan;

use super::Freezing;
use super::run_slot::task_line;

const NOT_RECORDED: &str = "not recorded: the room's Handoff.md does not say";

impl Freezing<'_> {
    pub(super) fn frozen_handoff(
        &mut self,
        plan: &RunPlan,
        mut must_read: Vec<Locator>,
        read_room: bool,
    ) -> Result<Handoff, AxError> {
        let transcript = runtime::Transcript::address(&plan.addr, plan.run)?;
        let dispatched = format!(
            "dispatched from the control surface; transcript at {}",
            transcript.as_str()
        );
        let text = if read_room {
            city::handoff(self.city_root, &plan.addr)?
        } else {
            None
        };
        let Some(text) = text else {
            return Handoff::new(
                must_read,
                task_line(plan),
                NOT_RECORDED.to_owned(),
                dispatched,
                NOT_RECORDED.to_owned(),
            );
        };
        let pinned = self
            .cas
            .put(text.as_bytes())
            .map_err(storage::StorageError::into_ax)?;
        must_read.push(Locator::cas(pinned));
        let sections = city::handoff_sections(&text);
        Handoff::new(
            must_read,
            sections.overall.unwrap_or_else(|| task_line(plan)),
            sections.progress.unwrap_or_else(|| NOT_RECORDED.to_owned()),
            match sections.context {
                Some(context) => format!("{dispatched}\n\n{context}"),
                None => dispatched,
            },
            sections
                .next_step
                .unwrap_or_else(|| NOT_RECORDED.to_owned()),
        )
    }
}
