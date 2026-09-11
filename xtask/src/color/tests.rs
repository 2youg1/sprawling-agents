// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use super::*;

use super::scan;
use super::scan::literal_at;

/// `Violation` has no `Debug` on purpose (it is rendered, not dumped),
/// so failures report the rules that fired.
fn rules(found: &[Violation]) -> String {
    found
        .iter()
        .map(|v| format!("{} :: {}", v.rule, v.violation))
        .collect::<Vec<_>>()
        .join(" | ")
}

/// The shape the client ships: the `@theme` token block, and beside it the
/// three inputs the proof needs that an `oklch()` call cannot carry.
const GOOD: &str = r"
@theme {
  --color-*: initial;
  --color-g0: oklch(0.145 0.018 264);
  --color-g1: oklch(0.195 0.018 264);
  --color-g2: oklch(0.245 0.018 264);
  --color-g3: oklch(0.300 0.018 264);
  --color-g4: oklch(0.360 0.018 264);
  --color-g5: oklch(0.430 0.018 264);
  --color-g6: oklch(0.520 0.018 264);
  --color-g7: oklch(0.630 0.018 264);
  --color-g8: oklch(0.730 0.018 264);
  --color-g9: oklch(0.830 0.018 264);
  --color-g10: oklch(0.930 0.018 264);

  --color-accent: oklch(0.680 calc(0.151 * var(--chroma)) 264);
  --color-alert: oklch(0.919 calc(0.046 * var(--chroma)) 84);
  --color-accent-hover: oklch(0.760 calc(0.109 * var(--chroma)) 264);
  --color-alert-hover: oklch(0.945 calc(0.030 * var(--chroma)) 84);
  --color-accent-solid: oklch(0.919 calc(0.034 * var(--chroma)) 264);

  --color-text: oklch(0.928 0.018 264);
  --color-text-quiet: oklch(0.852 0.018 264);
  --color-text-faint: oklch(0.771 0.018 264);
  --color-text-disabled: oklch(0.582 0.018 264);

  --text-figure: 28px;
  --text-title: 20px;
  --text-heading: 18px;
  --text-label: 14px;
  --text-body: 15px;
  --text-note: 15px;
  --font-weight-figure: 600;
  --font-weight-title: 600;
  --font-weight-heading: 600;
  --font-weight-label: 600;
  --font-weight-body: 400;
  --font-weight-note: 400;
}

:root {
  --ratio-accent: 90;
  --ratio-alert: 55;
  --ratio-accent-hover: 90;
  --ratio-alert-hover: 55;
  --ratio-accent-solid: 90;

  --tier-text: 90;
  --tier-text-quiet: 75;
  --tier-text-faint: 60;
  --tier-text-disabled: 30;
  --tier-figure: 60;
  --tier-title: 60;
  --tier-heading: 60;
  --tier-label: 90;
  --tier-body: 90;
  --tier-note: 75;

  --surface-ceiling: g2;
}
";

#[test]
fn the_real_shape_passes() {
    let found = judge_tokens(GOOD);
    assert!(found.is_empty(), "{}", rules(&found));
    assert_eq!(grey_ramp(GOOD).len(), 11);
    assert_eq!(parse_colour_tokens(GOOD).len(), 5);
    assert_eq!(parse_text_tokens(GOOD).len(), 4);
    assert_eq!(parse_type_scale(GOOD).len(), 6);
}

/// The names the rest of the repository uses, which are not the names CSS
/// spells. `badge` asks the ramp for `G1`; the stylesheet declares
/// `--color-g1`, and the reader who goes looking for either finds one
/// thing.
#[test]
fn a_token_is_reported_by_the_name_the_repository_uses() {
    let rungs = grey_ramp(GOOD);
    assert_eq!(
        rungs.first().map(|(name, l)| (name.as_str(), *l)),
        Some(("G0", 145))
    );
    assert_eq!(
        rungs.last().map(|(name, l)| (name.as_str(), *l)),
        Some(("G10", 930))
    );
    let coloured: Vec<String> = parse_colour_tokens(GOOD)
        .into_iter()
        .map(|(name, _, _, _)| name)
        .collect();
    assert!(
        coloured.iter().any(|name| name == "ACCENT_HOVER"),
        "got {coloured:?}"
    );
    assert_eq!(text_surface_ceiling(GOOD).as_deref(), Some("G2"));
}

/// Tailwind's reset lines declare no token and must not become one.
#[test]
fn a_reset_declaration_is_not_a_token() {
    assert!(
        !grey_ramp(GOOD).iter().any(|(name, _)| name.contains('*')),
        "the `--color-*: initial` reset was read as a rung"
    );
}

/// The reading this gate exists to make mechanical: these are the
/// measured values the design was solved against, so a change to the
/// transfer curve, the constants or the quantisation shows up here
/// rather than as a page that is quietly harder to read.
#[test]
fn the_measurement_reproduces_the_readings_the_design_was_solved_against() {
    for (text, surface, expected) in [
        (930, 245, 90.6),
        (930, 195, 91.9),
        (930, 300, 88.1),
        (928, 245, 90.0),
        (852, 245, 75.0),
        (771, 245, 60.0),
        (830, 245, 70.4),
        (630, 195, 38.2),
    ] {
        let got = apca_lc(text, surface);
        assert!(
            (got - expected).abs() < 0.15,
            "L {text} on L {surface}: expected Lc {expected}, measured {got:.1}"
        );
    }
}

#[test]
fn a_text_token_that_does_not_reach_its_tier_is_caught() {
    // G9 is the rung a designer would reach for when "a bit quieter"
    // is wanted. It reaches Lc 70.4 on a card, and body needs 90.
    let broken = GOOD.replace("--color-text: oklch(0.928", "--color-text: oklch(0.830");
    let found = judge_tokens(&broken);
    assert!(
        found
            .iter()
            .any(|v| v.violation.contains("TEXT claims Lc 90 and reaches 70.4")),
        "{}",
        rules(&found)
    );
}

#[test]
fn a_type_step_claiming_the_wrong_tier_is_caught() {
    let broken = GOOD.replace("--tier-note: 75", "--tier-note: 60");
    let found = judge_tokens(&broken);
    assert!(
        found
            .iter()
            .any(|v| v.violation.contains("note claims Lc 60 and demands Lc 75")),
        "{}",
        rules(&found)
    );
}

/// One of the two steps this library used to ship is below every
/// Bronze minimum, and no colour repairs that - which is why the fix
/// was to the type scale and not to the greys.
#[test]
fn a_step_too_small_for_any_tier_is_caught() {
    let broken = GOOD.replace("--text-label: 14px", "--text-label: 11px");
    let found = judge_tokens(&broken);
    assert!(
        found
            .iter()
            .any(|v| v.violation.contains("below every Bronze minimum")),
        "{}",
        rules(&found)
    );
}

/// The other step was legal and still had to go, which is a different
/// finding and worth its own assertion: 12px at weight 400 is admitted,
/// but only at the top tier, so the only token that may paint it is the
/// one body text uses. **A 12px line cannot be quieter than the prose
/// beside it** - and "quieter" was its entire job. Quiet has to be
/// bought with size here, not with grey.
#[test]
fn a_twelve_pixel_step_is_legal_and_still_cannot_be_quiet() {
    assert_eq!(bronze_tier(12, 400, false), Some(90));
    assert_eq!(bronze_tier(15, 400, false), Some(75));
    let quiet_enough: Vec<String> = parse_text_tokens(GOOD)
        .into_iter()
        .filter(|(_, _, tier)| *tier < 90)
        .map(|(name, _, _)| name)
        .collect();
    assert!(
        quiet_enough.iter().any(|name| name == "TEXT_QUIET"),
        "there is a quieter token; it is 12px that cannot use it: {quiet_enough:?}"
    );
}

#[test]
fn text_on_a_surface_the_ladder_may_not_reach_is_caught() {
    // G3 is where the ladder stops carrying text. Pointing the ceiling
    // at it must make the body token illegal, because it is.
    let broken = GOOD.replace("--surface-ceiling: g2", "--surface-ceiling: g3");
    let found = judge_tokens(&broken);
    assert!(
        found.iter().any(|v| v.violation.starts_with("TEXT claims")),
        "{}",
        rules(&found)
    );
}

#[test]
fn a_third_hue_is_caught() {
    let broken = GOOD.replace(
        "--color-alert: oklch(0.919 calc(0.046 * var(--chroma)) 84)",
        "--color-alert: oklch(0.919 calc(0.046 * var(--chroma)) 12)",
    );
    let found = judge_tokens(&broken);
    assert!(
        found.iter().any(|v| v.violation.contains("hue 12")),
        "{}",
        rules(&found)
    );
}

#[test]
fn pure_white_is_caught() {
    let broken = GOOD.replace("--color-g10: oklch(0.930", "--color-g10: oklch(1.000");
    let found = judge_tokens(&broken);
    assert!(
        found.iter().any(|v| v.violation.contains("1000 per mille")),
        "{}",
        rules(&found)
    );
}

#[test]
fn a_ramp_that_stops_short_of_the_ceiling_is_caught() {
    let broken = GOOD.replace("--color-g10: oklch(0.930", "--color-g10: oklch(0.900");
    let found = judge_tokens(&broken);
    assert!(
        found.iter().any(|v| v.violation.contains("145 to 900")),
        "{}",
        rules(&found)
    );
}

#[test]
fn an_interaction_variant_may_sit_above_the_ramp_ceiling() {
    // ALERT_HOVER at 945 is legal and the theme ships it; only
    // pure white is not.
    let found = judge_tokens(GOOD);
    assert!(found.is_empty(), "{}", rules(&found));
}

#[test]
fn a_third_ratio_is_caught() {
    let broken = GOOD.replace("--ratio-accent-hover: 90", "--ratio-accent-hover: 71");
    let found = judge_tokens(&broken);
    assert!(
        found.iter().any(|v| v.violation.contains("3 distinct")),
        "{}",
        rules(&found)
    );
}

/// A resolved chroma with no share beside it is the failure this gate
/// would otherwise pass silently: `oklch()` keeps the product and throws
/// away the multiplier, so a token that stops declaring its ratio simply
/// leaves the table it is judged by.
#[test]
fn a_coloured_token_that_declares_no_ratio_is_named() {
    let broken = GOOD.replace("--ratio-alert-hover: 55", "--ratio-nothing: 55");
    let found = judge_tokens(&broken);
    assert!(
        found
            .iter()
            .any(|v| v.violation.contains("ALERT_HOVER") && v.violation.contains("no `--ratio-`")),
        "{}",
        rules(&found)
    );
}

/// The axis chroma is a property of every rung, not of a constant that
/// says so. A stylesheet can declare the right number and then write a
/// rung that departs from it.
#[test]
fn a_rung_off_the_axis_chroma_is_caught() {
    let broken = GOOD.replace(
        "--color-g4: oklch(0.360 0.018",
        "--color-g4: oklch(0.360 0.040",
    );
    let found = judge_tokens(&broken);
    assert!(
        found
            .iter()
            .any(|v| v.violation.contains("G4 is at chroma 40 per mille")),
        "{}",
        rules(&found)
    );
}

#[test]
fn a_shortened_ramp_is_caught() {
    let broken = GOOD.replace("  --color-g5: oklch(0.430 0.018 264);\n", "");
    let found = judge_tokens(&broken);
    assert!(found.iter().any(|v| v.violation.contains("found 10")));
}

#[test]
fn colour_spellings_are_recognised_and_locators_are_not() {
    assert_eq!(literal_at("  color: #070A12;", true), Some("#rrggbb"));
    assert_eq!(literal_at("background: rgb(1,2,3)", true), Some("rgb("));
    assert_eq!(
        literal_at("--G0:oklch(0.145 0.018 264)", true),
        Some("oklch(")
    );
    assert_eq!(literal_at("let x = 3;", false), None);

    // The shape somebody writes in Rust when they mean a colour.
    assert_eq!(
        literal_at(r##"let bg = "#070A12";"##, false),
        Some("#rrggbb")
    );

    // A Locator fragment is not a colour. This exact line is what the
    // gate's first run tripped on.
    assert_eq!(
        literal_at(r##"format!("cas:b3-{H64}#B01-2"),"##, false),
        None
    );
    assert_eq!(literal_at("issue #4707 records the status", false), None);
    // A digest is longer than six hex digits either way.
    assert_eq!(
        literal_at(
            "// 692b5f963f99f018496b8df111314dfe1bed52ccfe1a40cb9a5975b3bc8664fe",
            true
        ),
        None
    );
}

/// The client names colour in exactly one file, and only there.
///
/// The scan runs over a tree holding the production point and one ordinary
/// stylesheet: the production point is silent and the other file is
/// reported, which is the whole rule.
#[test]
fn the_client_names_colour_in_one_file() {
    let root = std::env::temp_dir().join(format!("color-theme-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    let written = [
        (super::THEME, "  --color-g0: oklch(0.145 0.018 264);"),
        ("client/src/panel.css", "  color: oklch(0.145 0.018 264);"),
    ];
    for (rel, body) in written {
        let path = root.join(rel);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, body).unwrap();
    }

    let found = scan::scan_for_literals(&root).unwrap();
    let places: Vec<&str> = found.iter().map(|v| v.location.as_str()).collect();
    assert_eq!(
        places,
        ["client/src/panel.css:1"],
        "only the file that is not the production point is a violation; got {}",
        rules(&found)
    );
    std::fs::remove_dir_all(&root).unwrap();
}
