// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use super::*;

const AT: &str = "#/gallery at 1280px, dark in the colours it authored";

fn el(tag: &str, at: [i64; 4], parent: i64) -> Drawn {
    let [left, top, width, height] = at;
    Drawn {
        tag: tag.to_owned(),
        role: "-".to_owned(),
        name: String::new(),
        left,
        top,
        width,
        height,
        depth: if parent < 0 { 2 } else { 4 },
        parent,
        across: Overflow::Shows,
        down: Overflow::Shows,
        sampled: Sampled::Frame,
        first_mark: -1,
        underlined: false,
        fill: None,
        class: String::new(),
        text: None,
    }
}

fn page(drawn: Vec<Drawn>, declared: Declared) -> Page {
    Page {
        drawn,
        declared,
        painted_by: PaintSource::ThePage,
    }
}

fn nothing_declared() -> Declared {
    Declared::of(Vec::new(), Vec::new(), Vec::new())
}

fn ramp() -> Declared {
    Declared::of(
        vec![
            ("--color-g0".to_owned(), Paint::of(0x14, 0x16, 0x1a)),
            ("--color-g1".to_owned(), Paint::of(0x1e, 0x21, 0x26)),
            ("--color-text".to_owned(), Paint::of(0xe8, 0xea, 0xee)),
        ],
        vec![("--text-body".to_owned(), 1500)],
        vec![("--spacing-snug".to_owned(), 800)],
    )
}

fn holder() -> Drawn {
    el("DIV", [0, 0, 400, 400], -1)
}

#[test]
fn a_near_miss_against_a_majority_reports_the_value_to_write() {
    let mut drawn = vec![holder()];
    for top in [0_i64, 20, 40] {
        drawn.push(el("DIV", [56, top, 100, 16], 0));
    }
    drawn.push(el("DIV", [58, 60, 100, 16], 0));
    let measured = page(drawn, nothing_declared());
    let found = judge(&measured);
    let told: Vec<String> = found.iter().map(sheet::says).collect();

    assert_eq!(found.len(), 2, "{told:?}");
    assert!(
        told.iter()
            .any(|line| line.contains("the left edge") && line.contains("reads 58; 3 of 4 read 56")),
        "{told:?}",
    );
    assert!(
        found
            .iter()
            .all(|one| matches!(one.standing(), Standing::Noted)),
        "an observed population must not stop a build",
    );
}

#[test]
fn two_boxes_do_not_make_a_scale() {
    let drawn = vec![
        holder(),
        el("DIV", [56, 0, 100, 16], 0),
        el("DIV", [58, 20, 100, 16], 0),
    ];
    let measured = page(drawn, nothing_declared());
    let found = judge(&measured);
    assert!(found.is_empty(), "{}", found.len());
}

#[test]
fn a_box_far_from_the_others_is_a_different_column() {
    let mut drawn = vec![holder()];
    for top in [0_i64, 20, 40] {
        drawn.push(el("DIV", [56, top, 100, 16], 0));
    }
    drawn.push(el("DIV", [200, 60, 100, 16], 0));
    let measured = page(drawn, nothing_declared());
    let found = judge(&measured);
    assert!(found.is_empty(), "{}", found.len());
}

#[test]
fn a_box_outside_what_holds_it_is_refused() {
    let drawn = vec![holder(), el("BUTTON", [500, 12, 200, 40], 0)];
    let measured = page(drawn, nothing_declared());
    let found = judge(&measured);
    assert!(
        found
            .iter()
            .any(|one| matches!(one.finding, Finding::Escapes { .. })
                && matches!(one.standing(), Standing::Refused)),
        "an escape must stop a build",
    );
}

#[test]
fn a_colour_the_page_never_declared_is_reported_with_the_word_to_write() {
    let mut off = el("DIV", [0, 0, 100, 16], -1);
    off.fill = Some(Paint::of(0x1e, 0x21, 0x2b));
    let measured = page(vec![off], ramp());
    let found = judge(&measured);
    let written: Vec<String> = found.iter().map(sheet::remedy).collect();
    assert!(
        written.iter().any(|line| line.contains("--color-g1")),
        "{written:?}",
    );
}

#[test]
fn a_page_that_uses_its_own_words_says_nothing_about_them() {
    let mut on = el("DIV", [0, 0, 100, 16], -1);
    on.fill = Some(Paint::of(0x1e, 0x21, 0x26));
    on.text = Some(TextRun {
        ink: Some(Paint::of(0xe8, 0xea, 0xee)),
        px_x100: 1500,
        cut: Cut::Fits,
    });
    let measured = page(vec![on], ramp());
    let found = judge(&measured);
    assert!(found.is_empty(), "{}", found.len());
}

#[test]
fn a_declared_word_nobody_paints_is_named() {
    let mut on = el("DIV", [0, 0, 100, 16], -1);
    on.fill = Some(Paint::of(0x1e, 0x21, 0x26));
    let measured = page(vec![on], ramp());
    let report = written(
        Survey {
            page: &measured,
            at: AT,
            found: &judge(&measured),
            sources: &Sources::default(),
        },
        Shape::Prose,
    );
    assert!(report.contains("--color-g0"), "{report}");
    assert!(report.contains("--color-text"), "{report}");
}

#[test]
fn only_an_unmarked_cut_is_a_finding() {
    assert!(matches!(
        Cut::of(100, 140, Marking::Clip),
        Cut::Hidden { .. }
    ));
    assert!(matches!(Cut::of(100, 140, Marking::Ellipsis), Cut::Marked));
    assert!(matches!(Cut::of(100, 140, Marking::Spills), Cut::Fits));
    assert!(matches!(Cut::of(100, 101, Marking::Clip), Cut::Fits));
}

#[test]
fn a_clean_page_prints_nothing() {
    let mut on = el("DIV", [0, 0, 100, 16], -1);
    on.fill = Some(Paint::of(0x1e, 0x21, 0x26));
    let declared = Declared::of(
        vec![("--color-g1".to_owned(), Paint::of(0x1e, 0x21, 0x26))],
        Vec::new(),
        Vec::new(),
    );
    let measured = page(vec![on], declared);
    assert_eq!(
        written(
            Survey {
                page: &measured,
                at: AT,
                found: &judge(&measured),
                sources: &Sources::default(),
            },
            Shape::Prose
        ),
        ""
    );
}

#[test]
fn the_groups_come_out_in_repair_order() {
    let mut off = el("DIV", [0, 0, 100, 16], -1);
    off.fill = Some(Paint::of(0x2b, 0x2b, 0x2b));
    off.text = Some(TextRun {
        ink: Some(Paint::of(0xe8, 0xea, 0xee)),
        px_x100: 1300,
        cut: Cut::Hidden {
            shown: 100,
            needs: 140,
        },
    });
    let measured = page(vec![off], ramp());
    let report = written(
        Survey {
            page: &measured,
            at: AT,
            found: &judge(&measured),
            sources: &Sources::default(),
        },
        Shape::Prose,
    );
    let seats: Vec<usize> = Group::IN_ORDER
        .into_iter()
        .filter_map(|group| report.find(&format!("## {}", group.called())))
        .collect();
    let mut sorted = seats.clone();
    sorted.sort_unstable();
    assert_eq!(seats, sorted, "{report}");
}

#[test]
fn a_run_of_words_outside_its_row_is_reported_and_not_refused() {
    let mut span = el("SPAN", [500, 12, 200, 20], 0);
    span.sampled = Sampled::Words;
    let measured = page(vec![holder(), span], nothing_declared());
    let found = judge(&measured);
    assert!(
        found
            .iter()
            .any(|one| matches!(one.finding, Finding::Escapes { .. })
                && matches!(one.standing(), Standing::Noted)),
        "a reading the widened sample added must not stop a build",
    );
}

#[test]
fn a_colour_is_written_the_way_a_person_greps_for_it() {
    assert_eq!(Paint::of(0x2b, 0x2b, 0x2b).to_string(), "#2b2b2b");
}
