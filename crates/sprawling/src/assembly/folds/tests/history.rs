// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! What a page reads back out of a history: the city, one session, and
//! where a slice that stopped early resumes.

use super::super::*;
use crate::assembly::*;

/// The live page could see nothing from before it opened, because
/// the server broadcasts and never backfills.
#[test]
fn a_page_can_ask_for_the_history_that_happened_before_it_opened() {
    let dir = tempfile::tempdir().unwrap();
    let report = init_city(dir.path()).unwrap();
    let mut worker = RunWorker::new(
        dir.path(),
        gateway::Custodian::in_memory(),
        runtime::diagnostics::Diagnostics::off(),
    )
    .unwrap();
    for n in 0..6u8 {
        worker
            .handle(channels::Command::CreateBuilding {
                addr: Address::parse(&format!("lab{n}")).unwrap(),
                template: channels::TemplateName::parse("minimal").unwrap(),
                idem: kernel::IdemKey::derive(&RunId::CITY, kernel::Seq::FIRST, &[n]),
            })
            .unwrap();
    }
    let mut views = rebuild_views(&report.ledger_dir).unwrap();

    let channels::Answer::History(tail) = views.answer(&channels::Query::History {
        before: None,
        limit: 3,
    }) else {
        panic!("the history query has an answer");
    };
    assert_eq!(tail.records.len(), 3);
    let seqs: Vec<u64> = tail.records.iter().map(|r| r.seq().value()).collect();
    let mut ascending = seqs.clone();
    ascending.sort_unstable();
    assert_eq!(seqs, ascending, "oldest first: that is the fold's order");
    let earlier = tail.earlier.expect("there is more behind this slice");

    // Paging back reaches the genesis record and then says there is
    // nothing behind it, rather than answering an empty slice
    // forever.
    let channels::Answer::History(older) = views.answer(&channels::Query::History {
        before: Some(earlier),
        limit: channels::HISTORY_MAX,
    }) else {
        panic!("the history query has an answer");
    };
    assert_eq!(
        older.records[0].seq().value(),
        kernel::Seq::FIRST.value(),
        "paging back reaches the genesis line, which is sequence zero"
    );
    assert!(
        older.earlier.is_none(),
        "the first record has nothing behind it"
    );
    assert!(
        older.records.last().map(|r| r.seq().value()) < seqs.first().copied(),
        "the two slices do not overlap"
    );
}

/// Opening a session that started before this tab did.
///
/// `Query::History` carries no run, so four sessions divided one
/// bounded slice between them and a session older than the slice was
/// not in it at all - the page for it was blank. This asks for one
/// session and gets that session.
#[test]
fn one_session_can_be_asked_for_by_itself_rather_than_filtered_out_of_the_city() {
    let dir = tempfile::tempdir().unwrap();
    let report = init_city(dir.path()).unwrap();
    let mut worker = RunWorker::new(
        dir.path(),
        gateway::Custodian::in_memory(),
        runtime::diagnostics::Diagnostics::off(),
    )
    .unwrap();
    for n in 0..6u8 {
        worker
            .handle(channels::Command::CreateBuilding {
                addr: Address::parse(&format!("lab{n}")).unwrap(),
                template: channels::TemplateName::parse("minimal").unwrap(),
                idem: kernel::IdemKey::derive(&RunId::CITY, kernel::Seq::FIRST, &[n]),
            })
            .unwrap();
    }
    let mut views = rebuild_views(&report.ledger_dir).unwrap();

    // Everything a fresh city writes belongs to the city's own run,
    // so asking for it gets those records and asking for a session
    // nobody ever opened gets none of them.
    let channels::Answer::History(mine) = views.answer(&channels::Query::RunHistory {
        run: RunId::CITY,
        before: None,
        limit: channels::HISTORY_MAX,
    }) else {
        panic!("the run history query has an answer");
    };
    assert!(!mine.records.is_empty(), "the city's own run wrote these");
    assert!(
        mine.records.iter().all(|held| held.run() == RunId::CITY),
        "a session's history holds only that session"
    );
    let seqs: Vec<u64> = mine.records.iter().map(|r| r.seq().value()).collect();
    let mut ascending = seqs.clone();
    ascending.sort_unstable();
    assert_eq!(seqs, ascending, "oldest first, as the fold expects");

    let channels::Answer::History(stranger) = views.answer(&channels::Query::RunHistory {
        run: RunId::from_bytes([3u8; 16]),
        before: None,
        limit: channels::HISTORY_MAX,
    }) else {
        panic!("the run history query has an answer");
    };
    assert!(
        stranger.records.is_empty(),
        "a session this city never ran has no history in it"
    );
}

/// Stopping on the limit has to be told apart from reaching the
/// beginning of the session, or a busy city reads as a session with
/// nothing in it.
#[test]
fn a_run_history_that_stopped_early_says_where_to_resume_rather_than_that_it_ended() {
    let dir = tempfile::tempdir().unwrap();
    let report = init_city(dir.path()).unwrap();
    let mut worker = RunWorker::new(
        dir.path(),
        gateway::Custodian::in_memory(),
        runtime::diagnostics::Diagnostics::off(),
    )
    .unwrap();
    for n in 0..6u8 {
        worker
            .handle(channels::Command::CreateBuilding {
                addr: Address::parse(&format!("lab{n}")).unwrap(),
                template: channels::TemplateName::parse("minimal").unwrap(),
                idem: kernel::IdemKey::derive(&RunId::CITY, kernel::Seq::FIRST, &[n]),
            })
            .unwrap();
    }
    let mut views = rebuild_views(&report.ledger_dir).unwrap();

    // One record at a time, so the walk stops on the limit well
    // before it reaches the genesis line.
    let channels::Answer::History(page) = views.answer(&channels::Query::RunHistory {
        run: RunId::CITY,
        before: None,
        limit: 1,
    }) else {
        panic!("the run history query has an answer");
    };
    assert_eq!(page.records.len(), 1);
    let resume = page
        .earlier
        .expect("stopping on the limit is not reaching the beginning");
    assert!(
        resume.value() <= page.records[0].seq().value(),
        "resuming must not skip the records between"
    );

    // And paging with it does reach the beginning, which is the
    // other statement `earlier` has to be able to make.
    let mut before = Some(resume);
    let mut guard = 0;
    while let Some(at) = before {
        let channels::Answer::History(page) = views.answer(&channels::Query::RunHistory {
            run: RunId::CITY,
            before: Some(at),
            limit: 1,
        }) else {
            panic!("the run history query has an answer");
        };
        before = page.earlier;
        guard += 1;
        assert!(guard < 1_000, "paging back does not terminate");
    }
}
