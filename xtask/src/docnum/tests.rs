// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use super::*;

fn values() -> Readings {
    [
        ("wire_v".to_owned(), "31".to_owned()),
        ("gate_count".to_owned(), "20".to_owned()),
    ]
    .into_iter()
    .collect()
}

fn root() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("the xtask manifest directory has a parent")
        .to_path_buf()
}

#[test]
fn a_span_on_its_own_lines_keeps_its_value_on_its_own_line() {
    let text = "before\n<!-- xtask:begin wire_v -->\n15\n<!-- xtask:end -->\nafter\n";
    let found = spans(text).expect("the markers are well formed");
    assert_eq!(found.len(), 1);
    let span = &found[0];
    assert_eq!(span.key, "wire_v");
    assert_eq!(span.line, 2);
    assert_eq!(span.content, "\n15\n");
    assert_eq!(
        rewrite(text, &found, &values(), "doc.md").unwrap(),
        "before\n<!-- xtask:begin wire_v -->\n31\n<!-- xtask:end -->\nafter\n"
    );
}

#[test]
fn a_span_inside_one_line_stays_inside_that_line() {
    let text = "| `Command` | <!-- xtask:begin gate_count -->9<!-- xtask:end --> | what |\n";
    let found = spans(text).expect("the markers are well formed");
    assert_eq!(found[0].content, "9");
    assert_eq!(
        rewrite(text, &found, &values(), "doc.md").unwrap(),
        "| `Command` | <!-- xtask:begin gate_count -->20<!-- xtask:end --> | what |\n"
    );
}

#[test]
fn a_span_already_holding_todays_reading_is_left_alone() {
    let text = "the wire is at <!-- xtask:begin wire_v -->31<!-- xtask:end -->.\n";
    let found = spans(text).expect("the markers are well formed");
    let mut out = Vec::new();
    judge("doc.md", &found[0], &values(), &mut out);
    assert!(out.is_empty(), "{out:?}");
    assert_eq!(rewrite(text, &found, &values(), "doc.md").unwrap(), text);
}

#[test]
fn a_stale_span_is_refused_with_the_command_that_repairs_it() {
    let text = "the wire is at <!-- xtask:begin wire_v -->15<!-- xtask:end -->.\n";
    let found = spans(text).expect("the markers are well formed");
    let mut out = Vec::new();
    judge("ARCHITECTURE.md", &found[0], &values(), &mut out);
    assert_eq!(out.len(), 1);
    assert_eq!(out[0].location, "ARCHITECTURE.md:1");
    assert!(out[0].violation.contains("channels::WIRE_V says `31`"));
    assert_eq!(
        out[0].alternative,
        "run `cargo xtask docnum --write` and commit the result"
    );
}

#[test]
fn a_span_naming_no_fact_is_refused_and_never_written() {
    let text = "<!-- xtask:begin invented -->7<!-- xtask:end -->\n";
    let found = spans(text).expect("the markers are well formed");
    let mut out = Vec::new();
    judge("doc.md", &found[0], &values(), &mut out);
    assert_eq!(out.len(), 1);
    assert!(out[0].violation.contains("invented"));
    assert!(out[0].alternative.contains("wire_v"));
    // A writer that skipped it would leave the document looking
    // regenerated with one stale number still in it.
    assert!(rewrite(text, &found, &values(), "doc.md").is_err());
}

#[test]
fn an_unclosed_or_nested_marker_is_reported_rather_than_read() {
    let open = spans("<!-- xtask:begin wire_v -->31\n").expect_err("no end marker");
    assert!(open.why.contains("xtask:end"));
    let nested = spans(
        "<!-- xtask:begin wire_v --><!-- xtask:begin gate_count -->1<!-- xtask:end -->\
         <!-- xtask:end -->",
    )
    .expect_err("a span inside a span");
    assert!(nested.why.contains("second span"));
    let stray = spans("text <!-- xtask:end --> more\n").expect_err("an end with no begin");
    assert!(stray.why.contains("closes nothing"));
}

#[test]
fn a_reading_is_taken_once_however_many_spans_quote_it() {
    let text = "<!-- xtask:begin wire_v -->1<!-- xtask:end -->\n\
                <!-- xtask:begin wire_v -->2<!-- xtask:end -->\n";
    let found = spans(text).expect("the markers are well formed");
    let mut taken = Readings::new();
    readings(&root(), &found, &mut taken).expect("wire_v recounts");
    assert_eq!(taken.len(), 1);
    assert_eq!(
        taken.get("wire_v").map(String::as_str),
        Some(channels::WIRE_V.to_string().as_str())
    );
}

#[test]
fn a_span_naming_no_fact_leaves_the_map_without_it() {
    let text = "<!-- xtask:begin invented -->7<!-- xtask:end -->\n";
    let found = spans(text).expect("the markers are well formed");
    let mut taken = Readings::new();
    readings(&root(), &found, &mut taken).expect("an unknown key is not an error here");
    assert!(taken.is_empty());
}

#[test]
fn the_documents_of_this_tree_hold_no_span_this_gate_cannot_read() {
    let root = root();
    for file in documents(&root).expect("the tree walks") {
        let rel = walk::rel(&root, &file);
        let text = walk::read_text(&file).expect("markdown reads");
        if !text.contains(BEGIN) && !text.contains(END) {
            continue;
        }
        assert!(spans(&text).is_ok(), "{rel} has a malformed managed span");
    }
}
