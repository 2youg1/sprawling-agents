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

/// The shape the client ships: the `@theme` token block, and beside it the
/// three inputs the proof needs that an `oklch()` call cannot carry.
const GOOD: &str = r"
@theme {
  --color-*: initial;
  --color-g0: oklch(0.145 0.014 250);
  --color-g1: oklch(0.172 0.014 250);
  --color-g2: oklch(0.215 0.014 250);
  --color-g3: oklch(0.265 0.014 250);
  --color-g4: oklch(0.330 0.014 250);
  --color-g5: oklch(0.410 0.014 250);
  --color-g6: oklch(0.500 0.014 250);
  --color-g7: oklch(0.610 0.014 250);
  --color-g8: oklch(0.720 0.014 250);
  --color-g9: oklch(0.825 0.014 250);
  --color-g10: oklch(0.930 0.014 250);

  --color-accent: oklch(0.720 calc(0.136 * var(--chroma)) 250);
  --color-alert: oklch(0.800 calc(0.113 * var(--chroma)) 70);
  --color-accent-hover: oklch(0.790 calc(0.099 * var(--chroma)) 250);
  --color-alert-hover: oklch(0.850 calc(0.082 * var(--chroma)) 70);
  --color-accent-solid: oklch(0.919 calc(0.036 * var(--chroma)) 250);
  --color-reminder-first: oklch(0.800 calc(0.154 * var(--chroma)) 150);
  --color-reminder-second: oklch(0.680 calc(0.187 * var(--chroma)) 25);

  --color-text: oklch(0.930 0.014 250);
  --color-text-quiet: oklch(0.850 0.014 250);
  --color-text-faint: oklch(0.770 0.014 250);
  --color-text-disabled: oklch(0.580 0.014 250);

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
  --ratio-alert: 70;
  --ratio-accent-hover: 90;
  --ratio-alert-hover: 70;
  --ratio-accent-solid: 90;
  --ratio-reminder-first: 70;
  --ratio-reminder-second: 90;

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
  --tier-slack: 0.05;

  --surface-ceiling: g2;
}
";

#[test]
fn the_real_shape_passes() {
    let found = judge_tokens(GOOD, Mode::Dark);
    assert!(found.is_empty(), "{}", rules(&found));
    assert_eq!(grey_ramp(GOOD).len(), 11);
    assert_eq!(parse_colour_tokens(GOOD).len(), 7);
    assert_eq!(parse_text_tokens(GOOD).len(), 4);
    assert_eq!(parse_type_scale(GOOD).len(), 6);
}

/// The names the rest of the repository uses, which are not the names CSS
/// spells. A finding names the rung `G1`; the stylesheet declares
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
        (930, 215, 91.6),
        (930, 172, 92.5),
        (930, 265, 89.9),
        (850, 215, 75.5),
        (770, 215, 60.6),
        (580, 215, 30.7),
        (825, 215, 70.8),
        (610, 172, 35.8),
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
    // is wanted. It reaches Lc 70.8 on a card, and body needs 90.
    let broken = GOOD.replace("--color-text: oklch(0.930", "--color-text: oklch(0.825");
    let found = judge_tokens(&broken, Mode::Dark);
    assert!(
        found
            .iter()
            .any(|v| v.violation.contains("TEXT claims Lc 90 and reaches 70.8")),
        "{}",
        rules(&found)
    );
}

#[test]
fn a_type_step_claiming_the_wrong_tier_is_caught() {
    let broken = GOOD.replace("--tier-note: 75", "--tier-note: 60");
    let found = judge_tokens(&broken, Mode::Dark);
    assert!(
        found
            .iter()
            .any(|v| v.violation.contains("note claims Lc 60 and demands Lc 75")),
        "{}",
        rules(&found)
    );
}

/// A type step below every Bronze minimum is a failure no colour
/// repairs, so the remedy is in the type scale and not in the greys.
#[test]
fn a_step_too_small_for_any_tier_is_caught() {
    let broken = GOOD.replace("--text-label: 14px", "--text-label: 11px");
    let found = judge_tokens(&broken, Mode::Dark);
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
    let found = judge_tokens(&broken, Mode::Dark);
    assert!(
        found.iter().any(|v| v.violation.contains("TEXT claims")),
        "{}",
        rules(&found)
    );
}

#[test]
fn a_third_hue_is_caught() {
    let broken = GOOD.replace(
        "--color-alert: oklch(0.800 calc(0.113 * var(--chroma)) 70)",
        "--color-alert: oklch(0.800 calc(0.113 * var(--chroma)) 12)",
    );
    let found = judge_tokens(&broken, Mode::Dark);
    assert!(
        found.iter().any(|v| v.violation.contains("hue 12")),
        "{}",
        rules(&found)
    );
}

/// The checkpoints are the only exceptions, and each only on its own hue:
/// moving one off it is caught, and so is another token taking a
/// checkpoint's hue.
#[test]
fn a_checkpoint_keeps_its_own_hue_and_lends_it_to_nobody() {
    let moved = GOOD.replace(
        "--color-reminder-first: oklch(0.800 calc(0.154 * var(--chroma)) 150)",
        "--color-reminder-first: oklch(0.800 calc(0.154 * var(--chroma)) 25)",
    );
    let found = judge_tokens(&moved, Mode::Dark);
    assert!(
        found
            .iter()
            .any(|v| v.violation.contains("REMINDER_FIRST sits on hue 25")),
        "{}",
        rules(&found)
    );
    let borrowed = GOOD.replace(
        "--color-alert: oklch(0.800 calc(0.113 * var(--chroma)) 70)",
        "--color-alert: oklch(0.800 calc(0.113 * var(--chroma)) 150)",
    );
    let found = judge_tokens(&borrowed, Mode::Dark);
    assert!(
        found
            .iter()
            .any(|v| v.violation.contains("ALERT sits on hue 150")),
        "{}",
        rules(&found)
    );
}

#[test]
fn pure_white_is_caught() {
    let broken = GOOD.replace("--color-g10: oklch(0.930", "--color-g10: oklch(1.000");
    let found = judge_tokens(&broken, Mode::Dark);
    assert!(
        found.iter().any(|v| v.violation.contains("1000 per mille")),
        "{}",
        rules(&found)
    );
}

#[test]
fn a_ramp_that_stops_short_of_the_ceiling_is_caught() {
    let broken = GOOD.replace("--color-g10: oklch(0.930", "--color-g10: oklch(0.900");
    let found = judge_tokens(&broken, Mode::Dark);
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
    let found = judge_tokens(GOOD, Mode::Dark);
    assert!(found.is_empty(), "{}", rules(&found));
}

#[test]
fn a_third_ratio_is_caught() {
    let broken = GOOD.replace("--ratio-accent-hover: 90", "--ratio-accent-hover: 71");
    let found = judge_tokens(&broken, Mode::Dark);
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
    let broken = GOOD.replace("--ratio-alert-hover: 70", "--ratio-nothing: 70");
    let found = judge_tokens(&broken, Mode::Dark);
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
        "--color-g4: oklch(0.330 0.014",
        "--color-g4: oklch(0.330 0.040",
    );
    let found = judge_tokens(&broken, Mode::Dark);
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
    let broken = GOOD.replace("  --color-g5: oklch(0.410 0.014 250);\n", "");
    let found = judge_tokens(&broken, Mode::Dark);
    assert!(found.iter().any(|v| v.violation.contains("found 10")));
}
