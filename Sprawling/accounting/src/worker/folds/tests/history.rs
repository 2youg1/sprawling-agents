// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use crate::worker::*;

#[test]
fn a_page_can_ask_for_the_history_that_happened_before_it_opened() {
    let dir = tempfile::tempdir().unwrap();
    let report = crate::worker::fixture::init_city(dir.path()).unwrap();
    let mut worker = RunWorker::new(
        dir.path(),
        runtime::diagnostics::Diagnostics::off(),
        crate::worker::fixture::hands(),
    )
    .unwrap();
    for n in 0..6u8 {
        worker
            .handle(wire::Command::CreateBuilding {
                addr: Address::parse(&format!("lab{n}")).unwrap(),
                template: wire::TemplateName::parse("minimal").unwrap(),
                rules: None,
                source: None,
                idem: kernel::IdemKey::derive(&RunId::CITY, kernel::Seq::FIRST, &[n]),
            })
            .unwrap();
    }
    let views = crate::views::Views::rebuild(&report.ledger_dir).unwrap();

    let wire::Answer::History(tail) = views
        .prepare(&wire::Query::History {
            before: None,
            limit: 3,
        })
        .finish()
    else {
        panic!("the history query has an answer");
    };
    assert_eq!(tail.records.len(), 3);
    let seqs: Vec<u64> = tail.records.iter().map(|r| r.seq().value()).collect();
    let mut ascending = seqs.clone();
    ascending.sort_unstable();
    assert_eq!(seqs, ascending, "oldest first: that is the fold's order");
    let earlier = tail.earlier.expect("there is more behind this slice");

    let wire::Answer::History(older) = views
        .prepare(&wire::Query::History {
            before: Some(earlier),
            limit: wire::HISTORY_MAX,
        })
        .finish()
    else {
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

#[test]
fn one_session_can_be_asked_for_by_itself_rather_than_filtered_out_of_the_city() {
    let dir = tempfile::tempdir().unwrap();
    let report = crate::worker::fixture::init_city(dir.path()).unwrap();
    let mut worker = RunWorker::new(
        dir.path(),
        runtime::diagnostics::Diagnostics::off(),
        crate::worker::fixture::hands(),
    )
    .unwrap();
    for n in 0..6u8 {
        worker
            .handle(wire::Command::CreateBuilding {
                addr: Address::parse(&format!("lab{n}")).unwrap(),
                template: wire::TemplateName::parse("minimal").unwrap(),
                rules: None,
                source: None,
                idem: kernel::IdemKey::derive(&RunId::CITY, kernel::Seq::FIRST, &[n]),
            })
            .unwrap();
    }
    let views = crate::views::Views::rebuild(&report.ledger_dir).unwrap();

    let wire::Answer::History(mine) = views
        .prepare(&wire::Query::RunHistory {
            run: RunId::CITY,
            before: None,
            limit: wire::HISTORY_MAX,
        })
        .finish()
    else {
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

    let wire::Answer::History(stranger) = views
        .prepare(&wire::Query::RunHistory {
            run: RunId::from_bytes([3u8; 16]),
            before: None,
            limit: wire::HISTORY_MAX,
        })
        .finish()
    else {
        panic!("the run history query has an answer");
    };
    assert!(
        stranger.records.is_empty(),
        "a session this city never ran has no history in it"
    );
}

#[test]
fn a_run_history_that_stopped_early_says_where_to_resume_rather_than_that_it_ended() {
    let dir = tempfile::tempdir().unwrap();
    let report = crate::worker::fixture::init_city(dir.path()).unwrap();
    let mut worker = RunWorker::new(
        dir.path(),
        runtime::diagnostics::Diagnostics::off(),
        crate::worker::fixture::hands(),
    )
    .unwrap();
    for n in 0..6u8 {
        worker
            .handle(wire::Command::CreateBuilding {
                addr: Address::parse(&format!("lab{n}")).unwrap(),
                template: wire::TemplateName::parse("minimal").unwrap(),
                rules: None,
                source: None,
                idem: kernel::IdemKey::derive(&RunId::CITY, kernel::Seq::FIRST, &[n]),
            })
            .unwrap();
    }
    let views = crate::views::Views::rebuild(&report.ledger_dir).unwrap();

    let wire::Answer::History(page) = views
        .prepare(&wire::Query::RunHistory {
            run: RunId::CITY,
            before: None,
            limit: 1,
        })
        .finish()
    else {
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

    let mut before = Some(resume);
    let mut guard = 0;
    while let Some(at) = before {
        let wire::Answer::History(page) = views
            .prepare(&wire::Query::RunHistory {
                run: RunId::CITY,
                before: Some(at),
                limit: 1,
            })
            .finish()
        else {
            panic!("the run history query has an answer");
        };
        before = page.earlier;
        guard += 1;
        assert!(guard < 1_000, "paging back does not terminate");
    }
}
