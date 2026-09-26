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
//! The claim's `roadmap_claimed` line is appended as the node is booked,
//! so the history says who holds a node from the moment the model took
//! it rather than from the moment its run lands. A booking lasts until
//! its run comes home and lands, which is when the file on disk starts
//! to say who held the node; the run's landing is handed the lines that
//! give its still-booked nodes back, and writes those its plan did not
//! close, so every claim on the history is closed.

use std::collections::BTreeMap;
use std::sync::mpsc;

use kernel::{Address, AxCode, AxError, EventDraft, Ledger, NodeId, RunId};

use super::relay::Wake;
use accounting::effect;

/// Who a run's claims are made for, and the clock their lines are
/// stamped by. The five travel together from the run's dispatch to
/// every claim it makes.
pub(crate) struct Claimant {
    /// The building whose plan the claimed nodes belong to.
    pub(crate) building: Address,
    /// The room the run works in, which the claim's line is filed under.
    pub(crate) room: Address,
    pub(crate) run: RunId,
    pub(crate) who: String,
    /// The worker's own clock (accounting-SPEC.md 8-3).
    pub(crate) clock: std::sync::Arc<dyn accounting::Clock + Send + Sync>,
}

/// One claim, its `roadmap_claimed` line, the line that would hand the
/// node back, and the address its answer goes back to.
pub(crate) struct ClaimAsk {
    building: Address,
    node: NodeId,
    line: EventDraft,
    put_back: effect::Line,
    back: mpsc::SyncSender<Result<(), AxError>>,
}

/// One node booked for a run in flight, and the line that gives it back
/// if the run comes home without landing its plan.
struct Booked {
    run: RunId,
    put_back: effect::Line,
}

/// Which run holds which node of which building's plan while those runs
/// are in flight.
#[derive(Default)]
pub(crate) struct ClaimBook {
    held: BTreeMap<(Address, NodeId), Booked>,
}

/// The lines that give back the nodes a run still had booked when it
/// came home. Its landing closes each node as that node's closing line
/// reaches the ledger; whatever it did not close is still owed to the
/// history.
#[must_use = "a claim written at call time stays open on the history until its put-back line is written"]
pub(crate) struct OpenClaims {
    run: RunId,
    put_backs: BTreeMap<NodeId, effect::Line>,
}

impl OpenClaims {
    /// A line that closes `node`'s claim is on the ledger, so the last
    /// line the history holds for it no longer reads it as held.
    pub(crate) fn close(&mut self, node: &NodeId) {
        self.put_backs.remove(node);
    }

    /// The run these claims belong to, and the lines still owed for them.
    pub(crate) fn owed(self) -> (RunId, Vec<effect::Line>) {
        (self.run, self.put_backs.into_values().collect())
    }
}

impl ClaimBook {
    /// Appends the claim's line and books the node for the asking run,
    /// or refuses because another run in flight holds it, and sends the
    /// answer back to the lane. A line the ledger refuses books nothing,
    /// so no run holds a node the history does not show it holding.
    ///
    /// Hands back the line when the ledger took it, for the folds the
    /// accounting thread shows every line it writes.
    pub(crate) fn answer(&mut self, ask: ClaimAsk, ledger: &mut impl Ledger) -> Option<EventDraft> {
        let ClaimAsk {
            building,
            node,
            line,
            put_back,
            back,
        } = ask;
        let run = line.run;
        let key = (building, node);
        let answer = match self.held.get(&key).map(|booked| booked.run) {
            Some(holder) if holder != run => Err(AxError::failure(
                AxCode::InvalidArgs,
                "claim a plan node",
                format!(
                    "{} was claimed by {holder}, which is still working on it",
                    key.1
                ),
            )
            .with_recovery("list the plan and claim a node that is ready")),
            Some(_) | None => ledger.append(line.clone()).map(|_| {
                self.held.insert(key, Booked { run, put_back });
            }),
        };
        let taken = answer.is_ok().then_some(line);
        // A lane that stopped listening keeps its booking until it comes
        // home, which is what a lane that heard the answer would do.
        drop(back.send(answer));
        taken
    }

    /// Lets go of every node the run held, once it has come home, and
    /// hands over the lines that give them back.
    pub(crate) fn release(&mut self, run: RunId) -> OpenClaims {
        let (theirs, others): (BTreeMap<_, _>, BTreeMap<_, _>) = std::mem::take(&mut self.held)
            .into_iter()
            .partition(|(_, booked)| booked.run == run);
        self.held = others;
        OpenClaims {
            run,
            put_backs: theirs
                .into_iter()
                .map(|((_, node), booked)| (node, booked.put_back))
                .collect(),
        }
    }
}

/// The booking a run's plan desk asks through: a claim carried on the
/// accounting thread's one queue and waited for, like a relay append.
/// The claim's instant is taken when the model makes it.
pub(crate) fn booking(bell: mpsc::Sender<Wake>, claimant: Claimant) -> collab::Booking {
    collab::Booking::new(move |claim: &collab::ClaimEffect| {
        let line = EventDraft {
            run: claimant.run,
            t: claimant.clock.now()?,
            who: claimant.who.clone(),
            addr: Some(claimant.room.clone()),
            kind: claim.kind(),
            data: claim.payload(&claimant.who)?,
            ig: false,
        };
        let put_back = effect::handed_back(
            claim,
            "this run came home without landing its plan",
            &claimant.room,
            &claimant.who,
        )?
        .ok_or_else(|| {
            AxError::failure(
                AxCode::InvalidArgs,
                "claim a plan node",
                format!(
                    "{} was asked for by an effect that is not a claim",
                    claim.id()
                ),
            )
            .with_recovery("report this against collab::claim_tool: its desk books claims alone")
        })?;
        let (back, answer) = mpsc::sync_channel(0);
        bell.send(Wake::Claim(ClaimAsk {
            building: claimant.building.clone(),
            node: claim.id().clone(),
            line,
            put_back,
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

    use super::{ClaimAsk, ClaimBook, Claimant};
    use kernel::{Address, AxError, EventDraft, EventKind, EventRef, NodeId, RunId};

    /// The ledger the accounting thread writes to, keeping every line.
    #[derive(Default)]
    struct Kept(Vec<EventDraft>);

    impl kernel::Ledger for Kept {
        fn append(&mut self, draft: EventDraft) -> Result<EventRef, AxError> {
            let seq = u64::try_from(self.0.len()).unwrap().saturating_add(1);
            self.0.push(draft.clone());
            Ok(
                kernel::EventRecord::from_draft(draft, kernel::Seq::new(seq), kernel::GENESIS_PREV)
                    .to_ref(),
            )
        }
    }

    fn claimant(run: u8) -> Claimant {
        Claimant {
            building: Address::parse("lab").unwrap(),
            room: Address::parse("lab/room1").unwrap(),
            run: RunId::from_bytes([run; 16]),
            who: format!("potter@lab.{run}"),
            clock: std::sync::Arc::new(crate::assembly::SystemClock),
        }
    }

    fn ask(book: &mut ClaimBook, run: u8) -> bool {
        let (back, answer) = mpsc::sync_channel(1);
        let claim = collab::ClaimEffect::Claimed {
            id: NodeId::parse("1").unwrap(),
            item: "wire the kiln".to_owned(),
        };
        book.answer(
            ClaimAsk {
                building: Address::parse("lab").unwrap(),
                node: claim.id().clone(),
                line: EventDraft {
                    run: RunId::from_bytes([run; 16]),
                    t: kernel::TimeMs::new(0),
                    who: "potter".to_owned(),
                    addr: None,
                    kind: claim.kind(),
                    data: claim.payload("potter").unwrap(),
                    ig: false,
                },
                put_back: accounting::effect::handed_back(
                    &claim,
                    "came home",
                    &Address::parse("lab/room1").unwrap(),
                    "potter",
                )
                .unwrap()
                .unwrap(),
                back,
            },
            &mut Kept::default(),
        );
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
        drop(book.release(RunId::from_bytes([1; 16])));
        let after_landing = ask(&mut book, 2);
        assert_eq!((first, second, after_landing), (true, false, true));
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
            super::booking(bell, claimant(run)),
        );
        collab::ClaimTool::new(std::sync::Arc::new(std::sync::Mutex::new(desk))).unwrap()
    }

    /// Two runs in flight read node 1 as ready from one snapshot. Their
    /// desks ask through the production booking, the accounting thread
    /// serves the queue, and the second claim is refused at the call with
    /// the holder named. The one claim that was taken is on the ledger
    /// before either run lands, and the refused one left no line.
    #[test]
    fn two_runs_claiming_one_node_through_the_served_gate_leave_one_holder() {
        use kernel::Tool;
        let mut gate = crate::assembly::relay::RelayGate::open();
        let (first, second) = (plan_tool(gate.bell(), 1), plan_tool(gate.bell(), 2));
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
        let (mut homes, mut ledger) = (std::collections::VecDeque::new(), Kept::default());
        while !lanes.is_finished() {
            gate.serve(
                crate::assembly::relay::Patience::For(std::time::Duration::from_millis(5)),
                &mut ledger,
                &mut homes,
            );
        }
        let (taken, refused) = lanes.join().unwrap();
        let holder = RunId::from_bytes([1; 16]).to_string();
        let claims: Vec<(RunId, Option<String>)> = ledger
            .0
            .iter()
            .filter(|line| line.kind == EventKind::RoadmapClaimed)
            .map(|line| (line.run, line.addr.as_ref().map(ToString::to_string)))
            .collect();
        assert_eq!(
            (taken, refused.map_err(|e| e.contains(&holder)), claims),
            (
                true,
                Err(true),
                vec![(RunId::from_bytes([1; 16]), Some("lab/room1".to_owned()))]
            ),
            "the first run takes node 1, its claim is on the ledger at the call, and the second is refused naming the first"
        );
    }
}
