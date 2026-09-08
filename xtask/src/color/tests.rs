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

const GOOD: &str = r#"
pub const GRAY_CHROMA: u16 = 18;
pub const GRAY_RAMP: [(&str, u16); 11] = [
    ("G0", 145),
    ("G1", 195),
    ("G2", 245),
    ("G3", 300),
    ("G4", 360),
    ("G5", 430),
    ("G6", 520),
    ("G7", 630),
    ("G8", 730),
    ("G9", 830),
    ("G10", 930),
];
pub const COLOUR_TOKENS: [(&str, u16, u16, u16); 5] = [
    ("ACCENT", 680, HUE_AXIS, ACCENT_CHROMA_PERCENT),
    ("ALERT", 919, HUE_ALERT, ALERT_CHROMA_PERCENT),
    ("ACCENT_HOVER", 760, HUE_AXIS, ACCENT_CHROMA_PERCENT),
    ("ALERT_HOVER", 945, HUE_ALERT, ALERT_CHROMA_PERCENT),
    ("ACCENT_SOLID", 919, HUE_AXIS, ACCENT_CHROMA_PERCENT),
];
pub const TEXT_SURFACE_CEILING: &str = "G2";
pub const TEXT_TOKENS: [(&str, u16, u16); 4] = [
    ("TEXT", 928, 90),
    ("TEXT_QUIET", 852, 75),
    ("TEXT_FAINT", 771, 60),
    ("TEXT_DISABLED", 582, 30),
];
pub const TYPE_SCALE: [(&str, u16, u16, u16); 6] = [
    ("figure", 28, 600, 60),
    ("title", 20, 600, 60),
    ("heading", 18, 600, 60),
    ("label", 14, 600, 90),
    ("body", 14, 400, 90),
    ("note", 15, 400, 75),
];
"#;

#[test]
fn the_real_shape_passes() {
    let found = judge_tokens(GOOD);
    assert!(found.is_empty(), "{}", rules(&found));
    assert_eq!(grey_ramp(GOOD).len(), 11);
    assert_eq!(parse_colour_tokens(GOOD).len(), 5);
    assert_eq!(parse_text_tokens(GOOD).len(), 4);
    assert_eq!(parse_type_scale(GOOD).len(), 6);
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
    // is wanted. It reaches Lc 73.8 on a card, and body needs 90.
    let broken = GOOD.replace("(\"TEXT\", 928, 90)", "(\"TEXT\", 830, 90)");
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
    let broken = GOOD.replace("(\"note\", 15, 400, 75)", "(\"note\", 15, 400, 60)");
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
    let broken = GOOD.replace("(\"label\", 14, 600, 90)", "(\"label\", 11, 600, 90)");
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
    let quiet_enough: Vec<&str> = parse_text_tokens(GOOD)
        .into_iter()
        .filter(|(_, _, tier)| *tier < 90)
        .map(|(name, _, _)| match name.as_str() {
            "TEXT_QUIET" => "TEXT_QUIET",
            "TEXT_FAINT" => "TEXT_FAINT",
            _ => "other",
        })
        .collect();
    assert!(
        !quiet_enough.is_empty(),
        "there is a quieter token; it is 12px that cannot use it"
    );
}

#[test]
fn text_on_a_surface_the_ladder_may_not_reach_is_caught() {
    // G3 is where the ladder stops carrying text. Pointing the ceiling
    // at it must make the body token illegal, because it is.
    let broken = GOOD.replace(
        "pub const TEXT_SURFACE_CEILING: &str = \"G2\"",
        "pub const TEXT_SURFACE_CEILING: &str = \"G3\"",
    );
    let found = judge_tokens(&broken);
    assert!(
        found.iter().any(|v| v.violation.starts_with("TEXT claims")),
        "{}",
        rules(&found)
    );
}

#[test]
fn a_third_hue_is_caught() {
    let broken = GOOD.replace("(\"ALERT\", 919, HUE_ALERT", "(\"ALERT\", 919, 12");
    let found = judge_tokens(&broken);
    assert!(
        found.iter().any(|v| v.violation.contains("hue 12")),
        "{}",
        rules(&found)
    );
}

#[test]
fn pure_white_is_caught() {
    let broken = GOOD.replace("(\"G10\", 930)", "(\"G10\", 1000)");
    let found = judge_tokens(&broken);
    assert!(
        found.iter().any(|v| v.violation.contains("1000 per mille")),
        "{}",
        rules(&found)
    );
}

#[test]
fn a_ramp_that_stops_short_of_the_ceiling_is_caught() {
    let broken = GOOD.replace("(\"G10\", 930)", "(\"G10\", 900)");
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
    let broken = GOOD.replace(
        "(\"ACCENT_HOVER\", 760, HUE_AXIS, ACCENT_CHROMA_PERCENT)",
        "(\"ACCENT_HOVER\", 760, HUE_AXIS, 71)",
    );
    let found = judge_tokens(&broken);
    assert!(
        found.iter().any(|v| v.violation.contains("3 distinct")),
        "{}",
        rules(&found)
    );
}

#[test]
fn a_shortened_ramp_is_caught() {
    let broken = GOOD.replace("    (\"G5\", 430),\n", "");
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

/// Each client names colour in exactly one file, and only there.
///
/// Two clients coexist until `crates/web` is deleted, so the scan is run
/// over a tree that holds both production points plus one ordinary
/// stylesheet: the two production points are silent and the third file
/// is reported.
#[test]
fn each_client_may_name_colour_in_its_own_theme_file() {
    let root = std::env::temp_dir().join(format!("color-theme-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    let written = [
        (
            super::THEME,
            "pub const A: &str = \"oklch(0.145 0.018 264)\";",
        ),
        (
            "client/src/theme.css",
            "  --color-g0: oklch(0.145 0.018 264);",
        ),
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
        "only the file that is no client's production point is a violation; got {}",
        rules(&found)
    );
    std::fs::remove_dir_all(&root).unwrap();
}
