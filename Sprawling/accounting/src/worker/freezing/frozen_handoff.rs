// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The handoff a run is frozen with: what its room's `Handoff.md` says,
//! with the file's own bytes pinned for the successor to read.

use kernel::{AxError, Locator};
use runtime::handoff::Handoff;
use runtime::run::RunPlan;

use super::Freezing;
use super::run_slot::task_line;

/// What a section says when the room's handoff did not write it. Plain
/// absence, because a sentence that points elsewhere reads as a finding
/// and sends the successor looking for something nobody wrote.
const NOT_RECORDED: &str = "not recorded: the room's Handoff.md does not say";

impl Freezing<'_> {
    /// The handoff for `plan`, built from its room's `Handoff.md` when
    /// that file was filled in, and appending the file's bytes, pinned in
    /// the store, to `must_read`.
    ///
    /// The transcript's address is a pure function of the room and the
    /// run, so the handoff names it before a turn is taken. It names the
    /// room's file and never the ledger: the ledger is one chain for the
    /// whole city, under a subtree `read` refuses.
    ///
    /// # Errors
    /// Propagates a handoff file that exists and will not read, a store
    /// that will not take its bytes, and a handoff the runtime refuses.
    pub(super) fn frozen_handoff(
        &mut self,
        plan: &RunPlan,
        mut must_read: Vec<Locator>,
    ) -> Result<Handoff, AxError> {
        let transcript = runtime::Transcript::address(&plan.addr, plan.run)?;
        let dispatched = format!(
            "dispatched from the control surface; transcript at {}",
            transcript.as_str()
        );
        let Some(text) = city::handoff(self.city_root, &plan.addr)? else {
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
