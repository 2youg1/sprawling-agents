// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use super::*;
use channels::{B3Hash, EventDraft, Payload, RunId};

fn record(seq: u64, at: u64, kind: EventKind, who: &str) -> EventRecord {
    EventRecord::from_draft(
        EventDraft {
            run: RunId::from_bytes([1u8; 16]),
            t: TimeMs::new(at),
            who: who.to_owned(),
            addr: None,
            kind,
            data: Payload::empty(),
            ig: false,
        },
        Seq::new(seq),
        B3Hash::digest(b"prev"),
    )
}

fn stream() -> Vec<EventRecord> {
    vec![
        record(1, 10, EventKind::RunStarted, "alice"),
        record(2, 20, EventKind::ToolCalled, "alice"),
        record(3, 30, EventKind::ToolResult, "bob"),
        record(4, 40, EventKind::RunFrozen, "bob"),
    ]
}

#[test]
fn an_empty_filter_shows_everything() {
    let page = page(stream().iter(), &Filter::default(), 100);
    assert_eq!(page.rows.len(), 4);
    assert_eq!(page.filtered_out, 0);
}

#[test]
fn a_filtered_page_says_how_much_it_passed_over() {
    // A window that silently omits is worse than no window.
    let filter = Filter {
        kinds: vec![EventKind::ToolResult],
        ..Filter::default()
    };
    let page = page(stream().iter(), &filter, 100);
    assert_eq!(page.rows.len(), 1);
    assert_eq!(page.filtered_out, 3);
}

#[test]
fn time_bounds_are_inclusive_at_both_ends() {
    let filter = Filter {
        since: Some(TimeMs::new(20)),
        until: Some(TimeMs::new(30)),
        ..Filter::default()
    };
    let page = page(stream().iter(), &filter, 100);
    let seqs: Vec<u64> = page.rows.iter().map(|r| r.seq.value()).collect();
    assert_eq!(seqs, [2, 3]);
}

#[test]
fn a_page_stops_at_its_limit_and_says_where_to_resume() {
    let page = page(stream().iter(), &Filter::default(), 2);
    assert_eq!(page.rows.len(), 2);
    assert_eq!(page.next_from, Some(Seq::new(3)));
}

#[test]
fn the_last_page_has_no_next() {
    let page = page(stream().iter(), &Filter::default(), 4);
    assert_eq!(page.next_from, None);
}

#[test]
fn an_export_labels_itself_as_a_view() {
    let exported = export(&page(stream().iter(), &Filter::default(), 100));
    assert!(
        exported.starts_with("# sprawling ledger view"),
        "an export must not be mistaken for the Ledger"
    );
    assert!(exported.contains("not the Ledger"));
    assert_eq!(exported.lines().count(), 6, "header, columns, four rows");
}

#[test]
fn filtering_by_actor_is_a_substring_because_names_carry_prefixes() {
    let filter = Filter {
        actor: "ali".to_owned(),
        ..Filter::default()
    };
    assert_eq!(page(stream().iter(), &filter, 100).rows.len(), 2);
}
