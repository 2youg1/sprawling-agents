// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! What a run's desks left behind: the lines the history takes, and the
//! change the city may not make before them.
//!
//! `docs/glossary.md` defines the Ledger as the only history, and says
//! every effect becomes an EventRecord first. ARCHITECTURE.md section 5
//! calls that ordering the design's load-bearing rule rather than a
//! logging preference. It used to be spelled out once per desk inside
//! `dispatch_in`, and two of the six spelled it backwards - the shared
//! plan was written and the shelf was filed before the lines that
//! announce them.
//!
//! Here the order is a property of the types instead. [`Then`] - the
//! change - is reachable only out of [`Landing::record`], and that
//! method appends every line before it hands the change over. Writing
//! the other order means obtaining a value that has no other source.

use std::path::{Path, PathBuf};

use kernel::{Address, AxError, EventKind, NodeId, Payload, TimeMs};

/// One line of the history, attributed to whoever caused it.
///
/// The attribution travels with the line rather than with the batch: a
/// signal is recorded against the room it is going to, and consuming one
/// is recorded against the resident that read it.
pub(crate) struct Line {
    pub(crate) who: String,
    pub(crate) addr: Address,
    pub(crate) kind: EventKind,
    pub(crate) data: Payload,
}

/// What the city does once a desk's lines are on the ledger.
///
/// Every arm is a change somebody can observe without reading the
/// history: a room's queue, the goal register, the shared plan, the
/// building's shelf. That is exactly why none of them may happen first.
pub(crate) enum Then {
    /// The line was the whole of it.
    Nothing,
    /// Put each signal in the room it names, and knock on that door.
    Deliver(Vec<collab::Signal>),
    /// Hold this ground in the city's goal register.
    Hold(Vec<kernel::GoalEntry>),
    /// Replace the shared plan with `text`, the run's effects replayed
    /// onto `base`, only while the file still reads `base`.
    Roadmap {
        path: PathBuf,
        base: String,
        text: String,
    },
    /// Put these on the building's shelf.
    Shelf(Vec<Filing>),
}

/// One entry and the body it carries.
///
/// The entry already names its own place: `city::archive_entry` decides
/// that from the kind, the instant and the subject, none of which needs
/// a disk. That is what lets the line be written first.
pub(crate) struct Filing {
    pub(crate) entry: city::ArchiveEntry,
    pub(crate) body: String,
}

/// Everything one desk left behind, resolved.
///
/// Both fields are private, and `then` leaves only through
/// [`Landing::record`].
pub(crate) struct Landing<L = Line> {
    lines: Vec<L>,
    then: Then,
}

/// One line of a plan's landing and the node whose claim it closes; a
/// split closes nothing.
pub(crate) struct Closing {
    pub(crate) line: Line,
    pub(crate) closes: Option<NodeId>,
}

impl<L> Landing<L> {
    /// Appends every line, then hands back the change that follows them.
    ///
    /// # Errors
    /// Propagates the first line the ledger refuses, and makes no change
    /// at all in that case: the change is the return value, so a caller
    /// that never receives it cannot apply it.
    pub(crate) fn record(
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
    /// What a run said, and what it read out of its own queue.
    ///
    /// # Errors
    /// Propagates a signal whose payload cannot be built.
    pub(crate) fn signals(
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
            }
        }
        Ok(Landing {
            lines,
            then: Then::Deliver(deliver),
        })
    }

    /// What ground a run claimed, and where two claims met.
    ///
    /// # Errors
    /// Propagates an entry or a conflict whose payload cannot be built.
    pub(crate) fn goals(
        effects: Vec<collab::GoalEffect>,
        room: &Address,
        who: &str,
    ) -> Result<Landing, AxError> {
        let mut lines = Vec::new();
        let mut hold = Vec::new();
        for effect in effects {
            match effect {
                collab::GoalEffect::Registered(entry) => {
                    lines.push(Line {
                        who: who.to_owned(),
                        addr: room.clone(),
                        kind: EventKind::GoalRegistered,
                        data: Payload::of(&entry)?,
                    });
                    hold.push(entry);
                }
                collab::GoalEffect::Conflicted { entry, level } => {
                    lines.push(Line {
                        who: who.to_owned(),
                        addr: room.clone(),
                        kind: EventKind::GoalConflict,
                        data: collab::conflict_payload(&entry, &level)?,
                    });
                }
            }
        }
        Ok(Landing {
            lines,
            then: Then::Hold(hold),
        })
    }

    /// What a wave deleted, as the sweep found it. The payloads carry
    /// their own way back, so there is nothing left for the city to do
    /// once they are on the ledger.
    pub(crate) fn discards(payloads: Vec<Payload>, room: &Address, who: &str) -> Landing {
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

    /// What a run asked its building to remember.
    ///
    /// `at` is one stamp for the whole batch rather than one per entry:
    /// a settlement is one moment, and two entries filed by one run
    /// should not be able to land on two different days.
    ///
    /// # Errors
    /// Propagates a kind outside the four, an entry with no subject, and
    /// a payload that cannot be built.
    pub(crate) fn shelf(
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

/// What a run's claims on the shared plan came to. Two answers and no
/// third: either every effect still matches the file as it stands and
/// all of them are replayed onto it, or one of them does not and nothing
/// at all is written.
pub(crate) enum Claims {
    Landed(Box<Landing<Closing>>),
    /// The nodes that moved, so a person can be told which, and the
    /// `roadmap_released` lines that close the claims this run already
    /// put on the ledger when the model made them.
    Stale {
        nodes: Vec<String>,
        released: Box<Landing<Closing>>,
    },
}

impl Claims {
    /// Checks each effect against the file as it stands now rather than
    /// as it stood when the run was dispatched. The accounting thread
    /// already refused a node another run in flight held when the model
    /// claimed it (`serving::booking`); this is the backstop for a run
    /// that claimed from its old copy a node somebody had since landed,
    /// whose claim is dropped rather than written over that row.
    ///
    /// The effects are replayed onto `on_disk` rather than the desk's
    /// dispatch-time copy written back, so a row another run landed
    /// since this one was dispatched is kept.
    ///
    /// # Errors
    /// Propagates a claim whose payload cannot be built, or an effect
    /// the plan on disk refuses to take.
    pub(crate) fn of(
        effects: &[collab::ClaimEffect],
        on_disk: &str,
        path: PathBuf,
        room: &Address,
        who: &str,
    ) -> Result<Claims, AxError> {
        // Only the *first* effect on each node is checked against the
        // disk. A run that claims a node and then closes it produced
        // both, in that order, from the file as it stood when the run
        // was dispatched; asking the disk whether the second one still
        // holds would ask whether this run's own earlier effect had
        // already landed, and it has not — the desk writes the whole
        // file once, at the end.
        let mut asked = std::collections::BTreeSet::new();
        let stale: Vec<String> = effects
            .iter()
            .filter(|effect| asked.insert(effect.id().to_string()))
            .filter(|effect| !collab::still_true(on_disk, effect))
            .map(|effect| effect.id().to_string())
            .collect();
        if !stale.is_empty() {
            return Ok(Claims::Stale {
                nodes: stale,
                released: Box::new(Landing {
                    lines: released(effects, room, who)?,
                    then: Then::Nothing,
                }),
            });
        }
        let mut lines = Vec::new();
        let mut text = on_disk.to_owned();
        for effect in effects {
            text = effect.apply(&text)?;
            let closes = match effect {
                // The claim's line went on the ledger when the accounting
                // thread booked it (`serving::booking`); writing it again
                // here would count the node as claimed twice.
                collab::ClaimEffect::Claimed { .. } => continue,
                collab::ClaimEffect::PutDown { id, .. } => Some(id.clone()),
                collab::ClaimEffect::Split { parent, .. } => Some(parent.clone()),
            };
            lines.push(Closing {
                line: Line {
                    who: who.to_owned(),
                    addr: room.clone(),
                    // Which record this is, is the effect's own answer: the
                    // exit decided it, and a second match here would be a
                    // second opinion about what a stop means.
                    kind: effect.kind(),
                    data: effect.payload(who)?,
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

/// A `roadmap_released` line for each claim the accounting thread
/// booked, for a landing that writes nothing else: without it the
/// history reads the node as held by this run for ever.
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

/// The `roadmap_released` line that hands a claimed node back, saying
/// why in `note`, or `None` for an effect that is not a claim. The one
/// shape both a stale landing and a run that came home without landing
/// close a claim with (sprawling-SPEC.md 8-42-8).
///
/// # Errors
/// Propagates a payload that will not build.
pub(crate) fn handed_back(
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
        collab::ClaimEffect::PutDown { .. } | collab::ClaimEffect::Split { .. } => {
            return Ok(None);
        }
    };
    Ok(Some(Line {
        who: who.to_owned(),
        addr: room.clone(),
        kind: put_down.kind(),
        data: put_down.payload(who)?,
    }))
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::panic, reason = "test code")]
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
        let mut tool = collab::ClaimTool::new(desk.clone()).unwrap();
        tool.invoke(&kernel::ToolCall {
            id: "tu_1".to_owned(),
            name: kernel::ToolName::parse("plan").unwrap(),
            args: kernel::Payload::new(args.as_object().unwrap().clone()).unwrap(),
        })
        .unwrap();
    }

    /// What the plan reads after this run lands on `on_disk`.
    fn landed(desk: &std::sync::Arc<std::sync::Mutex<collab::ClaimDesk>>, on_disk: &str) -> String {
        let mut desk = desk.lock().unwrap();
        let effects = desk.take_effects();
        let room = Address::parse("lab/room1").unwrap();
        match Claims::of(&effects, on_disk, "Roadmap.md".into(), &room, "potter").unwrap() {
            Claims::Landed(landing) => match landing.then {
                Then::Roadmap { text, .. } => text,
                _ => panic!("a claim lands on the plan"),
            },
            Claims::Stale { nodes, .. } => panic!("{nodes:?} read as moved"),
        }
    }

    /// Two runs read one plan and take different nodes. The one that
    /// lands second writes its effects onto the plan as the first left
    /// it, so the first run's finished row survives the second landing.
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
