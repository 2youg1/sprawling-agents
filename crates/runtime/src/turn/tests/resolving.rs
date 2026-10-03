// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! A call through `call` accounted as the tool it names
//! (`crates/runtime/Spec.lean` §8-61).

use super::super::*;
use super::concurrent::{Placed, waved};
use super::helpers::*;
use crate::bench::Ticket;
use kernel::{Tool, ToolOutcome};

/// A face that resolves a call through `call` the way the lane's face
/// does, through a catalog that admitted the bench's `read`.
struct Resolving {
    placed: Placed,
    catalog: crate::Catalog,
}

impl Resolving {
    fn new() -> Resolving {
        let placed = Placed::new();
        let mut catalog = crate::Catalog::new();
        let read = kernel::ToolName::parse("read").unwrap();
        catalog
            .admit_tool(placed.bench.meta_of(&read).unwrap())
            .unwrap();
        Resolving { placed, catalog }
    }
}

impl ConcurrentInvoke for Resolving {
    fn meta_of(&self, call: &ToolCall) -> Option<&kernel::ToolMeta> {
        self.placed.meta_of(call)
    }

    fn resolve_call(&self, call: ToolCall) -> ToolCall {
        if call.name.as_str() != crate::tools::CallTool::NAME {
            return call;
        }
        self.catalog.resolve_call(&call).unwrap_or(call)
    }

    fn admit(&mut self, call: &ToolCall, t: TimeMs) -> Admitted {
        self.placed.admit(call, t)
    }

    fn tool(&self, ticket: &Ticket) -> Result<&dyn Tool, AxError> {
        self.placed.tool(ticket)
    }

    fn account(
        &mut self,
        call: &ToolCall,
        ticket: Ticket,
        answered: Result<ToolOutcome, AxError>,
    ) -> Result<ToolOutcome, AxError> {
        self.placed.account(call, ticket, answered)
    }
}

/// A call through `call` is accounted as the call it stands for: the
/// ledger names the tool that ran, with its registration, under the id
/// the model gave the `call`, so the result still pairs with it.
#[test]
fn a_call_through_call_is_accounted_as_the_tool_it_names() {
    let mut ledger = TestLedger::new();
    let through = ToolCall {
        id: "c1".to_owned(),
        name: kernel::ToolName::parse("call").unwrap(),
        args: Payload::of(&serde_json::json!({ "name": "read", "args": {} })).unwrap(),
    };
    let mut lines = lines();
    let turn = waved(opened_on::<1>(&mut lines), &mut ledger, vec![through]);
    let recording = advance(
        turn.execute_concurrent(
            Interrupt::None,
            &mut ledger,
            &mut Resolving::new(),
            &mut |_| Interrupt::None,
        )
        .unwrap(),
    );
    advance(recording.record(Interrupt::None, &mut ledger).unwrap());
    lines.barrier(&mut ledger).unwrap();
    let lines: Vec<serde_json::Value> = ledger
        .lines
        .iter()
        .map(|line| serde_json::from_slice::<serde_json::Value>(line).unwrap())
        .filter(|line| line["kind"] == "tool_called" || line["kind"] == "tool_result")
        .map(|line| {
            serde_json::json!({
                "kind": line["kind"],
                "name": line["data"]["name"],
                "id": line["data"]["id"].as_str().or(line["data"]["tool_use_id"].as_str()),
                "effect": line["data"]["effect"],
                "failed": line["data"].to_string().contains("\"error\""),
            })
        })
        .collect();
    assert_eq!(
        lines,
        vec![
            serde_json::json!({ "kind": "tool_called", "name": "read", "id": "c1",
                "effect": "read", "failed": false }),
            serde_json::json!({ "kind": "tool_result", "name": "read", "id": "c1",
                "effect": null, "failed": false }),
        ]
    );
}
