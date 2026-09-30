// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! A branch's origin checked by the one line it names: never by a
//! verify of the history, and refused as a damaged ledger when that line
//! is no record at all.

use super::addr;
use crate::assembly::fixture::*;
use crate::assembly::*;

/// A worker over a fresh city whose one run, the mother, has landed in
/// `lab/room2`, and the origin a branch of it would name.
fn a_mother_ran(city_root: &std::path::Path) -> (RunWorker, kernel::Origin, impl Sized) {
    init_city(city_root).unwrap();
    let (base_url, provider) =
        fake_openai(&["m-local"], vec![completion("the meter says 42", None)]);
    let mut worker = worker_with_provider(city_root, &base_url, "m-local").unwrap();
    worker
        .handle(wire::Command::Dispatch {
            addr: addr("lab/room2"),
            task: "measure the meter".to_owned(),
            goal: "a number is written down".to_owned(),
            model: None,
            mode: kernel::Mode::PlanGoal,
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

/// Checking a branch's origin reads the line it names, not the history:
/// the history was verified when the city opened, and a check that
/// verified it again would cost the whole ledger on every branch.
///
/// A chain broken after genesis is the witness: a verify refuses it, and
/// the check, which never reads that line, answers as it would on an
/// intact ledger.
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

/// A line at the origin that is no record at all is a damaged ledger,
/// not a line of some other run, and the refusal says so with the code
/// that sends the person to the ledger rather than to the branch.
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
