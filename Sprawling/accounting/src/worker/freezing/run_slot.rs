// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::path::Path;

use kernel::{Address, AxError, RunId};

use runtime::run::RunPlan;

use super::NEWLINE;

pub(super) struct Predecessor<'a> {
    pub(super) room: &'a Address,
    pub(super) run: Option<RunId>,
}

pub(super) fn run_segment(
    city_root: &Path,
    brief: &city::RunBrief,
    before: Predecessor<'_>,
) -> Result<Vec<u8>, AxError> {
    let mut out = Vec::new();
    if let Some(handoff) = city::handoff(city_root, before.room)? {
        out.extend_from_slice(handoff.as_bytes());
        out.push(NEWLINE);
        out.push(NEWLINE);
    }
    if let Some(run) = before.run {
        let transcript = runtime::Transcript::address(before.room, run)?;
        out.extend_from_slice(
            format!("Predecessor transcript: {}\n\n", transcript.as_str()).as_bytes(),
        );
    }
    out.extend_from_slice(brief.segment_text().as_bytes());
    Ok(out)
}

pub(super) fn task_line(plan: &RunPlan) -> String {
    format!("{} at {}", plan.task, plan.addr.as_str())
}
