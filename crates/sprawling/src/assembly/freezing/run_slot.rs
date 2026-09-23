// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The run slot of the prefix: what this run was asked to do, under
//! what the last one in this room left behind.

use std::path::Path;

use kernel::{Address, AxError, RunId};

use runtime::run::RunPlan;

use super::NEWLINE;

/// What a successor is told about the run it replaces: the room they
/// share, and the predecessor's id when there is one. The two travel
/// together because the transcript's address is made of both.
pub(super) struct Predecessor<'a> {
    pub(super) room: &'a Address,
    pub(super) run: Option<RunId>,
}

/// The run slot: what the last run in this room left behind, where its
/// conversation is, then what this one was asked for.
///
/// In that order, because the brief is what the agent acts on and the
/// last thing in a prompt is the thing that is read. A handoff that is
/// still its blank form contributes nothing and is left out. The
/// transcript line is one address, never the transcript itself: a
/// successor searches it for what it needs rather than rereading
/// everything its predecessor saw.
/// # Errors
/// Propagates a handoff that exists and cannot be read: a prefix that
/// left it out would tell the next session there was none.
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
