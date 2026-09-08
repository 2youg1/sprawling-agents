// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use super::{Box, judge};

use super::engine::{parse_box, sink};

#[cfg(test)]
fn boxed(kind: &str, class: &str, left: i64, top: i64, width: i64) -> Box {
    Box {
        kind: kind.to_owned(),
        tag: "DIV".to_owned(),
        class: class.to_owned(),
        left,
        top,
        width,
        height: 40,
    }
}

#[test]
fn a_second_left_edge_is_caught() {
    // The defect this pins: `.panel`'s shorthand margin reset the
    // inline auto margins the centre column had set, so a page's
    // heading was centred and the panels under it were not.
    let boxes = vec![
        boxed("centre", "centre", 200, 0, 1146),
        boxed("region", "record-head", 284, 60, 1040),
        boxed("region", "panel", 253, 160, 1040),
    ];
    let mut out = Vec::new();
    judge("record.html", &boxes, &mut out);
    assert_eq!(out.len(), 1, "one page, one edge");
    assert!(out.iter().any(|v| v.violation.contains("x=284")));
}

#[test]
fn a_head_laid_out_beside_its_body_is_caught() {
    // The defect this pins: `form { display: flex }` captured the
    // composer, so its head sat beside the box instead of above it.
    let boxes = vec![
        boxed("centre", "centre", 200, 0, 1146),
        boxed("region", "panel.composer", 253, 60, 1040),
        boxed("panel", "panel.composer", 253, 60, 1040),
        boxed("part", "panel-body", 755, 60, 400),
        boxed("part", "panel-head", 253, 205, 400),
    ];
    let mut out = Vec::new();
    judge("sessions.html", &boxes, &mut out);
    assert!(
        out.iter()
            .any(|v| v.rule.contains("the top of its own panel")),
        "a head below its own body is the row layout"
    );
}

#[test]
fn a_page_whose_regions_agree_is_clean() {
    let boxes = vec![
        boxed("centre", "centre", 200, 0, 1146),
        boxed("region", "record-head", 253, 60, 1040),
        boxed("region", "panel", 253, 160, 1040),
        boxed("panel", "panel", 253, 160, 1040),
        boxed("part", "panel-head", 253, 160, 400),
        boxed("part", "panel-body", 253, 200, 400),
    ];
    let mut out = Vec::new();
    judge("record.html", &boxes, &mut out);
    assert!(out.is_empty(), "{:?}", out.first().map(|v| &v.violation));
}

#[test]
fn a_box_wider_than_its_region_is_caught() {
    let boxes = vec![
        boxed("centre", "centre", 200, 0, 1000),
        boxed("region", "panel", 200, 60, 2376),
    ];
    let mut out = Vec::new();
    judge("sessions.html", &boxes, &mut out);
    assert!(out.iter().any(|v| v.rule.contains("wider than the region")));
}

#[test]
fn the_probe_writes_where_the_gate_reads() {
    let dom = "<html><body><pre id=\"sprawling-render\">region DIV a 1 2 3 4</pre></body>";
    let records = sink(dom).expect("the sink is found");
    let held = parse_box(records).expect("one record parses");
    assert_eq!((held.left, held.top, held.width, held.height), (1, 2, 3, 4));
}
