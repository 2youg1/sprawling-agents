// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! What a fork writes and what it reads: the lineage line and the node
//! the mother must own, and a branch rebuilt from the mother's own lines
//! rather than from a verify of the whole history.

#![allow(clippy::wildcard_enum_match_arm, reason = "test code")]

use super::super::*;
use crate::worker::fixture::*;
use crate::worker::*;

#[test]
fn a_fork_records_lineage_and_refuses_a_node_the_mother_does_not_own() {
    let dir = tempfile::tempdir().unwrap();
    crate::worker::fixture::init_city(dir.path()).unwrap();
    let (base_url, _provider) = fake_openai(&["m-local"], vec![completion("done", None)]);
    let mut worker = worker_with_provider(dir.path(), &base_url, "m-local").unwrap();
    let room = Address::parse("lab/room1").unwrap();
    worker
        .handle(wire::Command::Dispatch {
            addr: room.clone(),
            task: "mother work".to_owned(),
            goal: "a lineage".to_owned(),
            policy: kernel::RunPolicy::of(kernel::Mode::Work),
            idem: kernel::IdemKey::derive(&RunId::CITY, kernel::Seq::FIRST, b"dispatch"),
            session: None,
            effort: None,
            model: None,
        })
        .unwrap();
    // Find the mother's run_started node in the verified chain.
    let verified =
        runtime::replay::verify_ledger_dir(&kernel::layout::CityLayout::new(dir.path()).ledger())
            .unwrap();
    let (mother, node) = verified
        .lines()
        .iter()
        .find_map(|line| match line {
            runtime::replay::VerifiedLine::Known { record, .. }
                if record.kind() == EventKind::RunStarted =>
            {
                Some((record.run(), record.seq()))
            }
            _ => None,
        })
        .expect("a dispatch writes run_started");
    // Seq 0 is the genesis, a city event: not the mother's node, so a
    // session cannot branch from it - and the refusal comes before the
    // room is cleared, because a session that cannot inherit must not
    // have cost the room its shape.
    let err = worker
        .handle(wire::Command::OpenSession {
            addr: room.clone(),
            carry: wire::Carry::Nothing,
            from: Some(kernel::Origin {
                run: mother,
                at_seq: kernel::Seq::FIRST,
            }),
            idem: kernel::IdemKey::derive(&RunId::CITY, kernel::Seq::FIRST, b"bad"),
        })
        .unwrap_err();
    assert!(err.subject().contains("not an event of run"), "{err}");

    // The branch itself, and then the dispatch that inherits: the lineage
    // is the *run's* fact, so it is written when a run begins - a session
    // that never dispatches never has one.
    worker
        .handle(wire::Command::OpenSession {
            addr: room.clone(),
            carry: wire::Carry::Nothing,
            from: Some(kernel::Origin {
                run: mother,
                at_seq: node,
            }),
            idem: kernel::IdemKey::derive(&RunId::CITY, kernel::Seq::FIRST, b"branch"),
        })
        .unwrap();
    worker
        .handle(wire::Command::Dispatch {
            addr: room.clone(),
            task: "carry on".to_owned(),
            goal: "the line continues".to_owned(),
            policy: kernel::RunPolicy::of(kernel::Mode::Work),
            idem: kernel::IdemKey::derive(&RunId::CITY, kernel::Seq::FIRST, b"carry-on"),
            session: None,
            effort: None,
            model: None,
        })
        .unwrap();
    let after =
        runtime::replay::verify_ledger_dir(&kernel::layout::CityLayout::new(dir.path()).ledger())
            .unwrap();
    let forked = after
        .lines()
        .iter()
        .filter_map(|line| match line {
            runtime::replay::VerifiedLine::Known { record, .. }
                if record.kind() == EventKind::RunForked =>
            {
                Some(record.clone())
            }
            runtime::replay::VerifiedLine::Known { .. }
            | runtime::replay::VerifiedLine::IgnoredUnknown { .. } => None,
        })
        .next()
        .expect("the branch is a ledger fact once a run inherits it");
    assert_eq!(forked.addr(), Some(&room));
    assert_eq!(
        forked.data().as_map().get("from"),
        Some(&serde_json::Value::from(mother.to_string())),
        "the payload names the mother"
    );
    assert_eq!(
        forked.data().as_map().get("at_seq"),
        Some(&serde_json::Value::from(node.value())),
        "and the line the conversation was cut at"
    );
    assert_ne!(
        forked.run(),
        mother,
        "the line belongs to the run that inherits"
    );
}

/// Rebuilding a branch's conversation reads the mother's own lines, not
/// the history: the history was verified when the city opened, and a
/// rebuild that verified it again would cost the whole ledger on every
/// branch.
///
/// A chain broken after genesis is the witness: a verify refuses it, and
/// the rebuild, which never reads that line, inherits as it would from an
/// intact ledger.
#[test]
fn inheriting_a_branch_does_not_verify_the_history() {
    let dir = tempfile::tempdir().unwrap();
    crate::worker::fixture::init_city(dir.path()).unwrap();
    let (base_url, _provider) =
        fake_openai(&["m-local"], vec![completion("the meter says 42", None)]);
    let mut worker = worker_with_provider(dir.path(), &base_url, "m-local").unwrap();
    worker
        .handle(wire::Command::Dispatch {
            addr: Address::parse("lab/room2").unwrap(),
            task: "mother".to_owned(),
            goal: "a number is written down".to_owned(),
            policy: kernel::RunPolicy::of(kernel::Mode::Work),
            idem: kernel::IdemKey::derive(&RunId::CITY, kernel::Seq::FIRST, b"mother"),
            session: None,
            effort: None,
            model: None,
        })
        .unwrap();
    let verified =
        runtime::replay::verify_ledger_dir(&kernel::layout::CityLayout::new(dir.path()).ledger())
            .unwrap();
    let origin = verified
        .lines()
        .iter()
        .find_map(|line| match line {
            runtime::replay::VerifiedLine::Known { record, .. }
                if record.kind() == EventKind::RunStarted =>
            {
                Some(kernel::Origin {
                    run: record.run(),
                    at_seq: record.seq(),
                })
            }
            runtime::replay::VerifiedLine::Known { .. }
            | runtime::replay::VerifiedLine::IgnoredUnknown { .. } => None,
        })
        .expect("the mother ran");
    let at = Assignment {
        addr: Address::parse("lab/room1").unwrap(),
        session: None,
        effort: None,
        model: None,
        policy: kernel::RunPolicy::of(kernel::Mode::Work),
        parent: None,
        succession: None,
        taint: kernel::TaintSet::empty(),
        dispatched_by: kernel::event::Who::Person,
        origin: Some(origin),
    };

    break_the_chain_after_genesis(dir.path());

    let inherited = worker.inherited(&at, RunId::from_bytes([7; 16]));
    assert!(
        inherited
            .as_ref()
            .is_ok_and(|messages| !messages.is_empty()),
        "a branch rebuild verified the history, or inherited nothing: {inherited:?}"
    );
}

/// A provider reuses a cached prompt only for a prefix that matches byte
/// for byte, so a branch in the same room opens with the bytes its
/// mother last sent: the four frozen segments, every other key of the
/// rendered request, and each message the mother exchanged
/// (sprawling-SPEC.md 8-141).
#[test]
fn a_branch_first_request_carries_the_bytes_of_the_mothers_last() {
    let dir = tempfile::tempdir().unwrap();
    crate::worker::fixture::init_city(dir.path()).unwrap();
    let (base_url, provider) = fake_openai(
        &["m-local"],
        vec![
            completion("the meter reads 42", None),
            completion("in feet it is 138", None),
        ],
    );
    let mut worker = worker_with_provider(dir.path(), &base_url, "m-local").unwrap();
    let room = Address::parse("lab/room1").unwrap();
    let talk = |task: &str, idem: &[u8]| wire::Command::Dispatch {
        addr: room.clone(),
        task: task.to_owned(),
        goal: String::new(),
        policy: kernel::RunPolicy::of(kernel::Mode::Work),
        idem: kernel::IdemKey::derive(&RunId::CITY, kernel::Seq::FIRST, idem),
        session: None,
        effort: None,
        model: None,
    };
    worker.handle(talk("measure the meter", b"mother")).unwrap();
    let verified =
        runtime::replay::verify_ledger_dir(&kernel::layout::CityLayout::new(dir.path()).ledger())
            .unwrap();
    let mother = verified
        .lines()
        .iter()
        .filter_map(|line| match line {
            runtime::replay::VerifiedLine::Known { record, .. }
                if record.kind() == EventKind::RunStarted =>
            {
                Some(record.run())
            }
            runtime::replay::VerifiedLine::Known { .. }
            | runtime::replay::VerifiedLine::IgnoredUnknown { .. } => None,
        })
        .next()
        .expect("the mother ran");
    let last = verified
        .lines()
        .iter()
        .filter_map(|line| match line {
            runtime::replay::VerifiedLine::Known { record, .. } if record.run() == mother => {
                Some(record.seq())
            }
            runtime::replay::VerifiedLine::Known { .. }
            | runtime::replay::VerifiedLine::IgnoredUnknown { .. } => None,
        })
        .next_back()
        .expect("the mother wrote lines");
    worker
        .handle(wire::Command::OpenSession {
            addr: room.clone(),
            carry: wire::Carry::Nothing,
            from: Some(kernel::Origin {
                run: mother,
                at_seq: last,
            }),
            idem: kernel::IdemKey::derive(&RunId::CITY, kernel::Seq::FIRST, b"branch"),
        })
        .unwrap();
    worker.handle(talk("and in feet?", b"daughter")).unwrap();

    let chats: Vec<serde_json::Value> = provider
        .bodies()
        .iter()
        .filter_map(|body| serde_json::from_str::<serde_json::Value>(body).ok())
        .filter(|body| body.get("messages").is_some())
        .collect();
    let [mothers_last, daughters_first] = chats.as_slice() else {
        panic!("one request each, not {}: {chats:?}", chats.len());
    };
    let sent = mothers_last["messages"].as_array().unwrap().len();
    let mut opening = daughters_first.clone();
    opening["messages"].as_array_mut().unwrap().truncate(sent);
    assert_eq!(&opening, mothers_last);
}
