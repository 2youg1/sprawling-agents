// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::path::{Path, PathBuf};

use kernel::{Address, AxError, EventKind, NodeId, Payload, TimeMs};

pub struct Line {
    pub who: String,
    pub addr: Address,
    pub kind: EventKind,
    pub data: Payload,
}

pub enum Then {
    Nothing,

    Deliver(Vec<collab::Signal>),

    Roadmap {
        path: PathBuf,
        base: String,
        text: String,
    },

    Shelf(Vec<Filing>),
}

pub struct Filing {
    pub entry: city::ArchiveEntry,
    pub body: String,
}

pub struct Landing<L = Line> {
    lines: Vec<L>,
    then: Then,
}

pub struct Closing {
    pub line: Line,
    pub closes: Option<NodeId>,
}

impl<L> Landing<L> {
    pub fn record(
        self,
        append: &mut impl FnMut(L) -> Result<(), AxError>,
    ) -> Result<Then, AxError> {
        for line in self.lines {
            append(line)?;
        }
        Ok(self.then)
    }
}

impl Landing {
    pub fn signals(
        effects: Vec<collab::SignalEffect>,
        room: &Address,
        who: &str,
    ) -> Result<Landing, AxError> {
        let mut lines = Vec::new();
        let mut deliver = Vec::new();
        for effect in effects {
            match effect {
                collab::SignalEffect::Enqueued(signal) => {
                    lines.push(Line {
                        who: who.to_owned(),
                        addr: signal.room().clone(),
                        kind: EventKind::SignalEnqueued,
                        data: signal.enqueued_payload()?,
                    });
                    deliver.push(signal);
                }
                collab::SignalEffect::Consumed { signal, by } => {
                    let data = signal.consumed_payload(&by)?;
                    lines.push(Line {
                        who: by,
                        addr: room.clone(),
                        kind: EventKind::SignalConsumed,
                        data,
                    });
                }
                collab::SignalEffect::WaitStarted(started) => lines.push(Line {
                    who: who.to_owned(),
                    addr: room.clone(),
                    kind: EventKind::SignalWaitStarted,
                    data: Payload::of(&started)?,
                }),
                collab::SignalEffect::WaitEnded(ended) => lines.push(Line {
                    who: who.to_owned(),
                    addr: room.clone(),
                    kind: EventKind::SignalWaitEnded,
                    data: Payload::of(&ended)?,
                }),
            }
        }
        Ok(Landing {
            lines,
            then: Then::Deliver(deliver),
        })
    }

    pub fn discards(payloads: Vec<Payload>, room: &Address, who: &str) -> Landing {
        Landing {
            lines: payloads
                .into_iter()
                .map(|data| Line {
                    who: who.to_owned(),
                    addr: room.clone(),
                    kind: EventKind::FileDiscarded,
                    data,
                })
                .collect(),
            then: Then::Nothing,
        }
    }

    pub fn shelf(
        effects: Vec<collab::ArchiveEffect>,
        write_root: &Path,
        building: &Address,
        at: TimeMs,
        room: &Address,
        who: &str,
    ) -> Result<Landing, AxError> {
        let mut lines = Vec::new();
        let mut filings = Vec::new();
        for collab::ArchiveEffect::Recorded { kind, text } in effects {
            let entry = city::archive_entry(
                write_root,
                building,
                city::ArchiveKind::parse(&kind)?,
                at,
                &text,
            )?;
            lines.push(Line {
                who: who.to_owned(),
                addr: room.clone(),
                kind: EventKind::AssetArchived,
                data: Payload::of(&kernel::event::record::AssetArchived {
                    kind: entry.kind.as_str().to_owned(),
                    day: entry.day,
                    subject: entry.subject.clone(),
                })?,
            });
            filings.push(Filing { entry, body: text });
        }
        Ok(Landing {
            lines,
            then: Then::Shelf(filings),
        })
    }
}

pub enum Claims {
    Landed(Box<Landing<Closing>>),

    Stale {
        node: NodeId,
        released: Box<Landing<Closing>>,
    },
}

impl Claims {
    pub fn of(
        effects: &[collab::ClaimEffect],
        on_disk: &str,
        path: PathBuf,
        room: &Address,
        who: &str,
    ) -> Result<Claims, AxError> {
        let mut lines = Vec::new();
        let mut text = on_disk.to_owned();
        for effect in effects {
            if !collab::still_true(&text, effect) {
                return Ok(Claims::Stale {
                    node: effect.id().clone(),
                    released: Box::new(Landing {
                        lines: released(effects, room, who)?,
                        then: Then::Nothing,
                    }),
                });
            }
            text = effect.apply(&text)?;
            let closes = match effect {
                collab::ClaimEffect::Claimed { .. } => continue,
                collab::ClaimEffect::PutDown { id, .. } => Some(id.clone()),
                collab::ClaimEffect::Split { parent, .. } => Some(parent.clone()),

                collab::ClaimEffect::Added { .. } => None,
            };

            let Some((kind, data)) = effect.line(who)? else {
                continue;
            };
            lines.push(Closing {
                line: Line {
                    who: who.to_owned(),
                    addr: room.clone(),
                    kind,
                    data,
                },
                closes,
            });
        }
        Ok(Claims::Landed(Box::new(Landing {
            lines,
            then: Then::Roadmap {
                path,
                base: on_disk.to_owned(),
                text,
            },
        })))
    }
}

fn released(
    effects: &[collab::ClaimEffect],
    room: &Address,
    who: &str,
) -> Result<Vec<Closing>, AxError> {
    effects
        .iter()
        .filter_map(|effect| {
            let closes = Some(effect.id().clone());
            let line = handed_back(effect, "the plan moved before this run landed", room, who);
            line.map(|line| line.map(|line| Closing { line, closes }))
                .transpose()
        })
        .collect()
}

pub fn handed_back(
    claim: &collab::ClaimEffect,
    note: &str,
    room: &Address,
    who: &str,
) -> Result<Option<Line>, AxError> {
    let put_down = match claim {
        collab::ClaimEffect::Claimed { id, item } => collab::ClaimEffect::PutDown {
            id: id.clone(),
            item: item.clone(),
            exit: kernel::PlanExit::Stopped {
                id: id.clone(),
                why: kernel::StopCause::HandedBack {
                    note: note.to_owned(),
                },
            },
        },
        collab::ClaimEffect::PutDown { .. }
        | collab::ClaimEffect::Split { .. }
        | collab::ClaimEffect::Added { .. } => {
            return Ok(None);
        }
    };
    Ok(put_down.line(who)?.map(|(kind, data)| Line {
        who: who.to_owned(),
        addr: room.clone(),
        kind,
        data,
    }))
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::panic,
    clippy::wildcard_enum_match_arm,
    reason = "test code"
)]
mod tests {
    use kernel::{Address, Tool};

    use super::{Claims, Then};

    const SNAPSHOT: &str = "# Roadmap

| # | Item | Weight | Needs | Status | Evidence |
|---|------|--------|-------|--------|----------|
| 1 | wire the kiln | 1 |  | Not started |  |
| 2 | glaze the pots | 1 |  | Not started |  |
";

    fn desk(run: u8) -> std::sync::Arc<std::sync::Mutex<collab::ClaimDesk>> {
        std::sync::Arc::new(std::sync::Mutex::new(collab::ClaimDesk::new(
            format!("potter@lab.{run}"),
            Address::parse("lab/room1").unwrap(),
            SNAPSHOT.to_owned(),
            collab::Booking::new(|_| Ok(())),
        )))
    }

    fn act(desk: &std::sync::Arc<std::sync::Mutex<collab::ClaimDesk>>, args: serde_json::Value) {
        let tool = collab::ClaimTool::new(desk.clone()).unwrap();
        tool.invoke(&kernel::ToolCall {
            id: "tu_1".to_owned(),
            name: kernel::ToolName::parse("plan").unwrap(),
            args: kernel::Payload::new(args.as_object().unwrap().clone()).unwrap(),
        })
        .unwrap();
    }

    fn landed(desk: &std::sync::Arc<std::sync::Mutex<collab::ClaimDesk>>, on_disk: &str) -> String {
        let mut desk = desk.lock().unwrap();
        let effects = desk.take_effects();
        let room = Address::parse("lab/room1").unwrap();
        match Claims::of(&effects, on_disk, "Roadmap.md".into(), &room, "potter").unwrap() {
            Claims::Landed(landing) => match landing.then {
                Then::Roadmap { text, .. } => text,
                _ => panic!("a claim lands on the plan"),
            },
            Claims::Stale { node, .. } => panic!("{node} read as moved"),
        }
    }

    #[test]
    fn a_run_that_splits_its_row_and_claims_a_leaf_lands_both() {
        let run = desk(1);
        act(&run, serde_json::json!({ "action": "claim", "node": "1" }));
        act(
            &run,
            serde_json::json!({
                "action": "split", "node": "1", "parts": ["run the cable", "test the element"]
            }),
        );
        act(
            &run,
            serde_json::json!({ "action": "claim", "node": "1.1" }),
        );
        let (effects, left) = {
            let mut desk = run.lock().unwrap();
            (desk.take_effects(), desk.roadmap().map(str::to_owned))
        };
        let room = Address::parse("lab/room1").unwrap();
        let landed =
            match Claims::of(&effects, SNAPSHOT, "Roadmap.md".into(), &room, "potter").unwrap() {
                Claims::Landed(landing) => {
                    let kinds: Vec<_> = landing
                        .lines
                        .iter()
                        .map(|closing| closing.line.kind)
                        .collect();
                    match landing.then {
                        Then::Roadmap { text, .. } => Some((kinds, text)),
                        _ => None,
                    }
                }
                Claims::Stale { .. } => None,
            };
        assert_eq!(
            landed,
            left.map(|text| (vec![kernel::EventKind::RoadmapSplit], text)),
            "the split and the leaf's claim both land, and only the split writes a line here"
        );
    }

    #[test]
    fn two_runs_landing_different_nodes_keep_both_rows() {
        let (first, second) = (desk(1), desk(2));
        act(
            &first,
            serde_json::json!({ "action": "claim", "node": "1" }),
        );
        let evidence = format!("cas:b3-{}", "ab".repeat(32));
        act(
            &first,
            serde_json::json!({ "action": "finish", "node": "1", "evidence": evidence }),
        );
        act(
            &second,
            serde_json::json!({ "action": "claim", "node": "2" }),
        );
        let after_first = landed(&first, SNAPSHOT);
        let after_second = landed(&second, &after_first);
        let status = |text: &str, id: &str| match kernel::spine::check_roadmap_shape(text) {
            kernel::RoadmapShape::WellFormed { rows } => rows
                .into_iter()
                .find(|row| row.id.to_string() == id)
                .map(|row| row.status),
            kernel::RoadmapShape::Malformed { .. } => None,
        };
        assert_eq!(
            (status(&after_second, "1"), status(&after_second, "2")),
            (
                Some(kernel::RoadmapStatus::Done),
                Some(kernel::RoadmapStatus::InProgress)
            ),
            "the second landing keeps the first run's finished row"
        );
    }
}
