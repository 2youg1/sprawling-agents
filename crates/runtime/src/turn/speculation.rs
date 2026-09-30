// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Read-only tool calls started while the model is still generating,
//! and the cache that holds what they returned until the answer settles.
//!
//! `tools/adversary/design/Speculating.lean` is the authority on what may start
//! early and in which order the results reach the Ledger: only the reads
//! before the answer's first writing call, cached by position, recorded
//! in emission order, and a failed answer's cache discarded whole.

use kernel::{AxError, Effect, Increment, Model, ModelRequest, ModelReturn, ToolCall, ToolOutcome};

use super::wave::{ConcurrentInvoke, lost_answer};

/// Who acts on a model's answer while it is still being generated. The
/// one place that decides which door of the model port a turn takes.
pub enum Generating<'a, 'sink> {
    /// Nobody: the blocking door.
    Unwatched,
    /// A page reads the text as it arrives: the streaming door.
    Watched(&'a mut (dyn FnMut(&Increment) + 'sink)),
    /// The tool face also starts each read-only call the model hands
    /// over before its first writing call: the speculating door.
    Speculating {
        deltas: Option<&'a mut (dyn FnMut(&Increment) + 'sink)>,
        tools: &'a dyn ConcurrentInvoke,
    },
}

/// What the reads started early returned, by their position in the
/// answer. Each result is held beside the call that was started, so it
/// answers only the settled call equal to it: the model port owes that
/// equality, and holding the call keeps a port that broke it from
/// answering one call with another's result.
#[derive(Debug, Default)]
pub(super) struct Speculated {
    held: Vec<(ToolCall, Result<ToolOutcome, AxError>)>,
}

impl Speculated {
    /// The cached answer for each of `calls`, in order: present where the
    /// call at that position was started early and is the one that
    /// settled there.
    pub(super) fn answers_for(
        self,
        calls: &[&ToolCall],
    ) -> Vec<Option<Result<ToolOutcome, AxError>>> {
        let mut held = self.held.into_iter();
        calls
            .iter()
            .map(|call| {
                held.next()
                    .and_then(|(started, answered)| (started == **call).then_some(answered))
            })
            .collect()
    }
}

/// Whether the calls handed over so far are all reads.
#[derive(Clone, Copy)]
enum Prefix {
    Reading,
    Written,
}

/// Makes the model call through the speculating door, starting each
/// read-only call handed over before the first writing call on a scoped
/// thread of its own. Every thread is joined before this returns, so the
/// turn waits for the longer of the reads and the generation rather than
/// their sum. The scope is this module's exception to the one spawn point
/// (ARCHITECTURE §10 rule 3), as `wave`'s is.
///
/// # Errors
/// Whatever the model call fails with; the reads it started are joined
/// and their results dropped with it.
pub(super) fn call_ahead(
    model: &mut dyn Model,
    request: &ModelRequest,
    onto: &mut dyn FnMut(&Increment),
    tools: &dyn ConcurrentInvoke,
) -> Result<(ModelReturn, Speculated), AxError> {
    std::thread::scope(|scope| {
        let mut started = Vec::new();
        let mut prefix = Prefix::Reading;
        let returned = model.call_speculating(request, onto, &mut |call: &ToolCall| {
            if let Prefix::Reading = prefix
                && tools.effect_of(call) == Some(Effect::Read)
                && let Some(tool) = tools.ahead(call)
            {
                let handed = call.clone();
                started.push((call.clone(), scope.spawn(move || tool.invoke(&handed))));
            } else {
                prefix = Prefix::Written;
            }
        });
        let held = started
            .into_iter()
            .map(|(call, worker)| (call, worker.join().unwrap_or_else(|_| Err(lost_answer()))))
            .collect();
        returned.map(|value| (value, Speculated { held }))
    })
}
