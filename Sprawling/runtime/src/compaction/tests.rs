// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    reason = "test code"
)]

use super::*;

/// Both halves of one caller's work: decide, then carry it out. The
/// invariants below hold of the pair, which is how a caller uses
/// them, and a text this module declines to shorten comes back
/// whole.
fn compacted(text: &str, budget: ByteLen) -> Cut {
    let size = ByteLen::new(u64::try_from(text.len()).unwrap_or(u64::MAX));
    match plan(detect(text), size, budget) {
        Shrink::Keep | Shrink::MustOffload => Cut::whole(text),
        Shrink::Cut(strategy) => shorten(text, strategy, budget),
    }
}

#[test]
fn the_same_input_is_dispatched_the_same_way_twice() {
    let samples = [
        "diff --git a/x b/x\n@@ -1 +1 @@\n-a\n+b\n",
        "{\"a\":1}",
        "| head | row |\n| --- | --- |\n| a | b |\n",
        "12:00:01 INFO started\n12:00:02 INFO finished\n",
        "fn main() {\n    let x = 1;\n}\n",
        "This is an ordinary sentence with more than five words in it.",
        "??",
    ];
    for sample in samples {
        assert_eq!(detect(sample), detect(sample));
    }
    assert_eq!(detect(samples[0]), Content::Diff);
    assert_eq!(detect(samples[1]), Content::Structured);
    assert_eq!(detect(samples[2]), Content::Table);
    assert_eq!(detect(samples[3]), Content::Log);
    assert_eq!(detect(samples[4]), Content::Code);
    assert_eq!(detect(samples[5]), Content::Prose);
    assert_eq!(detect(samples[6]), Content::Unknown);
}

#[test]
fn shortening_never_lengthens() {
    let inputs = [
        String::new(),
        "a".to_owned(),
        "short".to_owned(),
        "x".repeat(4_000),
        "12:00:01 INFO a\n".repeat(300),
        "fn f() {\n  body();\n}\n".repeat(300),
        "{\"deep\":[1,2,3]}".repeat(300),
        "字".repeat(1_000),
        "# One\n\n## Two\n\n".repeat(200),
    ];
    for budget in [0u64, 1, 8, 64, 512, 4_096] {
        for input in &inputs {
            let cut = compacted(input, ByteLen::new(budget));
            assert!(
                cut.text.len() <= input.len(),
                "budget {budget} grew an input of {} to {}",
                input.len(),
                cut.text.len()
            );
        }
    }
}

#[test]
fn a_cut_lands_on_a_character_boundary() {
    // Every character here is three bytes, so a byte budget lands
    // mid-character unless the cut is moved.
    let text = format!("fn f() {{\n{}\n}}\n", "字".repeat(400));
    for budget in [7u64, 11, 100, 301] {
        let cut = compacted(&text, ByteLen::new(budget));
        assert!(
            std::str::from_utf8(cut.text.as_bytes()).is_ok(),
            "a cut mid-character produces bytes nothing downstream can read"
        );
    }
    let document = format!("# 宇\n\n## 宇\n\n{}\n", "宇".repeat(400));
    for budget in [7u64, 11, 100, 301] {
        let cut = compacted(&document, ByteLen::new(budget));
        assert!(
            std::str::from_utf8(cut.text.as_bytes()).is_ok(),
            "a section cut lands on line boundaries: {}",
            cut.text
        );
    }
}

#[test]
fn what_a_cut_reports_losing_is_what_the_reader_lost() {
    let log = (0..400)
        .map(|n| format!("12:00:{n:02} INFO line {n}\n"))
        .collect::<String>();
    let cut = compacted(&log, ByteLen::new(200));
    assert_eq!(cut.place, Elided::Head, "a log keeps its end");
    let marker = elision::marker(cut.dropped);
    let carried = cut.text.len().saturating_sub(marker.len());
    let dropped = usize::try_from(cut.dropped.get()).unwrap();
    assert_eq!(
        carried + dropped,
        log.len(),
        "the reported loss plus the surviving bytes is the input: {}",
        cut.text
    );
}

#[test]
fn a_log_keeps_its_end_and_prose_keeps_its_start() {
    let log = (0..400)
        .map(|n| format!("12:00:{n:02} INFO line {n}\n"))
        .collect::<String>();
    let cut = compacted(&log, ByteLen::new(200));
    assert_eq!(cut.place, Elided::Head);
    assert!(
        cut.text.contains("line 399"),
        "the news is at the end of a log"
    );

    let prose = "The first sentence says what this is about. ".repeat(200);
    let cut = compacted(&prose, ByteLen::new(200));
    assert_eq!(cut.place, Elided::Tail);
    assert!(cut.text.starts_with("The first sentence"));
}

#[test]
fn a_document_is_markup_even_when_it_opens_with_a_table() {
    let latex = "\\documentclass{article}\n\\begin{document}\n| a | b |\n| c | d |\n\\section{One}\ntext\n\\section{Two}\nmore\n";
    assert_eq!(detect(latex), Content::Markup);
    let markdown =
        "# Title\n\nintro words here\n\n## Part\n\nbody words\n\n## Other\n\nmore words\n";
    assert_eq!(detect(markdown), Content::Markup);
    // A source file's `# ` comments share the heading shape, and the
    // code beside them outranks the skeleton.
    let python = "# note one\n# note two\ndef f():\n    pass\n";
    assert_eq!(detect(python), Content::Code);
}

#[test]
fn a_markup_plan_asks_for_sections() {
    let markup = "# A\n\n## B\n\nbody\n";
    let size = ByteLen::new(u64::try_from(markup.len()).unwrap());
    assert_eq!(detect(markup), Content::Markup);
    assert_eq!(
        plan(Content::Markup, size, ByteLen::new(4)),
        Shrink::Cut(Strategy::Sections)
    );
}

#[test]
fn a_long_document_is_cut_at_sections_and_keeps_its_headings() {
    let mut doc = String::from("# One\n\nintro\n\n");
    for n in 0..200 {
        doc.push_str(&format!(
            "## Section {n}\n\n{}\n\n",
            "body words here. ".repeat(6)
        ));
    }
    let cut = compacted(&doc, ByteLen::new(600));
    assert_eq!(cut.place, Elided::Middle, "{}", cut.text);
    assert!(cut.text.len() <= doc.len(), "never longer than the input");
    assert!(
        cut.text.matches("## Section ").count() >= 5,
        "the headings of sections whose bodies were dropped survive: {}",
        cut.text
    );
    let marker = elision::gap_marker(cut.dropped);
    let carried = cut.text.len().saturating_sub(marker.len());
    let dropped = usize::try_from(cut.dropped.get()).unwrap();
    assert_eq!(
        carried.saturating_add(dropped),
        doc.len(),
        "the reported loss plus the surviving bytes is the input"
    );
}

#[test]
fn structured_and_unknown_are_never_truncated() {
    let json = format!("{{\"a\":[{}]}}", "1,".repeat(2_000));
    let size = ByteLen::new(u64::try_from(json.len()).unwrap());
    assert_eq!(
        plan(detect(&json), size, ByteLen::new(64)),
        Shrink::MustOffload,
        "truncated structured data looks parseable"
    );
    assert_eq!(
        plan(Content::Unknown, ByteLen::new(9_000), ByteLen::new(64)),
        Shrink::MustOffload,
        "nothing is thrown away on a guess"
    );
}

#[test]
fn something_within_budget_is_left_exactly_alone() {
    let text = "a short result";
    let cut = compacted(text, ByteLen::new(4_096));
    assert_eq!(cut.place, Elided::Nothing);
    assert_eq!(cut.text, text);
}
