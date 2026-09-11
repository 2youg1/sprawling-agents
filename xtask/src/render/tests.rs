// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use super::*;

/// `Violation` has no `Debug` on purpose (it is rendered, not dumped),
/// so failures report the rules that fired.
fn rules(found: &[Violation]) -> String {
    found
        .iter()
        .map(|v| format!("{} :: {}", v.rule, v.violation))
        .collect::<Vec<_>>()
        .join(" | ")
}

/// One element, in the shape the probe writes: what it is, where it was
/// drawn (`[left, top, width, height]`), and where it sits in the tree
/// (`(depth, nearest measured ancestor)`).
fn el(tag: &str, name: &str, at: [i64; 4], nesting: (i64, i64)) -> Drawn {
    let [left, top, width, height] = at;
    let (depth, parent) = nesting;
    Drawn {
        scrolls_across: false,
        scrolls_down: false,
        first_mark: -1,
        underlined: false,
        tag: tag.to_owned(),
        role: "-".to_owned(),
        name: name.to_owned(),
        left,
        top,
        width,
        height,
        depth,
        parent,
    }
}

/// The gallery as it stands: a rail, a main column, a heading, and three
/// sections that share one left edge.
fn good() -> Vec<Drawn> {
    vec![
        el("NAV", "去处", [0, 0, 44, 1061], (3, -1)),
        el("MAIN", "页面", [44, 0, 1372, 1061], (3, -1)),
        el("H1", "画廊", [361, 16, 728, 27], (5, 1)),
        el("SECTION", "thinking", [361, 67, 728, 113], (5, 1)),
        el("SECTION", "calling", [361, 204, 728, 113], (5, 1)),
        el("SECTION", "waiting", [361, 341, 728, 113], (5, 1)),
        el("BUTTON", "/stop", [951, 138, 51, 29], (8, 3)),
    ]
}

/// One clickable row of the rail: where it was drawn, and the centre of
/// the first mark inside it - the dot, or the glyph.
fn row(name: &str, top: i64, mark: i64) -> Drawn {
    let mut held = el("A", name, [0, top, 44, 44], (5, 0));
    held.first_mark = mark;
    held
}

/// The rail as it stands: every row's first mark on one x.
fn railed() -> Vec<Drawn> {
    let mut drawn = good();
    drawn.push(row("城", 8, 21));
    drawn.push(row("市长室", 52, 21));
    drawn.push(row("城市", 96, 21));
    drawn
}

fn judge(drawn: &[Drawn]) -> Vec<Violation> {
    let mut out = Vec::new();
    every_control_is_announceable(drawn, &mut out);
    every_landmark_is_named(drawn, &mut out);
    one_first_heading(drawn, &mut out);
    one_left_edge(drawn, &mut out);
    nothing_escapes_what_holds_it(drawn, &mut out);
    rows_share_a_first_mark(drawn, &mut out);
    no_key_is_underlined(drawn, &mut out);
    out
}

#[test]
fn the_rail_the_client_draws_passes() {
    let found = judge(&railed());
    assert!(found.is_empty(), "{}", rules(&found));
}

/// The defect §5.1 names: the status dot centred in its own 8 px box
/// sits five pixels left of every glyph under it.
#[test]
fn a_dot_five_pixels_left_of_the_glyphs_is_caught() {
    let mut drawn = railed();
    if let Some(first) = drawn.get_mut(7) {
        first.first_mark = 16;
    }
    let found = judge(&drawn);
    assert!(
        found.iter().any(|v| v.rule.contains("first mark")),
        "{}",
        rules(&found)
    );
}

/// A bar of tabs is not a column, and rows that share one top cannot
/// share one x without sitting on top of one another.
#[test]
fn a_row_of_tabs_is_not_asked_to_share_an_x() {
    let mut drawn = good();
    drawn.push(el("NAV", "组", [44, 0, 1372, 44], (4, 1)));
    for (at, mark) in [(0_i64, 66_i64), (1, 156), (2, 246)] {
        let mut tab = row("组", 0, mark);
        tab.left = at.saturating_mul(90).saturating_add(44);
        tab.parent = 7;
        tab.depth = 6;
        drawn.push(tab);
    }
    let found = judge(&drawn);
    assert!(found.is_empty(), "{}", rules(&found));
}

/// A row whose first mark the probe could not find is not compared
/// against the rows whose mark it did.
#[test]
fn a_row_with_no_mark_inside_it_is_left_alone() {
    let mut drawn = railed();
    drawn.push(el("A", "没有图形", [0, 140, 44, 44], (5, 0)));
    let found = judge(&drawn);
    assert!(found.is_empty(), "{}", rules(&found));
}

/// The other half of §5.2: the stylesheet underlined a link under the
/// pointer and took the key drawn inside it along.
#[test]
fn an_underlined_key_is_caught() {
    let mut drawn = railed();
    let mut key = el("KBD", "g c", [10, 60, 20, 16], (7, 8));
    key.underlined = true;
    drawn.push(key);
    let found = judge(&drawn);
    assert!(
        found.iter().any(|v| v.rule.contains("line under it")),
        "{}",
        rules(&found)
    );
}

#[test]
fn a_key_with_no_line_under_it_passes() {
    let mut drawn = railed();
    drawn.push(el("KBD", "g c", [10, 60, 20, 16], (7, 8)));
    let found = judge(&drawn);
    assert!(found.is_empty(), "{}", rules(&found));
}

#[test]
fn the_shape_the_client_draws_passes() {
    let found = judge(&good());
    assert!(found.is_empty(), "{}", rules(&found));
}

/// The finding the first real run made: the composer's text box is the
/// control the page exists for, and it was announced as nothing.
#[test]
fn a_control_with_no_accessible_name_is_caught() {
    let mut drawn = good();
    drawn.push(el("TEXTAREA", "", [373, 106, 704, 24], (8, 3)));
    let found = judge(&drawn);
    assert!(
        found
            .iter()
            .any(|v| v.violation.contains("textarea")
                && v.violation.contains("announced as nothing")),
        "{}",
        rules(&found)
    );
}

/// A control that is not drawn is not a control a person can reach, and
/// naming it would be a rule about markup rather than about a reader.
#[test]
fn a_control_with_no_area_is_not_asked_for_a_name() {
    let mut drawn = good();
    drawn.push(el("BUTTON", "", [0, 0, 0, 0], (8, 3)));
    let found = judge(&drawn);
    assert!(found.is_empty(), "{}", rules(&found));
}

#[test]
fn an_unnamed_landmark_is_caught() {
    let mut drawn = good();
    drawn.push(el("ASIDE", "", [1200, 0, 216, 1061], (3, -1)));
    let found = judge(&drawn);
    assert!(
        found.iter().any(|v| v.violation.contains("<aside>")),
        "{}",
        rules(&found)
    );
}

#[test]
fn a_page_with_two_first_headings_is_caught() {
    let mut drawn = good();
    drawn.push(el("H1", "第二个", [361, 900, 728, 27], (5, 1)));
    let found = judge(&drawn);
    assert!(
        found
            .iter()
            .any(|v| v.violation.starts_with("this page has 2")),
        "{}",
        rules(&found)
    );
}

#[test]
fn a_page_with_no_first_heading_is_caught() {
    let drawn: Vec<Drawn> = good().into_iter().filter(|held| held.tag != "H1").collect();
    let found = judge(&drawn);
    assert!(
        found
            .iter()
            .any(|v| v.violation.starts_with("this page has 0")),
        "{}",
        rules(&found)
    );
}

/// The defect this gate was written for: one region indented past the
/// others, so the page grows a second left edge.
#[test]
fn a_second_left_edge_is_caught() {
    let mut drawn = good();
    if let Some(region) = drawn.get_mut(4) {
        region.left = 393;
    }
    let found = judge(&drawn);
    assert!(
        found.iter().any(|v| v.rule.contains("one left edge")),
        "{}",
        rules(&found)
    );
}

/// A pixel of rounding is not a decision somebody made twice.
#[test]
fn a_single_pixel_of_rounding_is_not_a_second_edge() {
    let mut drawn = good();
    if let Some(region) = drawn.get_mut(4) {
        region.left = 362;
    }
    let found = judge(&drawn);
    assert!(found.is_empty(), "{}", rules(&found));
}

/// The home page whose task box floated into the top right corner: the
/// box is still painted, every test still passes, and it is the first
/// thing a person sees.
#[test]
fn a_box_that_has_left_its_container_is_caught() {
    let mut drawn = good();
    drawn.push(el("BUTTON", "escaped", [1300, 12, 200, 40], (8, 3)));
    let found = judge(&drawn);
    assert!(
        found
            .iter()
            .any(|v| v.violation.contains("escaped") && v.violation.contains("leaves")),
        "{}",
        rules(&found)
    );
}

/// A page longer than the window is not a layout defect.
///
/// The rule exists for a box painted over whatever is beside it, and a
/// container that scrolls paints nothing over anything: what is below
/// the fold is reached by scrolling. Without this the gate reads every
/// page with more content than one screen as broken, which is every
/// page this product has.
#[test]
fn a_section_below_the_fold_of_a_scrolling_column_is_not_an_escape() {
    let mut drawn = good();
    if let Some(column) = drawn.get_mut(1) {
        column.scrolls_down = true;
    }
    drawn.push(el("SECTION", "below", [361, 1041, 728, 412], (5, 1)));
    let found = judge(&drawn);
    assert!(
        found.iter().all(|v| !v.rule.contains("outside the box")),
        "{}",
        rules(&found)
    );
}

/// And the same section in a column that does not scroll still is one:
/// the exemption is the container's own overflow, not the direction.
#[test]
fn a_section_below_the_fold_of_a_fixed_column_is_still_an_escape() {
    let mut drawn = good();
    drawn.push(el("SECTION", "below", [361, 1041, 728, 412], (5, 1)));
    let found = judge(&drawn);
    assert!(
        found
            .iter()
            .any(|v| v.violation.contains("below") && v.violation.contains("leaves")),
        "{}",
        rules(&found)
    );
}

/// Containment is judged against the nearest measured ancestor, so an
/// element at the top of the page is not compared with nothing.
#[test]
fn an_element_with_no_measured_parent_is_left_alone() {
    let drawn = vec![el("MAIN", "页面", [44, 0, 1372, 1061], (3, -1))];
    let found = judge(&drawn);
    assert!(
        found.iter().all(|v| !v.rule.contains("outside the box")),
        "{}",
        rules(&found)
    );
}
