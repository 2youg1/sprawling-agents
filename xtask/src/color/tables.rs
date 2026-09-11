// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Reading the token block of `client/src/theme.css`: the gate parses the
//! stylesheet it judges rather than keeping a copy of it.
//!
//! Values are read at the precision the assertions use — a lightness or a
//! chroma as per mille, a hue, a tier and a weight as whole numbers. Per
//! mille is computed from the digit string rather than through a float, so
//! that the reading is the same on every machine and the parser hides no
//! rounding decision of its own.

/// A token's colour, split the way the assertions ask about it.
struct Oklch {
    lightness: u16,
    chroma: Chroma,
    hue: u16,
}

/// How a token gets its chroma. Exhaustive rather than an `Option<u16>`,
/// because the two cases are different kinds of token and the gate judges
/// them by different rules: a fixed chroma is on the grey axis, and a
/// scaled one takes a share of what the screen can show.
enum Chroma {
    /// Written out, as the grey ramp and the text tokens write it.
    Fixed(u16),
    /// `calc(<resolved> * var(--chroma))`, as a coloured token writes it.
    /// The share it was resolved from is `--ratio-<token>`.
    Scaled,
}

/// `("G0", 145)`, in the order the stylesheet declares them.
pub(crate) fn grey_ramp(source: &str) -> Vec<(String, u16)> {
    rungs(source)
        .into_iter()
        .map(|(name, colour)| (name, colour.lightness))
        .collect()
}

/// The chroma each rung actually carries. The single-axis rule is about
/// these values, so the gate reads them rather than a constant's spelling.
pub(super) fn grey_chromas(source: &str) -> Vec<(String, u16)> {
    rungs(source)
        .into_iter()
        .filter_map(|(name, colour)| match colour.chroma {
            Chroma::Fixed(chroma) => Some((name, chroma)),
            Chroma::Scaled => None,
        })
        .collect()
}

/// `("ACCENT", 680, 264, 90)`: name, lightness per mille, hue, and the
/// share of the displayable chroma it takes.
pub(super) fn parse_colour_tokens(source: &str) -> Vec<(String, u16, u16, u16)> {
    scaled_tokens(source)
        .into_iter()
        .filter_map(|(suffix, colour)| {
            let ratio = declaration(source, &format!("ratio-{suffix}")).and_then(whole)?;
            Some((canonical(&suffix), colour.lightness, colour.hue, ratio))
        })
        .collect()
}

/// The coloured tokens that do not say what share they take.
///
/// Reported separately rather than dropped, because a token missing from
/// the table it is judged by is invisible: the ratio assertions would pass
/// by counting one fewer token instead of naming the one that is silent.
pub(super) fn colour_tokens_without_ratio(source: &str) -> Vec<String> {
    scaled_tokens(source)
        .into_iter()
        .filter(|(suffix, _)| declaration(source, &format!("ratio-{suffix}")).is_none())
        .map(|(suffix, _)| canonical(&suffix))
        .collect()
}

/// `("TEXT", 928, 90)`: name, lightness per mille, and the APCA Lc it
/// claims to reach.
pub(super) fn parse_text_tokens(source: &str) -> Vec<(String, u16, u16)> {
    colour_declarations(source)
        .into_iter()
        .filter(|(suffix, _)| is_text(suffix))
        .filter_map(|(suffix, colour)| {
            let tier = declaration(source, &format!("tier-{suffix}")).and_then(whole)?;
            Some((canonical(&suffix), colour.lightness, tier))
        })
        .collect()
}

/// `("body", 15, 400, 90)`: name, size in px, weight, and the Lc it
/// demands. A step is a `--text-<name>` with a weight beside it; the three
/// declarations are read as one row because they are one decision.
pub(super) fn parse_type_scale(source: &str) -> Vec<(String, u16, u16, u16)> {
    declarations(source)
        .filter_map(|(name, value)| {
            let step = name.strip_prefix("text-").filter(|s| nameable(s))?;
            let px = whole(value)?;
            let weight = declaration(source, &format!("font-weight-{step}")).and_then(whole)?;
            let tier = declaration(source, &format!("tier-{step}")).and_then(whole)?;
            Some((step.to_owned(), px, weight, tier))
        })
        .collect()
}

/// The brightest surface text may sit on, as a rung name.
pub(super) fn text_surface_ceiling(source: &str) -> Option<String> {
    declaration(source, "surface-ceiling")
        .as_deref()
        .map(canonical)
}

/// The rungs of the grey ramp: `--color-g<n>`.
fn rungs(source: &str) -> Vec<(String, Oklch)> {
    colour_declarations(source)
        .into_iter()
        .filter(|(suffix, _)| is_rung(suffix))
        .map(|(suffix, colour)| (canonical(&suffix), colour))
        .collect()
}

/// The tokens whose chroma is a share rather than a number.
fn scaled_tokens(source: &str) -> Vec<(String, Oklch)> {
    colour_declarations(source)
        .into_iter()
        .filter(|(_, colour)| matches!(colour.chroma, Chroma::Scaled))
        .collect()
}

/// Every `--color-<name>` whose value is an `oklch()` call, paired with its
/// suffix as the stylesheet spells it.
fn colour_declarations(source: &str) -> Vec<(String, Oklch)> {
    declarations(source)
        .filter_map(|(name, value)| {
            let suffix = name.strip_prefix("color-").filter(|s| nameable(s))?;
            Some((suffix.to_owned(), oklch(value)?))
        })
        .collect()
}

fn is_rung(suffix: &str) -> bool {
    suffix
        .strip_prefix('g')
        .is_some_and(|digits| !digits.is_empty() && digits.bytes().all(|b| b.is_ascii_digit()))
}

fn is_text(suffix: &str) -> bool {
    suffix == "text" || suffix.starts_with("text-")
}

/// Tailwind's reset declarations (`--color-*: initial`) name no token.
fn nameable(suffix: &str) -> bool {
    !suffix.is_empty() && !suffix.contains('*')
}

/// The name every table, badge and document in this repository uses:
/// `accent-hover` is `ACCENT_HOVER`. Custom properties are lowercase by
/// convention; the colour library is not, and the gate reports the name a
/// reader will go looking for.
fn canonical(suffix: &str) -> String {
    suffix.replace('-', "_").to_uppercase()
}

/// The value of one declaration, wherever in the file it sits.
fn declaration(source: &str, name: &str) -> Option<String> {
    declarations(source)
        .find(|(found, _)| *found == name)
        .map(|(_, value)| value.to_owned())
}

/// Every `--name: value;` the stylesheet declares, in source order.
///
/// One line, one declaration: the multi-line values in this file are font
/// stacks and a gradient, and a value that does not finish on its own line
/// is not one of the values this gate judges.
fn declarations(source: &str) -> impl Iterator<Item = (&str, &str)> {
    source.lines().filter_map(|line| {
        let (name, tail) = line.trim().strip_prefix("--")?.split_once(':')?;
        let value = tail.split(';').next()?.trim();
        (!value.is_empty()).then_some((name.trim(), value))
    })
}

/// The three components of an `oklch(L C H)` value.
fn oklch(value: &str) -> Option<Oklch> {
    let inner = value.strip_prefix("oklch(")?.strip_suffix(')')?;
    let fields = fields(inner);
    let lightness = per_mille(fields.first()?)?;
    let chroma = match fields.get(1)? {
        scaled if scaled.starts_with("calc(") => Chroma::Scaled,
        fixed => Chroma::Fixed(per_mille(fixed)?),
    };
    Some(Oklch {
        lightness,
        chroma,
        hue: whole(fields.get(2)?)?,
    })
}

/// The whitespace-separated fields of a function's arguments, keeping a
/// nested call whole: `calc(0.151 * var(--chroma))` is one field, not
/// three.
fn fields(inner: &str) -> Vec<&str> {
    let mut found = Vec::new();
    let mut depth: usize = 0;
    let mut start: usize = 0;
    for (index, character) in inner.char_indices() {
        match character {
            '(' => depth = depth.saturating_add(1),
            ')' => depth = depth.saturating_sub(1),
            space if space.is_whitespace() && depth == 0 => {
                push_field(&mut found, inner.get(start..index));
                start = index.saturating_add(1);
            }
            _ => {}
        }
    }
    push_field(&mut found, inner.get(start..));
    found
}

fn push_field<'a>(found: &mut Vec<&'a str>, slice: Option<&'a str>) {
    if let Some(field) = slice.map(str::trim)
        && !field.is_empty()
    {
        found.push(field);
    }
}

/// `0.145` reads as 145. Fewer than three fractional digits pad with
/// zeros, which is what the notation means; more are not written here and
/// would be a precision this library does not use.
fn per_mille(field: &str) -> Option<u16> {
    let (units, fraction) = field.split_once('.').unwrap_or((field, ""));
    let mut thousandths: u16 = 0;
    for place in 0..3usize {
        let digit = fraction
            .as_bytes()
            .get(place)
            .and_then(|byte| char::from(*byte).to_digit(10))
            .unwrap_or(0);
        thousandths = thousandths
            .checked_mul(10)?
            .checked_add(u16::try_from(digit).ok()?)?;
    }
    units
        .parse::<u16>()
        .ok()?
        .checked_mul(1000)?
        .checked_add(thousandths)
}

/// A whole number, with the unit a stylesheet writes after it removed.
fn whole(field: impl AsRef<str>) -> Option<u16> {
    field.as_ref().trim_end_matches("px").parse().ok()
}
