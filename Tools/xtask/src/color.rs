// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

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

const CHECKPOINTS: [(&str, u16); 2] = [("REMINDER_FIRST", 150), ("REMINDER_SECOND", 25)];

const LIGHT_SELECTOR: &str = ":root[data-theme=\"light\"] {";
const BLOCK_END: &str = "\n}";

const RAMP: &str = "--color-g0:";

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

    const fn page(self) -> u16 {
        match self {
            Mode::Dark => 145,
            Mode::Light => 978,
        }
    }

    const fn far(self) -> u16 {
        match self {
            Mode::Dark => 930,
            Mode::Light => 250,
        }
    }

    const fn opens(self) -> &'static str {
        match self {
            Mode::Dark => RAMP,
            Mode::Light => LIGHT_SELECTOR,
        }
    }
}

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

    violations.extend(placed(
        roles::judge_roles(root, &source)?,
        theme.declaring(RAMP),
    ));
    violations.extend(scan_for_literals(root)?);
    violations.extend(disabled::judge_disabled_ink(root)?);
    Ok(violations)
}

fn judge_tokens(source: &str, mode: Mode) -> Vec<Violation> {
    let mut violations = Vec::new();
    let greys = grey_ramp(source);
    let colours = parse_colour_tokens(source);
    let named = |rule: &str, violation: String| {
        token_violation(rule, format!("{}: {violation}", mode.name()))
    };

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

    if (HUE_AXIS + 180) % 360 != HUE_ALERT {
        violations.push(token_violation(
            "the exception hue is derived, not chosen",
            format!("{HUE_ALERT} is not the complement of {HUE_AXIS}"),
        ));
    }

    for (name, chroma) in grey_chromas(source) {
        if chroma != GRAY_CHROMA {
            violations.push(named(
                "the grey ramp carries the axis chroma",
                format!("{name} is at chroma {chroma} per mille, not {GRAY_CHROMA}"),
            ));
        }
    }

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

    violations.extend(judge_readability(source, mode));
    violations
}

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
                      Tools/xtask/Spec.lean §8-8; colour rules are mechanical by design"
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
