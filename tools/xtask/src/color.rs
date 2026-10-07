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
//! that client's theme. The browser client's theme is an entry and the
//! parts it imports (`crate::theme`); the token assertions read it as one
//! stylesheet, and the scan's table of production points also holds the
//! playback stylesheet source.
//!
//! **The seven token assertions read the stylesheet, not a Rust table.**
//! Three of them ask what a resolved `oklch()` value was resolved *from* -
//! the share of the gamut a token takes, the APCA tier it claims, the
//! brightest surface text may sit on - and none of the three is a colour
//! component. They are declared beside the values they govern, as
//! properties the cascade ignores (tools/xtask/Spec.lean §8-8).

use std::path::Path;

use crate::report::{Violation, XtaskError};
use crate::theme::{self, Theme};

mod contrast;
mod disabled;
mod glass;
mod readability;
mod roles;
mod scan;
mod tables;

use tables::grey_ramp;

use readability::judge_readability;
use scan::scan_for_literals;
use tables::{colour_tokens_without_ratio, grey_chromas, parse_colour_tokens};

const HUE_AXIS: u16 = 250;
const HUE_ALERT: u16 = 70;
const GRAY_CHROMA: u16 = 14;

/// The two coloured tokens that may leave the axis, each with the one hue
/// it may take: the context ring's first reminder and its handoff
/// reminder (tools/xtask/Spec.lean §8-8b). Named rather than counted, so a
/// third token cannot borrow either hue.
const CHECKPOINTS: [(&str, u16); 2] = [("REMINDER_FIRST", 150), ("REMINDER_SECOND", 25)];

/// Where the light block begins, and what closes it. The gate reads one
/// stylesheet as two palettes, so it has to know which lines belong to
/// which reading; a selector is the only marker CSS gives it.
const LIGHT_SELECTOR: &str = ":root[data-theme=\"light\"] {";
const BLOCK_END: &str = "\n}";

/// What the dark reading is built on: the first rung of the ramp. The
/// part that declares it is where a finding about the token table is
/// reported, and the part holding `LIGHT_SELECTOR` for the light reading.
const RAMP: &str = "--color-g0:";

/// One of the two ways the client draws a page.
///
/// **The ramp is named for the page, never for the ink**: `g0` is the
/// page and `g10` is the surface furthest from it, in both readings.
/// What a mode fixes is which end of the lightness range the page sits
/// at, which is why each carries its own pair of ends and why a ramp is
/// judged for moving away from its page rather than for climbing.
#[derive(Clone, Copy)]
enum Mode {
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

    /// What the theme writes where this reading's token table begins.
    const fn opens(self) -> &'static str {
        match self {
            Mode::Dark => RAMP,
            Mode::Light => LIGHT_SELECTOR,
        }
    }
}

/// The stylesheet as one mode reads it: the shared declarations, with
/// every colour token the light block restates taken from that block.
///
/// Built as text rather than as a parsed table because every reader
/// below already parses text, and a second representation of the same
/// stylesheet is the second authority this gate exists to prevent.
fn reading(source: &str, mode: Mode) -> String {
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
    if !root.join(theme::ENTRY).is_file() {
        violations.push(Violation {
            gate: "color",
            location: theme::ENTRY.to_owned(),
            rule: "the client's theme is the sole production point for colour".to_owned(),
            violation: "the theme's entry is missing".to_owned(),
            alternative: format!("restore {}", theme::ENTRY),
        });
        return Ok(violations);
    }
    let theme = Theme::read(root)?;
    let source = theme.inlined();
    for mode in Mode::ALL {
        let read = reading(&source, mode);
        violations.extend(placed(
            judge_tokens(&read, mode),
            theme.declaring(mode.opens()),
        ));
        violations.extend(placed(
            glass::judge_glass(&source, &read, mode),
            theme.declaring(glass::OPACITY),
        ));
    }
    // Roles are judged as written: the hop is what serves both lightings,
    // and a mode's reading no longer holds the hops.
    violations.extend(placed(
        roles::judge_roles(root, &source)?,
        theme.declaring(RAMP),
    ));
    violations.extend(scan_for_literals(root)?);
    violations.extend(disabled::judge_disabled_ink(root)?);
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

    // 3. Every coloured token sits on the axis or on its single exception,
    //    except the two checkpoints, which sit on their own hues and only
    //    there.
    for (name, _, hue, _) in &colours {
        let allowed = match CHECKPOINTS
            .iter()
            .find(|(token, _)| *token == name.as_str())
        {
            Some((_, own)) => hue == own,
            None => *hue == HUE_AXIS || *hue == HUE_ALERT,
        };
        if !allowed {
            violations.push(named(
                "one hue axis and its complement, and each context-ring checkpoint on its own hue",
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

/// The token, glass and role assertions judge the theme as one text and
/// name its entry; this names the part that declares what they judged.
/// A finding that already names its own file, such as a rung spelled in a
/// view, keeps it.
fn placed(found: Vec<Violation>, part: &str) -> impl Iterator<Item = Violation> {
    found.into_iter().map(move |found| {
        if found.location == theme::ENTRY {
            Violation {
                location: part.to_owned(),
                ..found
            }
        } else {
            found
        }
    })
}

fn token_violation(rule: &str, violation: String) -> Violation {
    Violation {
        gate: "color",
        location: theme::ENTRY.to_owned(),
        rule: rule.to_owned(),
        violation,
        alternative: "adjust the client's theme, and record the reason in \
                      tools/xtask/Spec.lean §8-8; colour rules are mechanical by design"
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
