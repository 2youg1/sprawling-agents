// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Plan claims decided on the accounting thread at the moment a model
//! makes them.
//!
//! Each run's plan desk holds the file as it stood when the run was
//! dispatched, and runs dispatched beside each other read the same file.
//! The accounting thread sees every claim in the order the lanes send
//! them, so the first run to ask for a node takes it and the second is
//! refused before it spends a call on the node (sprawling-SPEC.md 8-42-8).
//! A booking lasts until its run comes home and lands, which is when the
//! file on disk starts to say who held the node.

use std::collections::BTreeMap;
use std::sync::mpsc;

use kernel::{Address, AxCode, AxError, NodeId, RunId};

use super::relay::Wake;

/// One claim, and the address its answer goes back to.
pub(crate) struct ClaimAsk {
    building: Address,
    node: NodeId,
    run: RunId,
    back: mpsc::SyncSender<Result<(), AxError>>,
}

/// Which run holds which node of which building's plan while those runs
/// are in flight.
#[derive(Default)]
pub(crate) struct ClaimBook {
    held: BTreeMap<(Address, NodeId), RunId>,
}

impl ClaimBook {
    /// Books the node for the asking run, or refuses because another run
    /// in flight holds it, and sends the answer back to the lane.
    pub(crate) fn answer(&mut self, ask: ClaimAsk) {
        let ClaimAsk {
            building,
            node,
            run,
            back,
        } = ask;
        let holder = *self.held.entry((building, node.clone())).or_insert(run);
        let answer = if holder == run {
            Ok(())
        } else {
            Err(AxError::failure(
                AxCode::InvalidArgs,
                "claim a plan node",
                format!("{node} was claimed by {holder}, which is still working on it"),
            )
            .with_recovery("list the plan and claim a node that is ready"))
        };
        // A lane that stopped listening keeps its booking until it comes
        // home, which is what a lane that heard the answer would do.
        drop(back.send(answer));
    }

    /// Lets go of every node the run held, once it has come home.
    pub(crate) fn release(&mut self, run: RunId) {
        self.held.retain(|_, holder| *holder != run);
    }
}

/// The booking a run's plan desk asks through: a claim carried on the
/// accounting thread's one queue and waited for, like a relay append.
pub(crate) fn booking(bell: mpsc::Sender<Wake>, building: Address, run: RunId) -> collab::Booking {
    collab::Booking::new(move |node: &NodeId| {
        let (back, answer) = mpsc::sync_channel(0);
        bell.send(Wake::Claim(ClaimAsk {
            building: building.clone(),
            node: node.clone(),
            run,
            back,
        }))
        .map_err(|_| gone("the accounting thread is no longer taking claims"))?;
        answer
            .recv()
            .map_err(|_| gone("the accounting thread ended before answering"))?
    })
}

fn gone(why: &str) -> AxError {
    AxError::failure(AxCode::StorageFatal, "claim a plan node", why)
        .with_recovery("the city is closing; resume it and claim the node again")
}

#[cfg(test)]
#[allow(clippy::unwrap_used, reason = "test code")]
mod tests {
    use std::sync::mpsc;

    use super::{ClaimAsk, ClaimBook};
    use kernel::{Address, NodeId, RunId};

    fn ask(book: &mut ClaimBook, run: u8) -> bool {
        let (back, answer) = mpsc::sync_channel(1);
        book.answer(ClaimAsk {
            building: Address::parse("lab").unwrap(),
            node: NodeId::parse("1").unwrap(),
            run: RunId::from_bytes([run; 16]),
            back,
        });
        answer.recv().unwrap().is_ok()
    }

    /// Two lanes that read one plan ask for one node; the accounting
    /// thread gives it to the first and refuses the second until the
    /// first has come home.
    #[test]
    fn the_second_run_to_ask_for_a_node_is_refused_until_the_first_lands() {
        let mut book = ClaimBook::default();
        let first = ask(&mut book, 1);
        let second = ask(&mut book, 2);
        book.release(RunId::from_bytes([1; 16]));
        let after_landing = ask(&mut book, 2);
        assert_eq!((first, second, after_landing), (true, false, true));
    }

    struct Unwritten;

    impl kernel::Ledger for Unwritten {
        fn append(&mut self, _: kernel::EventDraft) -> Result<kernel::EventRef, kernel::AxError> {
            Err(super::gone("a claim writes nothing to the ledger"))
        }
    }

    fn plan_tool(bell: mpsc::Sender<super::Wake>, run: u8) -> collab::ClaimTool {
        let desk = collab::ClaimDesk::new(
            format!("potter@lab.{run}"),
            Address::parse("lab/room1").unwrap(),
            "# Roadmap

| # | Item | Weight | Needs | Status | Evidence |
             |---|------|--------|-------|--------|----------|
             | 1 | wire the kiln | 1 |  | Not started |  |
"
            .to_owned(),
            super::booking(
                bell,
                Address::parse("lab").unwrap(),
                RunId::from_bytes([run; 16]),
            ),
        );
        collab::ClaimTool::new(std::sync::Arc::new(std::sync::Mutex::new(desk))).unwrap()
    }

    /// Two runs in flight read node 1 as ready from one snapshot. Their
    /// desks ask through the production booking, the accounting thread
    /// serves the queue, and the second claim is refused at the call with
    /// the holder named.
    #[test]
    fn two_runs_claiming_one_node_through_the_served_gate_leave_one_holder() {
        use kernel::Tool;
        let mut gate = crate::serving::relay::RelayGate::open();
        let (mut first, mut second) = (plan_tool(gate.bell(), 1), plan_tool(gate.bell(), 2));
        let call = kernel::ToolCall {
            id: "tu_1".to_owned(),
            name: kernel::ToolName::parse("plan").unwrap(),
            args: kernel::Payload::new(
                serde_json::json!({ "action": "claim", "node": "1" })
                    .as_object()
                    .unwrap()
                    .clone(),
            )
            .unwrap(),
        };
        let lanes = std::thread::spawn(move || {
            let taken = first.invoke(&call).is_ok();
            (taken, second.invoke(&call).map_err(|e| format!("{e:?}")))
        });
        let mut homes = std::collections::VecDeque::new();
        while !lanes.is_finished() {
            gate.serve(
                crate::serving::relay::Patience::For(std::time::Duration::from_millis(5)),
                &mut Unwritten,
                &mut homes,
            );
        }
        let (taken, refused) = lanes.join().unwrap();
        let holder = RunId::from_bytes([1; 16]).to_string();
        assert_eq!(
            (taken, refused.map_err(|e| e.contains(&holder))),
            (true, Err(true)),
            "the first run takes node 1 and the second is refused naming the first"
        );
    }
}
