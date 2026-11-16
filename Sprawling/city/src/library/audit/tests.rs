// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use kernel::event::record::AuditVerdict;
use kernel::{B3Hash, Seq};

use super::{AuditState, audit_state};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Event {
    Install(u8),
    Refused,
    Edit(u8),
    Audit(u8, AuditVerdict),
    FetchFailed(u8),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Shown {
    Absent,
    Unaudited,
    Stale,
    Audited(AuditVerdict),
}

fn digest(byte: u8) -> B3Hash {
    B3Hash::digest(&[byte])
}

fn replay(trace: &[Event]) -> (Option<B3Hash>, Vec<(B3Hash, AuditVerdict, Seq)>) {
    let mut content = None;
    let mut lines = Vec::new();
    for (index, event) in trace.iter().enumerate() {
        let at = Seq::new(u64::try_from(index).unwrap());
        match *event {
            Event::Install(d) | Event::Edit(d) => content = Some(digest(d)),
            Event::Refused => {}
            Event::Audit(d, verdict) => lines.push((digest(d), verdict, at)),
            Event::FetchFailed(d) => lines.push((digest(d), AuditVerdict::Unreachable, at)),
        }
    }
    (content, lines)
}

fn shown(trace: &[Event]) -> Shown {
    let (content, lines) = replay(trace);
    match content {
        None => Shown::Absent,
        Some(content) => match audit_state(&content, &lines) {
            AuditState::Unaudited => Shown::Unaudited,
            AuditState::Stale { .. } => Shown::Stale,
            AuditState::Audited { verdict, .. } => Shown::Audited(verdict),
        },
    }
}

use AuditVerdict::{Fail, Pass, Warn};
use Event::{Audit, Edit, FetchFailed, Install, Refused};

const VECTORS: &[(&[Event], Shown)] = &[
    (&[], Shown::Absent),
    (&[Refused], Shown::Absent),
    (&[Install(1)], Shown::Unaudited),
    (&[Install(1), Audit(1, Pass)], Shown::Audited(Pass)),
    (
        &[Install(1), Audit(1, Pass), Audit(1, Fail)],
        Shown::Audited(Fail),
    ),
    (&[Install(1), Audit(1, Warn), Edit(2)], Shown::Stale),
    (
        &[Install(1), Audit(1, Pass), Edit(2), Audit(2, Fail)],
        Shown::Audited(Fail),
    ),

    (&[Install(1), Edit(2), Audit(1, Pass)], Shown::Stale),

    (
        &[Install(1), Audit(1, Pass), Edit(2), Edit(1)],
        Shown::Audited(Pass),
    ),
    (&[Install(1), FetchFailed(1)], Shown::Unaudited),
    (
        &[Install(1), Audit(1, Pass), FetchFailed(1)],
        Shown::Audited(Pass),
    ),
    (
        &[FetchFailed(1), FetchFailed(1), Install(1)],
        Shown::Unaudited,
    ),
    (&[Audit(3, Fail), Install(1)], Shown::Stale),
    (
        &[Install(1), Refused, Audit(1, Warn), Refused],
        Shown::Audited(Warn),
    ),
];

#[test]
fn every_trace_vector_reads_as_the_model_shows_it() {
    let read: Vec<(&[Event], Shown)> = VECTORS.iter().map(|(t, _)| (*t, shown(t))).collect();
    assert_eq!(read, VECTORS.to_vec());
}

#[test]
fn shown_audited_only_for_an_audit_of_the_content_now() {
    for (trace, _) in VECTORS {
        if let Shown::Audited(verdict) = shown(trace) {
            let (content, _) = replay(trace);
            assert!(
                trace.iter().any(
                    |e| matches!(*e, Audit(d, v) if Some(digest(d)) == content && v == verdict)
                ),
                "{trace:?}"
            );
        }
    }
}

#[test]
fn content_changed_to_unaudited_bytes_is_never_shown_audited() {
    let afters: [&[Event]; 4] = [
        &[],
        &[Refused, FetchFailed(9)],
        &[Audit(1, Pass)],
        &[FetchFailed(9), Audit(2, Fail), Refused],
    ];
    for (before, _) in VECTORS {
        for after in afters {
            let trace = [*before, &[Edit(9)], after].concat();
            assert!(!matches!(shown(&trace), Shown::Audited(_)), "{trace:?}");
        }
    }
}

#[test]
fn failed_fetches_change_nothing() {
    for (trace, _) in VECTORS {
        let kept: Vec<Event> = trace
            .iter()
            .copied()
            .filter(|e| !matches!(e, FetchFailed(_)))
            .collect();
        assert_eq!(shown(&kept), shown(trace), "{trace:?}");
    }
}

#[test]
fn a_failed_fetch_never_blocks_an_install() {
    for (before, _) in VECTORS {
        let trace = [*before, &[FetchFailed(5), Install(5)]].concat();
        assert_eq!(replay(&trace).0, Some(digest(5)), "{trace:?}");
        assert_ne!(shown(&trace), Shown::Absent, "{trace:?}");
    }
}

mod execution;
