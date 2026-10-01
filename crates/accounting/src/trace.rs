// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Which calls a commit is the result of (accounting-SPEC.md 8-16).
//!
//! A commit is traced back from what `Query::Commit` already answers:
//! the run that wrote it and the commit that run announced before it.
//! Between the two, the run's own calls are the candidates, read with
//! the pairing rule the rounds fold owns; the other runs that called
//! tools in the same building meanwhile are counted beside them,
//! because their writes share the building's tree and may have landed
//! in the same commit.

use std::collections::BTreeMap;
use std::ops::Range;
use std::path::Path;

use kernel::{Address, AxError, EventKind, EventRecord, GitOid, RunId, Seq};
use storage::{LedgerIndex, LineReader, StorageError};

/// One commit, the calls its run made since its previous commit, and
/// who else called tools in the same building in that span.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Trace {
    /// The same value `Query::Commit` answers: run, actor, previous,
    /// parents.
    pub commit: wire::CommitAnswer,
    /// The run's calls in the span, oldest first.
    pub calls: Vec<wire::Call>,
    /// Every other run that called tools in the same building in the
    /// span, one entry each, by run id.
    pub nearby: Vec<Nearby>,
}

/// Another run that called tools in the commit's building in the span.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Nearby {
    pub run: RunId,
    /// The address of its first such call in the span.
    pub actor: Address,
    pub calls: u64,
}

/// The trace of `oid`, read from the city's own history; `None` when
/// this city never wrote that commit.
///
/// # Errors
/// Whatever `views::ask` refuses (a chain that does not verify), and a
/// line that cannot be read or parsed after it.
pub fn trace(city_root: &Path, oid: GitOid) -> Result<Option<Trace>, AxError> {
    let wire::Answer::Commit(commit) = crate::views::ask(city_root, &wire::Query::Commit { oid })?
    else {
        return Ok(None);
    };
    traced(city_root, *commit).map(Some)
}

/// The trace of `oid` as the line that first announced it, `line`, sees
/// it: the views' `previous` and `parents`, with the run, the actor, the
/// seq and the moment taken from `line` rather than from the commit's
/// latest announcement, so no later line changes the answer
/// (accounting-SPEC.md 8-16). `None` when this city never wrote that
/// commit, or `line` has no address.
///
/// # Errors
/// What [`trace`] refuses.
pub fn trace_first(
    city_root: &Path,
    oid: GitOid,
    line: &EventRecord,
) -> Result<Option<Trace>, AxError> {
    let (wire::Answer::Commit(commit), Some(actor)) = (
        crate::views::ask(city_root, &wire::Query::Commit { oid })?,
        line.addr(),
    ) else {
        return Ok(None);
    };
    let first = wire::CommitAnswer {
        run: line.run(),
        actor: actor.clone(),
        seq: line.seq(),
        at: line.t(),
        ..*commit
    };
    traced(city_root, first).map(Some)
}

/// The calls `commit`'s run made in its span, and who else called tools
/// in its building meanwhile.
fn traced(city_root: &Path, commit: wire::CommitAnswer) -> Result<Trace, AxError> {
    let ledger_dir = kernel::layout::CityLayout::new(city_root).ledger();
    let index = LedgerIndex::rebuild(&ledger_dir).map_err(StorageError::into_ax)?;
    let mut reader = index.reader(&ledger_dir);
    let after = commit.previous.map(|previous| previous.seq);
    let records = run_records(&index, &mut reader, &commit, after)?;
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
    let nearby = nearby(&index, &mut reader, &commit, &span)?;
    Ok(Trace {
        commit,
        calls,
        nearby,
    })
}

/// The commit's run's lines before the commit, oldest first: back past
/// `after` to the nearest `model_called` at or before it, so the turn
/// the span opens in is folded whole and its calls keep their answers.
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

/// The other runs whose `tool_called` lines in `span` were addressed
/// within the commit's building, counted by run.
fn nearby(
    index: &LedgerIndex,
    reader: &mut LineReader<'_>,
    commit: &wire::CommitAnswer,
    span: &Range<Seq>,
) -> Result<Vec<Nearby>, AxError> {
    let building = city::Building::of(&commit.actor)?;
    let mut found: BTreeMap<RunId, Nearby> = BTreeMap::new();
    let walk = index
        .seqs()
        .skip_while(|seq| *seq < span.start)
        .take_while(|seq| *seq < span.end);
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
        // A count of ledger lines cannot reach u64::MAX: the seq that
        // numbers them is a u64 too.
        counted.calls = counted.calls.saturating_add(1);
    }
    Ok(found.into_values().collect())
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

    /// What one line of the script says: who, where, what.
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

    /// A city whose ledger carries `script` after its genesis, each line
    /// at its own moment; the seq of the script's first line.
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

    /// Run 1 works in `lab/room1` and commits twice inside one turn; run
    /// 2 works in `lab/room2` and commits in between; run 3 works in
    /// another building.
    fn interleaved() -> (tempfile::TempDir, u64) {
        let read = || json!("read");
        let write = || json!({"write": {"domain": "lab"}});
        city(vec![
            (run(1), "lab/room1", EventKind::RunStarted, json!({})),
            (run(2), "lab/room2", EventKind::RunStarted, json!({})),
            (run(3), "yard/room3", EventKind::RunStarted, json!({})),
            (run(1), "lab/room1", EventKind::ModelCalled, json!({})),
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
            (run(2), "lab/room2", EventKind::ModelCalled, json!({})),
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

    /// What a trace says, in the terms a reader checks it by: the
    /// previous commit, each call's seq, tool, registration and outcome,
    /// and who else called in the building.
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

    /// The run's second commit is traced to the calls it made since its
    /// first - across the turn that commit fell in the middle of, and past
    /// the other run's commit in between - and the other run in the same
    /// building is counted beside them; the run in another building is
    /// not.
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

    /// A run's first commit is traced from the run's first line, and a
    /// commit this city never wrote has no trace.
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
