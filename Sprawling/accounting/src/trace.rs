// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::collections::BTreeMap;
use std::ops::Range;
use std::path::Path;

use kernel::layout::CityLayout;
use kernel::{Address, AxError, EventKind, EventRecord, GitOid, RunId, Seq};
use storage::{LedgerIndex, LineReader, StorageError};

use crate::views::Views;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Trace {

    pub commit: wire::CommitAnswer,

    pub calls: Vec<wire::Call>,

    pub nearby: Vec<Nearby>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Nearby {
    pub run: RunId,

    pub actor: Address,
    pub calls: u64,
}

pub fn trace(city_root: &Path, oid: GitOid) -> Result<Option<Trace>, AxError> {
    #[cfg(test)]
    counted::views_folded();
    let wire::Answer::Commit(commit) = crate::views::ask(city_root, &wire::Query::Commit { oid })?
    else {
        return Ok(None);
    };
    let ledger_dir = CityLayout::new(city_root).ledger();
    let index = LedgerIndex::rebuild(&ledger_dir).map_err(StorageError::into_ax)?;
    trace_through(&index, &ledger_dir, *commit).map(Some)
}

pub(crate) struct History {

    views: Result<Box<Views>, AxError>,
}

impl History {
    pub(crate) fn new(city_root: &Path) -> History {
        #[cfg(test)]
        counted::views_folded();
        History {
            views: Ok(Box::new(Views::new(city_root))),
        }
    }

    pub(crate) fn absorb(&mut self, record: &EventRecord) {
        let refused = match &mut self.views {
            Ok(views) => views.apply(record).err(),
            Err(_) => None,
        };
        if let Some(refused) = refused {
            self.views = Err(refused);
        }
    }

    pub(crate) fn commit(&self, oid: GitOid) -> Result<Option<wire::CommitAnswer>, AxError> {
        let views = self.views.as_ref().map_err(AxError::clone)?;
        let wire::Answer::Commit(commit) = views.prepare(&wire::Query::Commit { oid }).finish()
        else {
            return Ok(None);
        };
        Ok(Some(*commit))
    }
}

pub(crate) fn trace_through(
    index: &LedgerIndex,
    ledger_dir: &Path,
    commit: wire::CommitAnswer,
) -> Result<Trace, AxError> {
    let mut reader = index.reader(ledger_dir);
    let after = commit.previous.map(|previous| previous.seq);
    let records = run_records(index, &mut reader, &commit, after)?;
    let span = Range {
        start: match after {
            Some(previous) => previous.next()?,
            None => records.first().map_or(commit.seq, EventRecord::seq),
        },
        end: commit.seq,
    };
    let calls = crate::views::turns(&records)
        .into_iter()
        .flat_map(|turn| turn.calls)
        .filter(|call| span.contains(&call.at))
        .collect();
    let nearby = nearby(index, &mut reader, &commit, &span)?;
    Ok(Trace {
        commit,
        calls,
        nearby,
    })
}

fn run_records(
    index: &LedgerIndex,
    reader: &mut LineReader<'_>,
    commit: &wire::CommitAnswer,
    after: Option<Seq>,
) -> Result<Vec<EventRecord>, AxError> {
    let mut newest_first = Vec::new();
    for seq in index.run_seqs_before(commit.run, Some(commit.seq)) {
        let record = EventRecord::parse_line(&reader.line_at(seq).map_err(StorageError::into_ax)?)?;
        let opens_a_turn = record.kind() == EventKind::ModelCalled;
        newest_first.push(record);
        if opens_a_turn && after.is_some_and(|after| seq <= after) {
            break;
        }
    }
    newest_first.reverse();
    Ok(newest_first)
}

fn nearby(
    index: &LedgerIndex,
    reader: &mut LineReader<'_>,
    commit: &wire::CommitAnswer,
    span: &Range<Seq>,
) -> Result<Vec<Nearby>, AxError> {
    let building = city::Building::of(&commit.actor)?;
    let mut found: BTreeMap<RunId, Nearby> = BTreeMap::new();
    let entries = index.seqs_from(span.start);
    #[cfg(test)]
    let entries = counted::Walked::new(entries);
    let walk = entries.take_while(|seq| *seq < span.end);
    for seq in walk {
        let record = EventRecord::parse_line(&reader.line_at(seq).map_err(StorageError::into_ax)?)?;
        let Some(at) = record.addr() else {
            continue;
        };
        if record.run() == commit.run
            || record.kind() != EventKind::ToolCalled
            || !at.is_within(building.addr())
        {
            continue;
        }
        let counted = found.entry(record.run()).or_insert_with(|| Nearby {
            run: record.run(),
            actor: at.clone(),
            calls: 0,
        });

        counted.calls = counted.calls.saturating_add(1);
    }
    Ok(found.into_values().collect())
}

#[cfg(test)]
pub(crate) mod counted {
    use std::cell::{Cell, RefCell};

    use kernel::Seq;

    thread_local! {
        static VIEWS_FOLDS: Cell<u64> = const { Cell::new(0) };
        static NEARBY_WALKS: RefCell<Vec<u64>> = const { RefCell::new(Vec::new()) };
    }

    pub(super) struct Walked<I> {
        entries: I,
        taken: u64,
    }

    impl<I> Walked<I> {
        pub(super) fn new(entries: I) -> Walked<I> {
            Walked { entries, taken: 0 }
        }
    }

    impl<I: Iterator<Item = Seq>> Iterator for Walked<I> {
        type Item = Seq;
        fn next(&mut self) -> Option<Seq> {
            let entry = self.entries.next()?;
            self.taken = self.taken.saturating_add(1);
            Some(entry)
        }
    }

    impl<I> Drop for Walked<I> {
        fn drop(&mut self) {
            NEARBY_WALKS.with(|walks| walks.borrow_mut().push(self.taken));
        }
    }

    pub(crate) fn nearby_walks() -> Vec<u64> {
        NEARBY_WALKS.with(|walks| walks.borrow().clone())
    }

    pub(super) fn views_folded() {
        VIEWS_FOLDS.with(|folds| folds.set(folds.get().saturating_add(1)));
    }

    pub(crate) fn views_folds() -> u64 {
        VIEWS_FOLDS.with(Cell::get)
    }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "test code"
)]
mod tests {
    use super::*;
    use kernel::{EventDraft, EventKind, Ledger, Payload, Seq, TimeMs};
    use serde_json::{Value, json};

    fn run(n: u8) -> RunId {
        RunId::from_bytes([n; 16])
    }

    fn oid(n: u8) -> GitOid {
        GitOid::parse(&format!("{n:02x}").repeat(20)).unwrap()
    }

    fn addr(raw: &str) -> Address {
        Address::parse(raw).unwrap()
    }

    type Line = (RunId, &'static str, EventKind, Value);

    fn called(id: &str, tool: &str, effect: Value) -> Value {
        json!({"id": id, "name": tool, "args": {"path": "src/a.rs"}, "subject": "src/a.rs", "effect": effect})
    }

    fn answered(id: &str, tool: &str) -> Value {
        json!({"tool_use_id": id, "name": tool, "result": {"content": "ok"}})
    }

    fn committed(n: u8) -> Value {
        json!({"oid": oid(n).to_string()})
    }

    fn city(script: Vec<Line>) -> (tempfile::TempDir, u64) {
        let dir = tempfile::tempdir().unwrap();
        let report = crate::worker::fixture::init_city(dir.path()).unwrap();
        let (mut ledger, _) =
            storage::JsonlLedger::open(&report.ledger_dir, TimeMs::new(1_000)).unwrap();
        let mut first = None;
        for (at, (run, room, kind, data)) in (1_000u64..).zip(script) {
            let written = ledger
                .append(EventDraft {
                    run,
                    t: TimeMs::new(at),
                    who: "tester".to_owned(),
                    addr: Some(addr(room)),
                    kind,
                    data: Payload::new(data.as_object().cloned().unwrap()).unwrap(),
                    ig: false,
                })
                .unwrap();
            first.get_or_insert(written.seq().value());
        }
        (dir, first.unwrap())
    }

    fn interleaved() -> (tempfile::TempDir, u64) {
        let read = || json!("read");
        let write = || json!({"write": {"domain": "lab"}});
        city(vec![
            (run(1), "lab/room1", EventKind::RunStarted, json!({})),
            (run(2), "lab/room2", EventKind::RunStarted, json!({})),
            (run(3), "yard/room3", EventKind::RunStarted, json!({})),
            (
                run(1),
                "lab/room1",
                EventKind::ModelCalled,
                json!({"segments": [], "model": "m"}),
            ),
            (
                run(1),
                "lab/room1",
                EventKind::ToolCalled,
                called("c1", "read", read()),
            ),
            (
                run(1),
                "lab/room1",
                EventKind::ToolResult,
                answered("c1", "read"),
            ),
            (
                run(1),
                "lab/room1",
                EventKind::CheckpointCommitted,
                committed(1),
            ),
            (
                run(1),
                "lab/room1",
                EventKind::ToolCalled,
                called("c2", "edit", write()),
            ),
            (
                run(2),
                "lab/room2",
                EventKind::ModelCalled,
                json!({"segments": [], "model": "m"}),
            ),
            (
                run(2),
                "lab/room2",
                EventKind::ToolCalled,
                called("d1", "exec", json!("egress")),
            ),
            (
                run(2),
                "lab/room2",
                EventKind::ToolResult,
                answered("d1", "exec"),
            ),
            (
                run(2),
                "lab/room2",
                EventKind::CheckpointCommitted,
                committed(2),
            ),
            (
                run(1),
                "lab/room1",
                EventKind::ToolResult,
                answered("c2", "edit"),
            ),
            (
                run(3),
                "yard/room3",
                EventKind::ToolCalled,
                called("e1", "edit", write()),
            ),
            (
                run(1),
                "lab/room1",
                EventKind::ToolCalled,
                called("c3", "read", read()),
            ),
            (
                run(1),
                "lab/room1",
                EventKind::ToolResult,
                answered("c3", "read"),
            ),
            (
                run(1),
                "lab/room1",
                EventKind::CheckpointCommitted,
                committed(3),
            ),
        ])
    }

    type Said = (
        Option<wire::CommitAt>,
        Vec<(Seq, String, Option<kernel::Effect>, wire::Outcome)>,
        Vec<Nearby>,
    );

    fn said(trace: Trace) -> Said {
        (
            trace.commit.previous,
            trace
                .calls
                .into_iter()
                .map(|call| (call.at, call.tool, call.effect, call.outcome))
                .collect(),
            trace.nearby,
        )
    }

    #[test]
    fn a_trace_names_the_calls_its_run_made_since_its_previous_commit() {
        let (dir, first) = interleaved();
        let seq = |line: u64| Seq::new(first + line);
        let write = Some(kernel::Effect::Write {
            domain: addr("lab"),
        });
        assert_eq!(
            trace(dir.path(), oid(3)).unwrap().map(said),
            Some((
                Some(wire::CommitAt {
                    oid: oid(1),
                    seq: seq(6)
                }),
                vec![
                    (seq(7), "edit".to_owned(), write, wire::Outcome::Answered),
                    (
                        seq(14),
                        "read".to_owned(),
                        Some(kernel::Effect::Read),
                        wire::Outcome::Answered
                    ),
                ],
                vec![Nearby {
                    run: run(2),
                    actor: addr("lab/room2"),
                    calls: 1
                }],
            ))
        );
    }

    #[test]
    fn a_first_commit_is_traced_from_its_runs_first_line() {
        let (dir, first) = interleaved();
        let seq = |line: u64| Seq::new(first + line);
        assert_eq!(
            trace(dir.path(), oid(2)).unwrap().map(said),
            Some((
                None,
                vec![(
                    seq(9),
                    "exec".to_owned(),
                    Some(kernel::Effect::Egress),
                    wire::Outcome::Answered
                )],
                vec![Nearby {
                    run: run(1),
                    actor: addr("lab/room1"),
                    calls: 2
                }],
            ))
        );
        assert_eq!(trace(dir.path(), oid(9)).unwrap(), None);
    }
}
