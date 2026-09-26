// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::collections::BTreeSet;
use std::sync::{Arc, Mutex};

use kernel::{AxCode, AxError, Tool, ToolCall, ToolName};

use super::*;

const PLAN: &str = "\
# Roadmap

| # | Item | Weight | Needs | Status | Evidence |
|---|------|--------|-------|--------|----------|
| 1 | wire the kiln | 1 |  | Not started |  |
";

/// The authority both runs ask, standing in for the accounting thread:
/// one set of booked nodes, the first asker wins.
fn book(booked: &Arc<Mutex<BTreeSet<NodeId>>>) -> Booking {
    let booked = Arc::clone(booked);
    Booking::new(move |claim| {
        if booked.lock().unwrap().insert(claim.id().clone()) {
            Ok(())
        } else {
            Err(
                AxError::failure(AxCode::InvalidArgs, "claim a plan node", "held")
                    .with_recovery("list the plan"),
            )
        }
    })
}

fn tool(who: &str, booking: Booking) -> (ClaimTool, Arc<Mutex<ClaimDesk>>) {
    let desk = Arc::new(Mutex::new(ClaimDesk::new(
        who.to_owned(),
        Address::parse("lab/room1").unwrap(),
        PLAN.to_owned(),
        booking,
    )));
    (ClaimTool::new(Arc::clone(&desk)).unwrap(), desk)
}

fn claim_one() -> ToolCall {
    ToolCall {
        id: "tu_1".to_owned(),
        name: ToolName::parse("plan").unwrap(),
        args: Payload::new(
            serde_json::json!({ "action": "claim", "node": "1" })
                .as_object()
                .unwrap()
                .clone(),
        )
        .unwrap(),
    }
}

/// **P0-6.** Two runs dispatched beside each other read one plan, and
/// each desk's own copy says node 1 is ready. The second claim is refused
/// when the model makes it, so the second run holds nothing, queues no
/// effect and leaves the file alone, instead of doing the work and
/// finding out at landing.
#[test]
fn a_node_two_runs_read_as_ready_is_taken_by_the_first_to_ask() {
    let booked = Arc::new(Mutex::new(BTreeSet::new()));
    let (mut first, _) = tool("potter@lab.1", book(&booked));
    let (mut second, second_desk) = tool("glazer@lab.2", book(&booked));

    first.invoke(&claim_one()).unwrap();
    let refused = second.invoke(&claim_one());

    let mut desk = second_desk.lock().unwrap();
    let effects = desk.take_effects();
    assert_eq!(
        (refused.is_err(), desk.holding(), effects, desk.roadmap()),
        (true, None, Vec::new(), None),
        "the second claim is refused at the call, and the run that lost holds nothing"
    );
}
