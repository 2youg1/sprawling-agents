// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use super::addr;
use crate::worker::fixture::*;
use crate::worker::*;

fn a_mother_ran(city_root: &std::path::Path) -> (RunWorker, kernel::Origin, impl Sized) {
    crate::worker::fixture::init_city(city_root).unwrap();
    let (base_url, provider) =
        fake_openai(&["m-local"], vec![completion("the meter says 42", None)]);
    let mut worker = worker_with_provider(city_root, &base_url, "m-local").unwrap();
    worker
        .handle(wire::Command::Dispatch {
            addr: addr("lab/room2"),
            task: "measure the meter".to_owned(),
            goal: "a number is written down".to_owned(),
            model: None,
            policy: kernel::RunPolicy::of(kernel::Mode::Work),
            idem: kernel::IdemKey::derive(&RunId::CITY, kernel::Seq::FIRST, b"mother"),
            session: None,
            effort: None,
        })
        .unwrap();
    let started =
        runtime::replay::verify_ledger_dir(&kernel::layout::CityLayout::new(city_root).ledger())
            .unwrap()
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
    (worker, started, provider)
}

#[test]
fn checking_a_branch_origin_does_not_verify_the_history() {
    let dir = tempfile::tempdir().unwrap();
    let (mut worker, started, _provider) = a_mother_ran(dir.path());
    break_the_chain_after_genesis(dir.path());

    let checked = worker.origin_is_real(started);
    assert!(
        checked.is_ok(),
        "a branch check verified the history: {checked:?}"
    );
}

#[test]
fn a_branch_restores_the_mothers_shape() {
    let dir = tempfile::tempdir().unwrap();
    let (mut worker, started, _provider) = a_mother_ran(dir.path());
    let room = addr("lab/room2");
    city::write_session(
        dir.path(),
        &room,
        &city::SessionShape::new("m-other", Some(kernel::Effort::Low), None),
    )
    .unwrap();

    worker
        .open_session(&room, wire::Carry::Nothing, Some(started))
        .unwrap();

    let own = city::own_layer(dir.path(), &room).unwrap();
    assert_eq!(own.model(), Some("m-local"));
    assert_eq!(own.effort(), None, "an unstated effort stays unstated");
    assert!(own.naming().is_some());
}

#[test]
fn a_damaged_line_at_the_origin_is_refused_as_a_damaged_ledger() {
    let dir = tempfile::tempdir().unwrap();
    let (mut worker, started, _provider) = a_mother_ran(dir.path());
    worker.origin_is_real(started).unwrap();
    let segment =
        storage::ledger_segments_at(&kernel::layout::CityLayout::new(dir.path()).ledger())
            .unwrap()
            .into_iter()
            .next()
            .unwrap();
    let text = std::fs::read_to_string(&segment).unwrap();
    let damaged: Vec<String> = text
        .split('\n')
        .enumerate()
        .map(|(at, line)| {
            if u64::try_from(at).unwrap() == started.at_seq.value() {
                "x".repeat(line.len())
            } else {
                line.to_owned()
            }
        })
        .collect();
    std::fs::write(&segment, damaged.join("\n")).unwrap();

    let refused = worker.origin_is_real(started).unwrap_err();
    assert_eq!(*refused.code(), kernel::AxCode::CasCorrupt, "{refused:?}");
}
