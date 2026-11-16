// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use kernel::{AxError, Increment, Model, ModelRequest, ModelReturn, ToolCall, ToolOutcome};

use super::wave::{ConcurrentInvoke, lost_answer, reads_only};

pub enum Generating<'a, 'sink> {

    Unwatched,

    Watched(&'a mut (dyn FnMut(&Increment) + 'sink)),

    Speculating {
        deltas: Option<&'a mut (dyn FnMut(&Increment) + 'sink)>,
        tools: &'a dyn ConcurrentInvoke,
    },
}

#[derive(Debug, Default)]
pub(super) struct Speculated {
    held: Vec<(ToolCall, Result<ToolOutcome, AxError>)>,
}

impl Speculated {

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

#[derive(Clone, Copy)]
enum Prefix {
    Reading,
    Written,
}

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

            let call = tools.resolve_call(call.clone());
            if let Prefix::Reading = prefix
                && reads_only(tools, &call)
                && let Some(tool) = tools.ahead(&call)
            {
                let handed = call.clone();
                started.push((call, scope.spawn(move || tool.invoke(&handed))));
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
