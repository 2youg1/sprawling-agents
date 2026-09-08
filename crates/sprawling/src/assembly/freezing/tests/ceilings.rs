// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! What a frozen run runs under: the ceiling the work that sent it was
//! given, the ceiling an answer carries on under, and the effort a config
//! layer states.

use super::super::*;
use crate::assembly::fixture::*;
use crate::assembly::*;

/// Work handed down is the same work, so it is done under the same
/// ceiling.
///
/// `knock` already carries the speaker's `BudgetCap` and says why in
/// its own comment; delegation is the stronger case of the same
/// thing and was the one path that zeroed it. What that costs today
/// is not overspending - nothing enforces a cap yet, and `StatusTool`
/// is its only reader - but a delegate telling a model its budget is
/// zero while its parent was told the truth. That is the defect
/// section 8-12 already named: a model that calls `status` and gets a
/// row of noughts learns not to ask again.
///
/// Both halves are asserted, so the test cannot pass by carrying
/// nothing anywhere.
#[test]
fn work_handed_down_is_done_under_the_ceiling_that_sent_it() {
    let dir = tempfile::tempdir().unwrap();
    init_city(dir.path()).unwrap();
    let hand_down = |id: &str| {
        completion_with(
            "handing it down",
            "delegate",
            id,
            serde_json::json!({
                "room": "lab/helper",
                "task": "measure the thing",
                "goal": "a number, then stop",
            }),
        )
    };
    let read_situation =
        |id: &str| completion_with("what am I under", "status", id, serde_json::json!({}));
    let (base_url, provider) = fake_openai(
        &["m-local"],
        vec![
            // First dispatch, on the default ceiling. Its only job is
            // to get the person's answer on record, which settles the
            // Policy that lets the second dispatch delegate without
            // stopping - so the run under test never goes near the
            // approval resumption path.
            hand_down("tu_1"),
            completion("waiting on a person", None),
            hand_down("tu_2"),
            completion("parent done", None),
            completion("child done", None),
            // Second dispatch, carrying a real ceiling. The parent
            // reads its situation first - that reading is the control
            // arm, green before this card and after it - then hands
            // the work down.
            read_situation("tu_3"),
            hand_down("tu_4"),
            completion("parent done", None),
            // The child reads its own. This is the arm the card is
            // about, and it said zero.
            read_situation("tu_5"),
            completion("child done", None),
        ],
    );
    let mut worker = worker_with_provider(dir.path(), &base_url, "m-local").unwrap();
    worker
        .handle(channels::Command::CreateBuilding {
            addr: Address::parse("lab").unwrap(),
            template: channels::TemplateName::parse("minimal").unwrap(),
            idem: kernel::IdemKey::derive(&RunId::CITY, kernel::Seq::FIRST, b"create"),
        })
        .unwrap();
    let ceiling = kernel::BudgetCap {
        usd: kernel::UsdMicros::new(250_000),
        tokens: kernel::Tokens::new(4_000),
    };
    fn send(worker: &mut RunWorker, budget: kernel::BudgetCap, tag: &[u8]) {
        worker
            .handle(channels::Command::Dispatch {
                addr: Address::parse("lab/room1").unwrap(),
                task: "get it measured".to_owned(),
                goal: "the number is written down, then stop".to_owned(),
                mode: channels::ModeTag::parse("plan").unwrap(),
                budget,
                idem: kernel::IdemKey::derive(&RunId::CITY, kernel::Seq::FIRST, tag),
                session: None,
                effort: None,
            })
            .unwrap();
    }
    send(
        &mut worker,
        kernel::BudgetCap::default(),
        b"settle-the-policy",
    );
    allow_the_one_pending_item(&mut worker);
    send(&mut worker, ceiling, b"under-a-real-ceiling");

    let stated = "budget: 250000 usd_micros, 4000 tokens".to_owned();
    let asked = provider.bodies();

    // The control arm: the run the person dispatched was already told
    // the truth, so this stays green on both sides of the fix and a
    // regression here means the test stopped reaching the code.
    assert_eq!(
        ceiling_read_by(&asked, "lab/room1"),
        vec![stated.clone()],
        "the dispatched run reads the ceiling it was sent with"
    );
    // The arm this card is about. Before the fix it read
    // "budget: 0 usd_micros, 0 tokens".
    assert_eq!(
        ceiling_read_by(&asked, "lab/helper"),
        vec![stated],
        "work handed down is done under the ceiling that sent it"
    );
}

/// Whose reading is whose. A conversation carries its earlier turns,
/// so one run's status answer appears in every later request of that
/// same run - counting bodies counts one run many times and passes
/// without a fix. What tells readings apart is the address inside
/// the status block itself, and the answer is the set of distinct
/// ceilings read at that address rather than how many times.
fn ceiling_read_by(bodies: &[String], room: &str) -> Vec<String> {
    let mut readings = Vec::new();
    for body in bodies {
        // The status text is a JSON string inside a JSON string, so
        // one newline arrives as two backslashes and an `n`.
        for (at, _) in body.match_indices(&format!("addr: {room}\\\\n")) {
            let window = body
                .get(at..body.len().min(at.saturating_add(240)))
                .unwrap_or_default();
            let Some(from) = window.find("budget: ") else {
                continue;
            };
            let tail = window.get(from..).unwrap_or_default();
            let line = tail.split("\\\\n").next().unwrap_or_default();
            readings.push(line.to_owned());
        }
    }
    readings.sort_unstable();
    readings.dedup();
    readings
}

/// An answer carries the work on, and the work it carries on is the
/// same work - so it runs under the ceiling that sent it.
///
/// The resumption path went through `fn dispatch`, which writes
/// `BudgetCap::default()`, and no word could fix that: `BlockedJob`
/// was rebuilt out of `run_started`, and that record did not carry a
/// ceiling for it to be rebuilt from. Both runs here stand at the
/// same address, so the discriminator is not `addr:` but the set of
/// distinct ceilings read there: two values before this card, one
/// after.
#[test]
fn work_resumed_by_an_answer_is_done_under_the_ceiling_that_sent_it() {
    let dir = tempfile::tempdir().unwrap();
    let report = init_city(dir.path()).unwrap();
    let read_situation =
        |id: &str| completion_with("what am I under", "status", id, serde_json::json!({}));
    let (base_url, provider) = fake_openai(
        &["m-local"],
        vec![
            read_situation("tu_1"),
            completion("stopping here", None),
            // The run the answer carries on: it reads its own
            // situation, which is the arm this card is about.
            read_situation("tu_2"),
            completion("carried on", None),
        ],
    );
    let mut worker = worker_with_provider(dir.path(), &base_url, "m-local").unwrap();
    let addr = Address::parse("lab/room1").unwrap();
    let ceiling = kernel::BudgetCap {
        usd: kernel::UsdMicros::new(250_000),
        tokens: kernel::Tokens::new(4_000),
    };
    worker
        .handle(channels::Command::Dispatch {
            addr: addr.clone(),
            task: "empty the archive".to_owned(),
            goal: "one sweep, then stop".to_owned(),
            mode: channels::ModeTag::parse("plan").unwrap(),
            budget: ceiling,
            idem: kernel::IdemKey::derive(&RunId::CITY, kernel::Seq::FIRST, b"under-a-cap"),
            session: None,
            effort: None,
        })
        .unwrap();

    // The item that run left waiting, recorded the way the bench
    // records one: against the run and the address, because
    // answering it later has to find the work it was holding up.
    let run = {
        let verified = runtime::replay::verify_ledger_dir(&report.ledger_dir).unwrap();
        let mut found = None;
        for line in verified.raw_lines() {
            let record = EventRecord::parse_line(line).unwrap();
            if record.kind() == EventKind::RunStarted {
                found = Some(record.run());
            }
        }
        found.expect("a dispatch starts a run")
    };
    let item = kernel::ApprovalItem {
        id: kernel::ApprovalId::new("item-1").unwrap(),
        source: kernel::ApprovalSource::Gate,
        actor: "lab/room1".to_owned(),
        action_desc: "empty the archive".to_owned(),
        artifact: Locator::parse("file:lab/room1@0000000000000000000000000000000000000000")
            .unwrap(),
        cluster_key: kernel::ClusterKey {
            class: kernel::ApprovalClass::DiscardEscalate,
            detail: "lab/room1".to_owned(),
        },
        created: TimeMs::new(1),
        tainted: false,
    };
    let value = serde_json::to_value(&item).unwrap();
    worker
        .record_for(
            run,
            effect::Line {
                who: "lab/room1".to_owned(),
                addr,
                kind: EventKind::ApprovalRequested,
                data: Payload::new(value.as_object().cloned().unwrap()).unwrap(),
            },
        )
        .unwrap();

    worker
        .handle(channels::Command::Approve {
            item: kernel::ApprovalId::new("item-1").unwrap(),
            verdict: kernel::PolicyVerdict::Allow,
            idem: kernel::IdemKey::derive(&RunId::CITY, kernel::Seq::FIRST, b"approve"),
        })
        .unwrap();

    assert_eq!(
        ceiling_read_by(&provider.bodies(), "lab/room1"),
        vec!["budget: 250000 usd_micros, 4000 tokens".to_owned()],
        "both the run the person sent and the run their answer carried on read one ceiling, \
         the one that was sent"
    );
}

#[test]
fn the_effort_a_config_layer_states_is_what_goes_out_on_the_wire() {
    let dir = tempfile::tempdir().unwrap();
    init_city(dir.path()).unwrap();
    let room = Address::parse("lab/room1").unwrap();
    let city_layer = city::config_path(dir.path(), &room, city::Layer::City).unwrap();
    std::fs::create_dir_all(city_layer.parent().unwrap()).unwrap();
    std::fs::write(&city_layer, "[model]\neffort = \"low\"\n").unwrap();
    let building_layer = city::config_path(dir.path(), &room, city::Layer::Building).unwrap();
    std::fs::create_dir_all(building_layer.parent().unwrap()).unwrap();
    std::fs::write(&building_layer, "[model]\neffort = \"xhigh\"\n").unwrap();

    let (base_url, provider) = fake_openai(&["m-local"], vec![completion("done", None)]);
    let mut worker = worker_with_provider(dir.path(), &base_url, "m-local").unwrap();
    worker
        .handle(channels::Command::Dispatch {
            addr: room,
            task: "think about it".to_owned(),
            goal: "answer".to_owned(),
            mode: channels::ModeTag::parse("plan").unwrap(),
            budget: kernel::BudgetCap::default(),
            idem: kernel::IdemKey::derive(&RunId::CITY, kernel::Seq::FIRST, b"dispatch"),
            session: None,
            effort: None,
        })
        .unwrap();

    let asked = provider.bodies().join("\n");
    assert!(
        asked.contains("\"effort\":\"xhigh\""),
        "the building layer overrides the city layer, and the resolved level is what the \
         provider was asked for: {asked}"
    );
    assert!(!asked.contains("\"effort\":\"low\""));
}
