// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The leading reads of a wave run at once, and their answers handed back
//! in call order.

use kernel::{AxCode, AxError, Tool, ToolCall, ToolOutcome};

use super::{Admitted, ConcurrentInvoke};

/// Runs the tool of every cleared call that holds no answer from while
/// the model was generating, all at once, and answers in call order, one
/// answer per such call: the reorder buffer. The first runs on this
/// thread, so a wave of one read starts no thread at all. The scope is
/// this module's exception to the one spawn point (ARCHITECTURE §10
/// rule 3): every thread it starts is joined before it returns.
pub(super) fn all_at_once(
    tools: &dyn ConcurrentInvoke,
    calls: &[&ToolCall],
    admitted: &[Admitted],
    early: &[Option<Result<ToolOutcome, AxError>>],
) -> Vec<Result<ToolOutcome, AxError>> {
    let running: Vec<(&ToolCall, Result<&dyn Tool, AxError>)> = calls
        .iter()
        .zip(admitted)
        .zip(early)
        .filter_map(|((call, admission), cached)| match (admission, cached) {
            (Admitted::Cleared(ticket), None) => Some((*call, tools.tool(ticket))),
            (Admitted::Cleared(_), Some(_)) | (Admitted::Answered(_), _) => None,
        })
        .collect();
    let run = |(call, tool): &(&ToolCall, Result<&dyn Tool, AxError>)| match tool {
        Ok(tool) => tool.invoke(call),
        Err(unheld) => Err(unheld.clone()),
    };
    let Some((first, rest)) = running.split_first() else {
        return Vec::new();
    };
    std::thread::scope(|scope| {
        let others: Vec<_> = rest
            .iter()
            .map(|job| scope.spawn(move || run(job)))
            .collect();
        std::iter::once(run(first))
            .chain(
                others
                    .into_iter()
                    .map(|worker| worker.join().unwrap_or_else(|_| Err(lost_answer()))),
            )
            .collect()
    })
}

pub(in crate::turn) fn lost_answer() -> AxError {
    AxError::failure(
        AxCode::ToolUnavailable,
        "run a read-only tool call",
        "the tool stopped its thread without an answer",
    )
    .with_recovery("call the tool again; report it when it stops a second time")
}
