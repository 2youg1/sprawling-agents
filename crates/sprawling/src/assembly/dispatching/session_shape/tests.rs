// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

#![allow(
    clippy::float_arithmetic,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::string_slice,
    clippy::arithmetic_side_effects,
    reason = "test code"
)]

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex, PoisonError};

use crate::assembly::fixture::*;
use crate::assembly::*;

/// One frozen segment as the record states it: which slot, which hash,
/// and how many bytes went into the prompt.
type Row = (String, String, u64);

/// Every prefix this city froze, read off the ledger as it is written.
///
/// A watcher rather than a read-back, because the two runs have to be
/// compared as the assembler produced them: a later re-assembly would
/// read files that may have moved since and would be comparing
/// something other than what the runs were sent.
type Prefixes = Arc<Mutex<Vec<Vec<Row>>>>;

fn watch(worker: &mut RunWorker, seen: &Prefixes) {
    let seen = Arc::clone(seen);
    worker.observe(Box::new(move |record: &EventRecord| {
        if record.kind() != EventKind::PromptAssembled {
            return;
        }
        let rows = record
            .data()
            .as_map()
            .get("segments")
            .and_then(serde_json::Value::as_array)
            .map(|rows| rows.iter().filter_map(row_of).collect())
            .unwrap_or_default();
        seen.lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push(rows);
    }));
}

fn row_of(row: &serde_json::Value) -> Option<Row> {
    Some((
        row.get("slot")?.as_str()?.to_owned(),
        row.get("hash")?.as_str()?.to_owned(),
        row.get("len")?.as_u64()?,
    ))
}

/// The bytes behind a recorded prefix, read back from the store. Equal
/// hashes already say the bytes are equal; this is the reading a person
/// can check without trusting the hash to mean what it says.
fn bytes_of(city_root: &std::path::Path, rows: &[Row]) -> BTreeMap<String, Vec<u8>> {
    let store = memory::Cas::open(&kernel::layout::CityLayout::new(city_root).cas()).unwrap();
    rows.iter()
        .map(|(slot, hash, _)| {
            let hash: kernel::B3Hash =
                serde_json::from_value(serde_json::Value::String(hash.clone())).unwrap();
            (slot.clone(), store.get(&hash).unwrap())
        })
        .collect()
}

fn ask(addr: &Address, effort: Option<kernel::Effort>, key: &[u8]) -> channels::Command {
    channels::Command::Dispatch {
        addr: addr.clone(),
        task: "write one line".to_owned(),
        goal: "the line is written".to_owned(),
        mode: kernel::Mode::PlanGoal,
        idem: kernel::IdemKey::derive(&RunId::CITY, kernel::Seq::FIRST, key),
        session: None,
        effort,
    }
}

fn choose(worker: &mut RunWorker, model: &str, key: &[u8]) {
    worker
        .handle(channels::Command::SelectModel {
            endpoint: channels::ProviderName::parse("house").unwrap(),
            model: model.to_owned(),
            tag: kernel::ModelTag::Main,
            context_tokens: kernel::Window::new(32_768),
            max_output_tokens: kernel::Ceiling::new(4_096),
            idem: kernel::IdemKey::derive(&RunId::CITY, kernel::Seq::FIRST, key),
        })
        .unwrap();
}

fn open_lab(dir: &std::path::Path) {
    init_city(dir).unwrap();
    city::create_building(
        dir,
        &Address::parse("lab").unwrap(),
        city::BuildingTemplate::Minimal,
    )
    .unwrap();
}

/// Two dispatches into one session send the same bytes, and a dispatch
/// that would change how hard it thinks is refused with the session
/// left exactly as it was.
///
/// This is the property roadmap 17.2 guards at the supply layer, moved
/// to the seam that can break it: the blocks a person pays to have
/// cached are only identical across two dispatches while the shape that
/// renders them is frozen with the session. A silent `effort` write is
/// how it broke, and the refusal is the way out — the two ways to keep
/// working, whether or not the session is running.
#[test]
fn a_session_keeps_the_shape_it_froze_and_refuses_a_different_effort() {
    let dir = tempfile::tempdir().unwrap();
    open_lab(dir.path());
    let (base_url, _provider) = fake_openai(
        &["m-local"],
        vec![
            completion("done", None),
            completion("done", None),
            completion("done", None),
        ],
    );
    let mut worker = worker_with_provider(dir.path(), &base_url, "m-local").unwrap();
    let room = Address::parse("lab/room1").unwrap();
    let seen: Prefixes = Arc::new(Mutex::new(Vec::new()));
    watch(&mut worker, &seen);

    worker
        .handle(ask(&room, Some(kernel::Effort::Low), b"dispatch-1"))
        .unwrap();
    worker
        .handle(ask(&room, Some(kernel::Effort::Low), b"dispatch-2"))
        .unwrap();

    let frozen = seen.lock().unwrap_or_else(PoisonError::into_inner).clone();
    assert_eq!(frozen.len(), 2, "one assembled prompt per dispatch");
    assert_eq!(
        frozen[0], frozen[1],
        "the second run in this session froze the same four segments"
    );
    assert_eq!(
        bytes_of(dir.path(), &frozen[0]),
        bytes_of(dir.path(), &frozen[1]),
        "and the bytes behind them are equal, slot by slot"
    );
    assert_eq!(
        frozen[0]
            .iter()
            .map(|(slot, _, _)| slot.as_str())
            .collect::<Vec<_>>(),
        ["city", "building", "resident", "run"],
        "the four slots, in the order the prefix concatenates them"
    );

    // A person asking for a different effort mid-session: refused, and
    // nothing of that dispatch reached the disk or the history.
    let refused = worker
        .handle(ask(&room, Some(kernel::Effort::High), b"dispatch-3"))
        .unwrap_err();
    assert_eq!(*refused.code(), AxCode::ConfigInvalid, "{refused}");
    assert!(
        refused.subject().contains("the effort changed"),
        "the refusal names the field that moved: {refused}"
    );
    assert!(
        refused
            .recovery()
            .contains("open a new session, or fork this run"),
        "and the two ways out: {refused}"
    );
    assert_eq!(
        city::settled_effort(dir.path(), &room)
            .unwrap()
            .map(|(effort, _)| effort),
        Some(kernel::Effort::Low),
        "the refused dispatch left the session's effort where it was"
    );
    assert_eq!(
        seen.lock().unwrap_or_else(PoisonError::into_inner).len(),
        2,
        "a refused dispatch assembled no prompt"
    );

    // And the session still works: the same request, which moves
    // nothing, is carried out and freezes the same bytes again.
    worker
        .handle(ask(&room, Some(kernel::Effort::Low), b"dispatch-4"))
        .unwrap();
    let after = seen.lock().unwrap_or_else(PoisonError::into_inner).clone();
    assert_eq!(after.len(), 3);
    assert_eq!(
        after[2], frozen[0],
        "the session runs on the prefix it froze"
    );
}

/// A model chosen after a session opened is a model for the sessions
/// that start next, never for the one already working.
///
/// `SelectModel` is city-wide configuration and stays free: blocking
/// it would forbid the ordinary act of pointing the main tag at another
/// model. What may not happen is that act reaching a session that
/// opened under the old one, where it would move the prefix the session
/// is paying to cache. The dispatch that would carry it is refused, and
/// a new session takes the new model.
#[test]
fn a_model_chosen_after_a_session_opened_does_not_reach_it() {
    let dir = tempfile::tempdir().unwrap();
    open_lab(dir.path());
    let (base_url, _provider) = fake_openai(
        &["m-local", "m-other"],
        vec![completion("done", None), completion("done", None)],
    );
    let mut worker = worker_with_provider(dir.path(), &base_url, "m-local").unwrap();
    let room = Address::parse("lab/room1").unwrap();

    worker.handle(ask(&room, None, b"dispatch-1")).unwrap();
    choose(&mut worker, "m-other", b"select-other");

    let refused = worker.handle(ask(&room, None, b"dispatch-2")).unwrap_err();
    assert_eq!(*refused.code(), AxCode::ConfigInvalid, "{refused}");
    assert!(
        refused.subject().contains("the model changed"),
        "the refusal names the field that moved: {refused}"
    );
    assert!(
        refused
            .recovery()
            .contains("open a new session, or fork this run"),
        "and the two ways out: {refused}"
    );
    assert_eq!(
        city::own_layer(dir.path(), &room).unwrap().model(),
        Some("m-local"),
        "the session still records the model it opened with"
    );

    // What the registry now says is what a session opened now gets.
    let elsewhere = Address::parse("lab/room2").unwrap();
    worker.handle(ask(&elsewhere, None, b"dispatch-3")).unwrap();
    assert_eq!(
        city::own_layer(dir.path(), &elsewhere).unwrap().model(),
        Some("m-other")
    );
}
