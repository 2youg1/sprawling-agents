// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use super::{asked, record, turns};
use kernel::Ledger;
use wire::{AxCode, AxError, EventDraft, EventKind, Payload, RunId, TimeMs};

/// A city whose ledger holds these lines, oldest first, under one run,
/// answered the way the page asks: rebuilt from disk, then `Query::Rounds`.
/// A close is only worth reading when the ledger is the one on disk, so the
/// test takes the production door rather than a fold handed its records.
fn closed_city(
    dir: &std::path::Path,
    run: RunId,
    drafts: Vec<(EventKind, serde_json::Value)>,
) -> wire::RoundsAnswer {
    let report = crate::worker::fixture::init_city(dir).unwrap();
    let mut ledger = storage::JsonlLedger::open(&report.ledger_dir, TimeMs::new(9))
        .unwrap()
        .0;
    for (offset, (kind, data)) in drafts.into_iter().enumerate() {
        ledger
            .append(EventDraft {
                run,
                t: TimeMs::new(u64::try_from(offset).unwrap().saturating_add(10)),
                who: "lab/parser".to_owned(),
                addr: Some(kernel::Address::parse("lab/parser").unwrap()),
                kind,
                data: Payload::new(data.as_object().unwrap().clone()).unwrap(),
                ig: false,
            })
            .unwrap();
    }
    drop(ledger);
    let mut views = crate::views::Views::rebuild(&report.ledger_dir).unwrap();
    let wire::Answer::Rounds(answer) = views.answer(&wire::Query::Rounds { run }) else {
        panic!("Rounds answers with rounds");
    };
    *answer
}

/// The reason a run stopped reaches the page from the close, not from a
/// carrier row the fold had no call to pair: a wave that refused to stage
/// leaves no `tool_result`, and a loadtime code leaves no row at all.
#[test]
fn a_failed_close_names_the_error_that_stopped_it() {
    let failed = AxError::failure(AxCode::WorktreeBusy, "stage a wave", "work/nested/")
        .with_recovery("resolve the repository state, then retry the wave");
    let dir = tempfile::tempdir().unwrap();
    let answer = closed_city(
        dir.path(),
        RunId::from_bytes([8u8; 16]),
        vec![
            (
                EventKind::RunStarted,
                serde_json::json!({ "task": "a thing" }),
            ),
            (EventKind::ModelCalled, serde_json::json!({})),
            (
                EventKind::RunFrozen,
                serde_json::json!({
                    "completion": "cancelled",
                    "failure": serde_json::to_value(&failed).unwrap(),
                }),
            ),
        ],
    );
    let closing = answer.closing.expect("the window held run_frozen");
    assert_eq!(closing.failure, Some(failed));
    assert_eq!(
        answer.turns.len(),
        1,
        "the turn the run opened still stands"
    );
}

/// A close written before the key existed still names the carrier that
/// stopped it, and one that did not stop there names nothing: an error met
/// on the way is not the reason a run finished.
#[test]
fn an_old_close_names_the_carrier_that_stopped_it() {
    let failed = AxError::failure(AxCode::BudgetExhausted, "ask the provider", "main")
        .with_recovery("raise the ceiling, then retry the turn");
    let payload = serde_json::to_value(&failed).unwrap();
    let unpaired = [
        asked(1),
        record(
            2,
            EventKind::ToolResult,
            serde_json::json!({ "error": payload }),
        ),
        record(
            3,
            EventKind::RunFrozen,
            serde_json::json!({ "completion": "cancelled" }),
        ),
    ];
    assert_eq!(
        turns(&unpaired)[0].calls.len(),
        0,
        "a carrier row with no call to pair is dropped by the fold, which is \
         why the close has to say it"
    );

    let stopped_dir = tempfile::tempdir().unwrap();
    let stopped = closed_city(
        stopped_dir.path(),
        RunId::from_bytes([9u8; 16]),
        vec![
            (EventKind::ModelCalled, serde_json::json!({})),
            (EventKind::BudgetLimit, payload.clone()),
            (
                EventKind::RunFrozen,
                serde_json::json!({ "completion": "cancelled" }),
            ),
        ],
    );
    assert_eq!(
        stopped.closing.expect("the window held run_frozen").failure,
        Some(failed.clone()),
        "an old close still names the carrier that stopped it"
    );

    let finished_dir = tempfile::tempdir().unwrap();
    let finished = closed_city(
        finished_dir.path(),
        RunId::from_bytes([10u8; 16]),
        vec![
            (EventKind::ModelCalled, serde_json::json!({})),
            (EventKind::BudgetLimit, payload),
            (
                EventKind::RunFrozen,
                serde_json::json!({ "completion": "limit" }),
            ),
        ],
    );
    assert_eq!(
        finished
            .closing
            .expect("the window held run_frozen")
            .failure,
        None,
        "a run the ceiling cut did not stop against the carrier it met"
    );
}
