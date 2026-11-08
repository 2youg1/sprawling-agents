// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use super::*;

fn recall(shown: &ReadShown, addr: &Address, stamp: &Stamp, want: Span) -> Recall {
    shown.recall(addr, stamp, "lab/file.txt", want)
}

fn observed(total: u64, bytes: usize, digest: Option<B3Hash>) -> Observation {
    Observation::new(total, bytes, digest)
}

fn whole(addr: Address, stamp: Stamp, observed: Observation) -> Claim {
    Claim::whole(addr, stamp, observed)
}

fn window(addr: Address, stamp: Stamp, span: Span, observed: Observation) -> Claim {
    Claim::window(addr, stamp, span, observed)
}

fn evidence(digest: Option<B3Hash>) -> (ReadShown, Address, Stamp) {
    let file = tempfile::NamedTempFile::new().unwrap();
    let stamp = Stamp::of(file.path()).unwrap().unwrap();
    let addr = Address::parse("lab/file.txt").unwrap();
    let shown = ReadShown::default();
    shown.stage(
        "call-1",
        whole(addr.clone(), stamp.clone(), observed(40, 4_000, digest)),
    );
    (shown, addr, stamp)
}

#[test]
fn only_an_admitted_unshortened_claim_can_answer_a_repeat() {
    let (shown, addr, stamp) = evidence(None);
    let want = Span {
        offset: 0,
        lines: 40,
    };

    assert_eq!(recall(&shown, &addr, &stamp, want), Recall::Fresh);
    shown.admit("call-1");
    assert_eq!(recall(&shown, &addr, &stamp, want), Recall::Fresh);
    shown.confirm(&BTreeSet::from(["call-1".to_owned()]));
    assert_eq!(recall(&shown, &addr, &stamp, want), Recall::Fresh);

    shown.stage(
        "call-2",
        whole(addr.clone(), stamp.clone(), observed(40, 4_000, None)),
    );
    shown.admit("call-2");
    shown.confirm(&BTreeSet::new());
    assert_eq!(
        recall(&shown, &addr, &stamp, want),
        Recall::Repeat(Answer {
            lines: 40,
            total_lines: 40
        })
    );
}

#[test]
fn a_racy_claim_demands_a_content_comparison() {
    let digest = B3Hash::digest(b"the complete bytes");
    let (shown, addr, stamp) = evidence(Some(digest));
    shown.admit("call-1");
    shown.confirm(&BTreeSet::new());

    assert_eq!(
        recall(
            &shown,
            &addr,
            &stamp,
            Span {
                offset: 0,
                lines: 40
            },
        ),
        Recall::Compare {
            digest,
            answer: Answer {
                lines: 40,
                total_lines: 40,
            },
        }
    );
}

#[test]
fn windows_cover_only_what_they_actually_delivered() {
    let file = tempfile::NamedTempFile::new().unwrap();
    let stamp = Stamp::of(file.path()).unwrap().unwrap();
    let addr = Address::parse("lab/file.txt").unwrap();
    let shown = ReadShown::default();
    shown.stage(
        "call-1",
        window(
            addr.clone(),
            stamp.clone(),
            Span {
                offset: 10,
                lines: 20,
            },
            observed(100, 4_000, None),
        ),
    );
    shown.admit("call-1");
    shown.confirm(&BTreeSet::new());

    assert_eq!(
        recall(
            &shown,
            &addr,
            &stamp,
            Span {
                offset: 10,
                lines: 20
            },
        ),
        Recall::Repeat(Answer {
            lines: 20,
            total_lines: 100
        })
    );
    assert_eq!(
        recall(
            &shown,
            &addr,
            &stamp,
            Span {
                offset: 0,
                lines: 100
            },
        ),
        Recall::Fresh
    );
    assert_eq!(
        recall(
            &shown,
            &addr,
            &stamp,
            Span {
                offset: 25,
                lines: 10
            },
        ),
        Recall::Fresh
    );
}

#[test]
fn a_conflicting_digest_replaces_rather_than_merges_old_evidence() {
    let (shown, addr, stamp) = evidence(Some(B3Hash::digest(b"before")));
    shown.admit("call-1");
    shown.confirm(&BTreeSet::new());
    let after = B3Hash::digest(b"after");
    shown.stage(
        "call-2",
        whole(
            addr.clone(),
            stamp.clone(),
            observed(40, 4_000, Some(after)),
        ),
    );
    shown.admit("call-2");
    shown.confirm(&BTreeSet::new());

    assert_eq!(
        recall(
            &shown,
            &addr,
            &stamp,
            Span {
                offset: 0,
                lines: 40
            },
        ),
        Recall::Compare {
            digest: after,
            answer: Answer {
                lines: 40,
                total_lines: 40,
            },
        }
    );
}

#[test]
fn confirmation_follows_admission_order_instead_of_call_names() {
    let (shown, addr, stamp) = evidence(Some(B3Hash::digest(b"old")));
    shown.admit("call-1");
    shown.confirm(&BTreeSet::new());
    for (call, text) in [("z", b"middle".as_slice()), ("a", b"new".as_slice())] {
        shown.stage(
            call,
            whole(
                addr.clone(),
                stamp.clone(),
                observed(40, 4_000, Some(B3Hash::digest(text))),
            ),
        );
        shown.admit(call);
    }
    shown.confirm(&BTreeSet::new());
    assert!(
        matches!(recall(&shown, &addr, &stamp, Span { offset: 0, lines: 40 }),
        Recall::Compare { digest, .. } if digest == B3Hash::digest(b"new"))
    );
}

#[test]
fn repeating_a_tiny_claim_never_makes_elision_profitable() {
    let (shown, addr, stamp) = evidence(None);
    shown.discard("call-1");
    for _ in 0..100 {
        shown.stage(
            "small",
            whole(addr.clone(), stamp.clone(), observed(40, 1, None)),
        );
        shown.admit("small");
        shown.confirm(&BTreeSet::new());
    }
    assert_eq!(
        recall(
            &shown,
            &addr,
            &stamp,
            Span {
                offset: 0,
                lines: 40
            }
        ),
        Recall::Fresh
    );
}

#[test]
fn discarding_an_attempt_prevents_later_admission_of_its_claim() {
    let (shown, addr, stamp) = evidence(None);
    shown.discard("call-1");
    shown.admit("call-1");
    shown.confirm(&BTreeSet::new());
    assert_eq!(
        recall(
            &shown,
            &addr,
            &stamp,
            Span {
                offset: 0,
                lines: 40
            }
        ),
        Recall::Fresh
    );
}
