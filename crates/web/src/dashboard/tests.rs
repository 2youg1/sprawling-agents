// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use super::*;

fn usd(v: u64) -> UsdMicros {
    UsdMicros::new(v)
}

#[test]
fn the_page_carries_exactly_the_five_attribution_dimensions() {
    // Constitution 12.3, and the same five memory::attribution reconciles
    // against one authoritative total (A20).
    assert_eq!(CostDimension::all().len(), 5);
    let mut names: Vec<&str> = CostDimension::all().iter().map(|d| d.as_str()).collect();
    names.sort_unstable();
    names.dedup();
    assert_eq!(names.len(), 5);
}

#[test]
fn rows_are_ordered_by_what_they_cost() {
    let rows = cost_rows(
        vec![
            ("skill-a".to_owned(), usd(0), usd(100_000)),
            ("skill-b".to_owned(), usd(0), usd(900_000)),
            ("skill-c".to_owned(), usd(0), usd(500_000)),
        ],
        usd(1_500_000),
    );
    let order: Vec<&str> = rows.iter().map(|r| r.label.as_str()).collect();
    assert_eq!(order, ["skill-b", "skill-c", "skill-a"]);
    assert_eq!(rows[0].share, 600);
}

#[test]
fn shares_are_taken_against_the_authoritative_total_not_the_row_sum() {
    // Rows may not cover the whole spend - an unattributed remainder is
    // honest and A20 keeps it. Normalising to the row sum would hide it.
    let rows = cost_rows(
        vec![("tool".to_owned(), usd(0), usd(250_000))],
        usd(1_000_000),
    );
    assert_eq!(rows[0].share, 250, "a quarter of the real total");
}

#[test]
fn an_unchanged_row_is_steady_rather_than_rising_by_nothing() {
    assert_eq!(Trend::between(usd(10), usd(10)), Trend::Steady);
    assert_eq!(Trend::between(usd(10), usd(11)), Trend::Rising);
    assert_eq!(Trend::between(usd(11), usd(10)), Trend::Falling);
}

#[test]
fn trends_are_marked_by_shape_so_the_page_survives_desaturation() {
    let mut marks: Vec<&str> = [Trend::Rising, Trend::Steady, Trend::Falling]
        .iter()
        .map(|t| t.mark())
        .collect();
    marks.sort_unstable();
    marks.dedup();
    assert_eq!(marks.len(), 3);
}

#[test]
fn a_chart_refuses_a_fifth_series_rather_than_reusing_a_pattern() {
    assert!(drawable(4));
    assert!(!drawable(5));
    assert_eq!(SERIES_WIDTHS.len(), SERIES_PER_CHART_MAX);
    assert_eq!(SERIES_DASHES.len(), SERIES_PER_CHART_MAX);
    let pairs: std::collections::BTreeSet<(&str, &str)> = SERIES_WIDTHS
        .iter()
        .zip(SERIES_DASHES.iter())
        .map(|(w, d)| (*w, *d))
        .collect();
    assert_eq!(pairs.len(), SERIES_PER_CHART_MAX, "no pair repeats");
}

#[test]
fn a_zero_total_produces_no_share_rather_than_a_division() {
    assert_eq!(share_per_mille(usd(5), usd(0)), 0);
}

#[test]
fn the_folded_line_holds_everything_the_row_says() {
    let row = CostRow {
        label: "prefix".to_owned(),
        spent: usd(1_500_000),
        share: 333,
        trend: Trend::Rising,
    };
    let line = fold_line(&row);
    assert!(line.contains("prefix"));
    assert!(line.contains("$1.50"));
    assert!(line.contains("33%"));
    assert!(line.contains('^'));
}
