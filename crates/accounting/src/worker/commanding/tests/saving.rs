// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! A page's save and a person's decision on proposal cards, through the
//! worker's own doors (`crates/wire/Spec.lean` §8-72, 8-73; `crates/accounting/spec/Worker/Commanding/Saving.lean` §8-22).

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::string_slice,
    reason = "test code"
)]

use kernel::B3Hash;
use kernel::event::record::{ProposalOffered, ProposalWithdrawn, SliceVerdict, Verdict};

use crate::worker::Posted;
use crate::worker::*;

const NOTES: &str = "hall/notes.md";

fn notes() -> Address {
    Address::parse(NOTES).unwrap()
}

fn key(name: &[u8]) -> kernel::IdemKey {
    kernel::IdemKey::derive(&RunId::CITY, kernel::Seq::FIRST, name)
}

fn city(text: &str) -> (tempfile::TempDir, RunWorker) {
    let dir = tempfile::tempdir().unwrap();
    crate::worker::fixture::init_city(dir.path()).unwrap();
    std::fs::write(dir.path().join(NOTES), text).unwrap();
    let worker = reopen(dir.path());
    (dir, worker)
}

fn reopen(root: &Path) -> RunWorker {
    RunWorker::new(
        root,
        runtime::diagnostics::Diagnostics::off(),
        crate::worker::fixture::hands(),
    )
    .unwrap()
}

fn on_disk(root: &Path) -> String {
    std::fs::read_to_string(root.join(NOTES)).unwrap()
}

/// The payloads of every line of one kind, in the order written.
fn lines(root: &Path, kind: &str) -> Vec<serde_json::Value> {
    let ledger = kernel::layout::CityLayout::new(root).ledger();
    runtime::replay::verify_ledger_dir(&ledger)
        .unwrap()
        .raw_lines()
        .iter()
        .map(|line| serde_json::from_slice::<serde_json::Value>(line).unwrap())
        .filter(|value| value["kind"] == kind)
        .map(|value| value["data"].clone())
        .collect()
}

/// The stretch and its new text travel together: one edit.
fn put_range(baseline: &str, edit: (u64, u64, &str), idem: &[u8]) -> wire::Command {
    let (start, end, text) = edit;
    wire::Command::PutRange(wire::RangeWrite {
        doc: notes(),
        baseline: B3Hash::digest(baseline.as_bytes()),
        edits: vec![documents::TextEdit {
            span: documents::Span::new(start, end).unwrap(),
            text: text.to_owned(),
        }],
        idem: key(idem),
    })
}

/// Offers a proposal about `source[start..end]` as run `run` would.
/// The stretch and the suggestion travel together: one change to one
/// version.
fn offer(worker: &mut RunWorker, run: u8, source: &str, change: (usize, usize, &str)) -> B3Hash {
    let (start, end, after) = change;
    let offered = ProposalOffered {
        doc: notes(),
        baseline: B3Hash::digest(source.as_bytes()),
        start: u64::try_from(start).unwrap(),
        end: u64::try_from(end).unwrap(),
        before: source[start..end].to_owned(),
        after: after.to_owned(),
    };
    let run = RunId::from_bytes([run; 16]);
    let id = documents::Offer::of(run, &offered).unwrap().id();
    worker
        .record_for(
            run,
            crate::effect::Line {
                who: "resident".to_owned(),
                addr: Address::parse("hall/mayor").unwrap(),
                kind: EventKind::ProposalOffered,
                data: Payload::of(&offered).unwrap(),
            },
        )
        .unwrap();
    id
}

fn decide(cards: &[(B3Hash, Vec<SliceVerdict>)], idem: &[u8]) -> wire::Command {
    wire::Command::DecideProposals(wire::ProposalDecisions {
        doc: notes(),
        decisions: cards
            .iter()
            .map(|(proposal, verdicts)| wire::ProposalDecision {
                proposal: *proposal,
                verdicts: verdicts.clone(),
            })
            .collect(),
        idem: key(idem),
    })
}

fn accept(slice: u32) -> SliceVerdict {
    SliceVerdict {
        slice,
        verdict: Verdict::Accept,
    }
}

fn open_cards(root: &Path) -> (Option<B3Hash>, Vec<B3Hash>) {
    let wire::Answer::Proposals(answer) =
        crate::views::ask(root, &wire::Query::Proposals(notes())).unwrap()
    else {
        panic!("the open cards of one document");
    };
    (
        answer.version,
        answer.open.iter().map(|card| card.id).collect(),
    )
}

fn code(refused: &AxError) -> &'static str {
    refused.code().as_str()
}

/// A3 on the wire: two saves made on one version, the first lands and
/// is recorded with the version it left, the second is told the version
/// moved and the first one's bytes stand.
#[test]
fn a_second_save_from_the_same_version_is_refused_and_the_first_stands() {
    let (dir, mut worker) = city("one two\n");
    worker
        .handle(put_range("one two\n", (0, 3, "ONE"), b"first"))
        .unwrap();
    let late = worker
        .handle(put_range("one two\n", (4, 7, "TWO"), b"second"))
        .unwrap_err();
    assert_eq!(
        (code(&late), on_disk(dir.path())),
        ("E_VERSION_CONFLICT", "ONE two\n".to_owned())
    );
    let written = lines(dir.path(), "document_written");
    assert_eq!(
        written,
        vec![serde_json::json!({
            "at": NOTES,
            "baseline": B3Hash::digest(b"one two\n").to_string(),
            "version": B3Hash::digest(b"ONE two\n").to_string(),
            "bytes": 8,
        })]
    );
}

/// The receipt a page waits for carries the key it sent, and the same
/// frame sent again is answered without a second save.
#[test]
fn a_save_sent_twice_under_one_key_lands_once_and_its_receipt_names_the_key() {
    let (dir, mut worker) = city("one\n");
    for _ in 0..2 {
        worker.serve_one(Posted {
            command: put_range("one\n", (0, 3, "uno"), b"once"),
            reply: wire::Reply::nowhere(),
        });
    }
    let written = lines(dir.path(), "document_written");
    assert_eq!(written.len(), 1);
    assert_eq!(written[0]["idem"], key(b"once").to_string());
    assert_eq!(on_disk(dir.path()), "uno\n");
}

/// The files that govern the city and its buildings have doors that
/// judge what they are given; a byte-range save does not reach them.
#[test]
fn a_save_inside_the_reserved_subtree_is_refused() {
    let (_dir, mut worker) = city("x");
    for doc in [".sprawling/CONFIG.toml", "hall/.sprawling/RULES.toml"] {
        let refused = worker
            .handle(wire::Command::PutRange(wire::RangeWrite {
                doc: Address::parse(doc).unwrap(),
                baseline: B3Hash::digest(b""),
                edits: Vec::new(),
                idem: key(doc.as_bytes()),
            }))
            .unwrap_err();
        assert_eq!(code(&refused), "E_OUTSIDE_WRITE_DOMAIN", "{doc}");
    }
}

/// A10: three cards on one version, read back by a city reopened from
/// its history, decided at once - one amended, one rejected, one
/// accepted - land as one save, and none of them stays open.
#[test]
fn cards_accepted_amended_and_rejected_land_as_one_save_after_a_reopen() {
    let source = "One. Two. Three.\n";
    let (dir, mut worker) = city(source);
    let two = offer(&mut worker, 1, source, (5, 9, "Deux."));
    let three = offer(&mut worker, 1, source, (10, 16, "Trois."));
    let one = offer(&mut worker, 2, source, (0, 4, "Uno."));
    drop(worker);
    assert_eq!(
        open_cards(dir.path()),
        (
            Some(B3Hash::digest(source.as_bytes())),
            vec![two, three, one]
        )
    );

    let mut worker = reopen(dir.path());
    let amended = SliceVerdict {
        slice: 1,
        verdict: Verdict::Amend {
            text: "Zwei.".to_owned(),
        },
    };
    worker
        .handle(decide(
            &[
                (two, vec![accept(0), amended]),
                (three, Vec::new()),
                (one, vec![accept(0), accept(1)]),
            ],
            b"decide",
        ))
        .unwrap();
    assert_eq!(on_disk(dir.path()), "Uno. Zwei. Three.\n");
    assert_eq!(lines(dir.path(), "document_written").len(), 1);
    assert_eq!(lines(dir.path(), "proposal_decided").len(), 3);
    assert_eq!(open_cards(dir.path()).1, Vec::<B3Hash>::new());
}

/// A card is handled once: a withdrawn card and a decided one are
/// refused when decided again, and a repeat under the key of the first
/// decision is that decision, not a second one.
#[test]
fn a_withdrawn_or_decided_card_is_not_decided_again() {
    let source = "One. Two.\n";
    let (dir, mut worker) = city(source);
    let kept = offer(&mut worker, 1, source, (5, 9, "Deux."));
    let taken_back = offer(&mut worker, 1, source, (0, 4, "Uno."));
    worker
        .record_for(
            RunId::from_bytes([1; 16]),
            crate::effect::Line {
                who: "resident".to_owned(),
                addr: Address::parse("hall/mayor").unwrap(),
                kind: EventKind::ProposalWithdrawn,
                data: Payload::of(&ProposalWithdrawn {
                    proposal: taken_back,
                })
                .unwrap(),
            },
        )
        .unwrap();
    let withdrawn = worker
        .handle(decide(&[(taken_back, vec![accept(0), accept(1)])], b"late"))
        .unwrap_err();
    assert_eq!(code(&withdrawn), "E_INVALID_ARGS");

    for _ in 0..2 {
        worker.serve_one(Posted {
            command: decide(&[(kept, vec![accept(0), accept(1)])], b"once"),
            reply: wire::Reply::nowhere(),
        });
    }
    assert_eq!(lines(dir.path(), "proposal_decided").len(), 1);
    let again = worker
        .handle(decide(&[(kept, Vec::new())], b"again"))
        .unwrap_err();
    assert_eq!(
        (code(&again), on_disk(dir.path())),
        ("E_INVALID_ARGS", "One. Deux.\n".to_owned())
    );
}

/// A10: a card made on a version that has since moved is refused when
/// it would write, and stays open; rejecting it writes nothing and is
/// not refused.
#[test]
fn a_stale_card_is_refused_and_rejecting_it_is_not() {
    let source = "One. Two.\n";
    let (dir, mut worker) = city(source);
    let card = offer(&mut worker, 1, source, (5, 9, "Deux."));
    worker
        .handle(put_range(source, (0, 4, "Uno."), b"moved"))
        .unwrap();
    let stale = worker
        .handle(decide(&[(card, vec![accept(0), accept(1)])], b"stale"))
        .unwrap_err();
    assert_eq!(
        (code(&stale), on_disk(dir.path()), open_cards(dir.path()).1),
        ("E_VERSION_CONFLICT", "Uno. Two.\n".to_owned(), vec![card])
    );
    worker
        .handle(decide(&[(card, Vec::new())], b"reject"))
        .unwrap();
    assert_eq!(
        (
            lines(dir.path(), "document_written").len(),
            open_cards(dir.path()).1
        ),
        (1, Vec::<B3Hash>::new())
    );
}
