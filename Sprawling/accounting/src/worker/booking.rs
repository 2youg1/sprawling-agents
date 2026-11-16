// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::collections::BTreeMap;
use std::sync::mpsc;

use kernel::event::record::RoadmapMoved;
use kernel::{Address, AxCode, AxError, EventDraft, EventKind, Ledger, NodeId, Payload, RunId};

use super::relay::Wake;
use crate::effect;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
pub(crate) struct PlanRead(u64);

pub(crate) struct Claimant {
    pub building: Address,

    pub room: Address,
    pub run: RunId,
    pub who: String,

    pub clock: std::sync::Arc<dyn crate::Clock + Send + Sync>,

    pub read: PlanRead,
}

pub(crate) struct ClaimAsk {
    building: Address,
    node: NodeId,
    line: EventDraft,
    put_back: effect::Line,
    read: PlanRead,
    back: mpsc::SyncSender<Result<(), AxError>>,
}

struct Booked {
    run: RunId,
    put_back: effect::Line,
}

#[derive(Default)]
pub(crate) struct ClaimBook {
    held: BTreeMap<(Address, NodeId), Booked>,

    stopped: BTreeMap<(Address, NodeId), PlanRead>,

    stops: PlanRead,
}

#[must_use = "a claim written at call time stays open on the history until its put-back line is written"]
pub(crate) struct OpenClaims {
    run: RunId,
    put_backs: BTreeMap<NodeId, effect::Line>,
}

impl OpenClaims {
    pub fn close(&mut self, node: &NodeId) {
        self.put_backs.remove(node);
    }

    pub fn owed(self) -> (RunId, Vec<effect::Line>) {
        (self.run, self.put_backs.into_values().collect())
    }
}

impl ClaimBook {
    pub fn answer(&mut self, ask: ClaimAsk, ledger: &mut impl Ledger) -> Option<EventDraft> {
        let ClaimAsk {
            building,
            node,
            line,
            put_back,
            read,
            back,
        } = ask;
        let run = line.run;
        let key = (building, node);
        let stopped_since_read = self.stopped.get(&key).is_some_and(|stop| *stop > read);
        let answer = match self.held.get(&key).map(|booked| booked.run) {
            _ if stopped_since_read => Err(AxError::failure(
                AxCode::InvalidArgs,
                "claim a plan node",
                format!(
                    "{} ended after this run read the plan; the history holds its end",
                    key.1
                ),
            )
            .with_recovery("list the plan and claim a node that is ready")),
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

        drop(back.send(answer));
        taken
    }

    pub fn read_mark(&self) -> PlanRead {
        self.stops
    }

    #[expect(
        clippy::wildcard_enum_match_arm,
        reason = "four kinds end or reopen a node; the rest of the event vocabulary does not"
    )]
    pub fn absorb(
        &mut self,
        kind: EventKind,
        addr: Option<&Address>,
        data: &Payload,
    ) -> Result<(), AxError> {
        let Some(building) = addr.and_then(super::building_of) else {
            return Ok(());
        };
        match kind {
            EventKind::RoadmapFinished | EventKind::RoadmapSplit | EventKind::RoadmapBlocked => {
                let node = data.read::<RoadmapMoved>()?.node;
                self.stops = PlanRead(self.stops.0.saturating_add(1));
                self.stopped.insert((building, node), self.stops);
            }
            EventKind::RoadmapReleased => {
                let node = data.read::<RoadmapMoved>()?.node;
                self.stopped.remove(&(building, node));
            }
            _ => {}
        }
        Ok(())
    }

    pub fn release(&mut self, run: RunId) -> OpenClaims {
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

pub(crate) fn booking(bell: mpsc::Sender<Wake>, claimant: Claimant) -> collab::Booking {
    collab::Booking::new(move |claim: &collab::ClaimEffect| {
        let not_a_claim = || {
            AxError::failure(
                AxCode::InvalidArgs,
                "claim a plan node",
                format!(
                    "{} was asked for by an effect that is not a claim",
                    claim.id()
                ),
            )
            .with_recovery("report this against collab::claim_tool: its desk books claims alone")
        };
        let (kind, data) = claim.line(&claimant.who)?.ok_or_else(not_a_claim)?;
        let line = EventDraft {
            run: claimant.run,
            t: claimant.clock.now()?,
            who: claimant.who.clone(),
            addr: Some(claimant.room.clone()),
            kind,
            data,
            ig: false,
        };
        let put_back = effect::handed_back(
            claim,
            "this run came home without landing its plan",
            &claimant.room,
            &claimant.who,
        )?
        .ok_or_else(not_a_claim)?;
        let (back, answer) = mpsc::sync_channel(0);
        bell.send(Wake::Claim(ClaimAsk {
            building: claimant.building.clone(),
            node: claim.id().clone(),
            line,
            put_back,
            read: claimant.read,
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
            clock: std::sync::Arc::new(crate::worker::fixture::WallClock),
            read: super::PlanRead::default(),
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
                    kind: claim.kind().unwrap(),
                    data: claim.line("potter").unwrap().unwrap().1,
                    ig: false,
                },
                put_back: crate::effect::handed_back(
                    &claim,
                    "came home",
                    &Address::parse("lab/room1").unwrap(),
                    "potter",
                )
                .unwrap()
                .unwrap(),
                read: super::PlanRead::default(),
                back,
            },
            &mut Kept::default(),
        );
        answer.recv().unwrap().is_ok()
    }

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
            "# Roadmap\n\n| # | Item | Weight | Needs | Status | Evidence |\n\
             |---|------|--------|-------|--------|----------|\n\
             | 1 | wire the kiln | 1 |  | Not started |  |\n"
                .to_owned(),
            super::booking(bell, claimant(run)),
        );
        collab::ClaimTool::new(std::sync::Arc::new(std::sync::Mutex::new(desk))).unwrap()
    }

    #[test]
    fn two_runs_claiming_one_node_through_the_served_gate_leave_one_holder() {
        use kernel::Tool;
        let mut gate = crate::worker::relay::RelayGate::open(crate::worker::fixture::monotonic);
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
                crate::worker::relay::Patience::For(std::time::Duration::from_millis(5)),
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
