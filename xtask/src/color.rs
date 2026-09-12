// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Colour gate: the single-hue language is machine-decidable or it is just
//! a preference somebody can argue with.
//!
//! Two halves, and they sit in different places on purpose. The **six
//! assertions on the token values** are checked here against the tables in
//! `web::theme`, which this gate parses the same way `modmap` parses the
//! module table - the authority is the source file, and the gate reads it
//! rather than keeping a copy. The **repository scan** for stray colour
//! literals is the half only a gate can do, because it is a statement about
//! every file rather than about one table.
//!
//! A theme file is exempt from the scan: it is a production point. Nothing
//! else may name a colour.
//!
//! **The authority is one sentence**: colour is named once per client, in
//! that client's theme file. There is one client, so the table of
//! production points has one row and `THEME` is that row.
//!
//! **The seven token assertions read the stylesheet, not a Rust table.**
//! Three of them ask what a resolved `oklch()` value was resolved *from* -
//! the share of the gamut a token takes, the APCA tier it claims, the
//! brightest surface text may sit on - and none of the three is a colour
//! component. They are declared beside the values they govern, as
//! properties the cascade ignores (xtask-SPEC.md section 8-8).

use std::path::Path;

use crate::report::{Violation, XtaskError};
use crate::walk;

mod contrast;
mod scan;
mod tables;

pub(crate) use tables::grey_ramp;

use contrast::{apca_lc, bronze_tier};
use scan::scan_for_literals;
use tables::{
    colour_tokens_without_ratio, grey_chromas, parse_colour_tokens, parse_text_tokens,
    parse_type_scale, text_surface_ceiling,
};

pub(crate) const THEME: &str = "client/src/theme.css";
pub(crate) const HUE_AXIS: u16 = 264;
const HUE_ALERT: u16 = 84;
pub(crate) const GRAY_CHROMA: u16 = 18;

/// Where the light block begins, and what closes it. The gate reads one
/// stylesheet as two palettes, so it has to know which lines belong to
/// which reading; a selector is the only marker CSS gives it.
const LIGHT_SELECTOR: &str = ":root[data-theme=\"light\"] {";
const BLOCK_END: &str = "\n}";

/// One of the two ways the client draws a page.
///
/// **The ramp is named for the page, never for the ink**: `g0` is the
/// page and `g10` is the surface furthest from it, in both readings.
/// What a mode fixes is which end of the lightness range the page sits
/// at, which is why each carries its own pair of ends and why a ramp is
/// judged for moving away from its page rather than for climbing.
#[derive(Clone, Copy)]
pub(crate) enum Mode {
    Dark,
    Light,
}

impl Mode {
    const ALL: [Mode; 2] = [Mode::Dark, Mode::Light];

    const fn name(self) -> &'static str {
        match self {
            Mode::Dark => "the dark page",
            Mode::Light => "the light page",
        }
    }

    /// The lightness of `g0`, which is the page itself.
    const fn page(self) -> u16 {
        match self {
            Mode::Dark => 145,
            Mode::Light => 978,
        }
    }

    /// The lightness of `g10`, the surface furthest from the page.
    ///
    /// The light page travels less far than the dark one, and the two
    /// numbers are not a symmetry that was broken. APCA charges dark ink
    /// on a light surface far more than the reverse, so the light page
    /// spends most of its range on the three surfaces that carry text
    /// and has less left for the fills below them - where a further rung
    /// would buy separation nothing needs.
    const fn far(self) -> u16 {
        match self {
            Mode::Dark => 930,
            Mode::Light => 250,
        }
    }
}

/// The stylesheet as one mode reads it: the shared declarations, with
/// every colour token the light block restates taken from that block.
///
/// Built as text rather than as a parsed table because every reader
/// below already parses text, and a second representation of the same
/// stylesheet is the second authority this gate exists to prevent.
pub(crate) fn reading(source: &str, mode: Mode) -> String {
    let Some((before, rest)) = source.split_once(LIGHT_SELECTOR) else {
        return source.to_owned();
    };
    let (light, after) = rest.split_once(BLOCK_END).unwrap_or((rest, ""));
    let shared = format!("{before}{after}");
    match mode {
        Mode::Dark => shared,
        Mode::Light => {
            let without_colour: String = shared
                .lines()
                .filter(|line| !line.trim_start().starts_with("--color-"))
                .collect::<Vec<_>>()
                .join("\n");
            format!("{light}\n{without_colour}")
        }
    }
}

pub(crate) fn check(root: &Path) -> Result<Vec<Violation>, XtaskError> {
    let mut violations = Vec::new();
    let theme_path = root.join(THEME);
    if !theme_path.is_file() {
        violations.push(Violation {
            gate: "color",
            location: THEME.to_owned(),
            rule: "the client's theme file is the sole production point for colour".to_owned(),
            violation: "the theme file is missing".to_owned(),
            alternative: format!("restore {THEME}"),
        });
        return Ok(violations);
    }
    let source = walk::read_text(&theme_path)?;
    for mode in Mode::ALL {
        violations.extend(judge_tokens(&reading(&source, mode), mode));
    }
    violations.extend(scan_for_literals(root)?);
    Ok(violations)
}

/// The seven assertions, in the order the gate table lists them, against
/// one mode's reading of the stylesheet.
fn judge_tokens(source: &str, mode: Mode) -> Vec<Violation> {
    let mut violations = Vec::new();
    let greys = grey_ramp(source);
    let colours = parse_colour_tokens(source);
    let named = |rule: &str, violation: String| {
        token_violation(rule, format!("{}: {violation}", mode.name()))
    };

    // 1. Eleven grey rungs, each further from the page than the last.
    if greys.len() != 11 {
        violations.push(named(
            "the grey ramp has eleven rungs",
            format!("found {}", greys.len()),
        ));
    }
    let away = mode.far() > mode.page();
    if !greys.windows(2).all(|pair| match pair {
        [(_, near), (_, far)] => {
            if away {
                far > near
            } else {
                far < near
            }
        }
        _ => true,
    }) {
        violations.push(named(
            "every rung of the grey ramp is further from the page than the one before it",
            "a rung turns back towards the page".to_owned(),
        ));
    }

    // 2. Two lightness rules, and they are not the same rule.
    //
    //    The grey ramp spans exactly the declared floor and ceiling: it is
    //    the information surface, and its ends are the contract.
    //
    //    Every token, grey or coloured, avoids pure black and pure white -
    //    that is what the ban is actually protecting against (glare, and
    //    smearing on OLED). Running the gate for the first time found that
    //    the theme states a ceiling of 930 while its own token table
    //    puts ALERT_HOVER at 945. Both shipped, so the ceiling is a property
    //    of the ramp and the interaction variants sit above it by design;
    //    reading it as a global bound would have made the table illegal.
    if let (Some(page), Some(far)) = (greys.first(), greys.last())
        && (page.1 != mode.page() || far.1 != mode.far())
    {
        violations.push(named(
            "the grey ramp spans exactly the declared page and furthest surface",
            format!("the ramp runs {} to {}", page.1, far.1),
        ));
    }
    let every_lightness = greys
        .iter()
        .map(|(name, lightness)| (name, *lightness))
        .chain(
            colours
                .iter()
                .map(|(name, lightness, _, _)| (name, *lightness)),
        );
    for (name, lightness) in every_lightness {
        if lightness == 0 || lightness >= 1000 {
            violations.push(named(
                "no pure black and no pure white (glare, and OLED smear)",
                format!("{name} is at {lightness} per mille"),
            ));
        }
    }

    // 3. Every coloured token sits on the axis or on its single exception.
    for (name, _, hue, _) in &colours {
        if *hue != HUE_AXIS && *hue != HUE_ALERT {
            violations.push(named(
                "one hue axis and one exception, which is its complement",
                format!("{name} sits on hue {hue}"),
            ));
        }
    }

    // 4. The exception hue really is the complement.
    if (HUE_AXIS + 180) % 360 != HUE_ALERT {
        violations.push(token_violation(
            "the exception hue is derived, not chosen",
            format!("{HUE_ALERT} is not the complement of {HUE_AXIS}"),
        ));
    }

    // 5. The grey ramp's chroma is the single axis value, rung by rung.
    //    Read off the rungs rather than off a constant's spelling: what the
    //    rule is about is the colour the client ships, and a stylesheet can
    //    state the constant correctly and then write a rung that departs
    //    from it.
    for (name, chroma) in grey_chromas(source) {
        if chroma != GRAY_CHROMA {
            violations.push(named(
                "the grey ramp carries the axis chroma",
                format!("{name} is at chroma {chroma} per mille, not {GRAY_CHROMA}"),
            ));
        }
    }

    // 6. Coloured tokens take a ratio, never a written chroma, and there are
    //    exactly two ratios in the library.
    let mut ratios: Vec<u16> = colours.iter().map(|(_, _, _, ratio)| *ratio).collect();
    ratios.sort_unstable();
    ratios.dedup();
    if ratios.len() != 2 {
        violations.push(named(
            "exactly two chroma ratios, never merged",
            format!("found {} distinct ratios", ratios.len()),
        ));
    }
    for name in colour_tokens_without_ratio(source) {
        violations.push(named(
            "a coloured token states the share of the gamut it takes",
            format!("{name} resolves a chroma and declares no `--ratio-` beside it"),
        ));
    }
    if colours.is_empty() {
        violations.push(named(
            "the coloured token table is readable",
            "no coloured token parsed out of the theme file".to_owned(),
        ));
    }

    // 7. Text reaches the contrast its own size demands.
    violations.extend(judge_readability(source, &greys, mode));
    violations
}

/// The seventh assertion: every text token reaches the tier it claims, and
/// every type step claims the tier its size and weight actually demand.
///
/// This one is here rather than in a browser because it never needed one.
/// The surfaces are a closed ladder and the tokens are a closed table, so
/// the pairs are enumerable and the judgement is a pure function - what the
/// architecture calls a missing end-to-end gate is, for contrast, a missing
/// table. It is checked against `TEXT_SURFACE_CEILING`, the brightest
/// surface text may sit on, because a token that passed on the page and
/// failed on a card would be one rule with two answers.
fn judge_readability(source: &str, greys: &[(String, u16)], mode: Mode) -> Vec<Violation> {
    let mut violations = Vec::new();
    let named = |rule: &str, violation: String| {
        token_violation(rule, format!("{}: {violation}", mode.name()))
    };
    let tokens = parse_text_tokens(source);
    let steps = parse_type_scale(source);
    if tokens.is_empty() || steps.is_empty() {
        violations.push(named(
            "the text token and type tables are readable",
            "TEXT_TOKENS or TYPE_SCALE parsed to nothing".to_owned(),
        ));
        return violations;
    }
    let Some(surface) = text_surface_ceiling(source).and_then(|name| {
        greys
            .iter()
            .find(|(rung, _)| *rung == name)
            .map(|(_, l)| *l)
    }) else {
        violations.push(named(
            "the surface furthest from the page that carries text is a rung of the ramp",
            "TEXT_SURFACE_CEILING names no rung of GRAY_RAMP".to_owned(),
        ));
        return violations;
    };

    for (name, lightness, claimed) in &tokens {
        let reached = apca_lc(*lightness, surface);
        if reached + 0.05 < f64::from(*claimed) {
            violations.push(named(
                "a text token reaches the tier it claims",
                format!("{name} claims Lc {claimed} and reaches {reached:.1}"),
            ));
        }
    }

    for (name, px, weight, claimed) in &steps {
        // A step called `body` is prose and takes the body column; every
        // other step is something that qualifies prose and takes the
        // content column. Bronze states different minimum sizes for the
        // two, and merging them would let a 15px note claim a tier only a
        // 15px sentence may claim.
        let derived = bronze_tier(*px, *weight, name == "body");
        match derived {
            None => violations.push(token_violation(
                "every type step is large enough for some tier to admit it",
                format!(
                    "{name} is {px}px at weight {weight}, below every Bronze minimum: \
                     no colour makes it legible"
                ),
            )),
            Some(tier) if tier != *claimed => violations.push(token_violation(
                "a type step claims the tier its size and weight demand",
                format!("{name} claims Lc {claimed} and demands Lc {tier}"),
            )),
            Some(_) => {}
        }
        if let Some(tier) = derived
            && !tokens.iter().any(|(_, _, claim)| *claim >= tier)
        {
            violations.push(token_violation(
                "some text token can serve every type step",
                format!("{name} demands Lc {tier} and no text token reaches it"),
            ));
        }
    }
    violations
}

fn token_violation(rule: &str, violation: String) -> Violation {
    Violation {
        gate: "color",
        location: THEME.to_owned(),
        rule: rule.to_owned(),
        violation,
        alternative: "adjust the client's theme file, and record the reason in \
                      xtask-SPEC.md section 8-8; colour rules are mechanical by design"
            .to_owned(),
    }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    reason = "test code"
)]
mod tests;
