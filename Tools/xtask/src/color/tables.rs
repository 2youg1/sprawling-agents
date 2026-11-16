// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

struct Oklch {
    lightness: u16,
    chroma: Chroma,
    hue: u16,
}

enum Chroma {
    Fixed(u16),

    Scaled,
}

pub(crate) fn grey_ramp(source: &str) -> Vec<(String, u16)> {
    rungs(source)
        .into_iter()
        .map(|(name, colour)| (name, colour.lightness))
        .collect()
}

pub(super) fn grey_chromas(source: &str) -> Vec<(String, u16)> {
    rungs(source)
        .into_iter()
        .filter_map(|(name, colour)| match colour.chroma {
            Chroma::Fixed(chroma) => Some((name, chroma)),
            Chroma::Scaled => None,
        })
        .collect()
}

pub(super) fn parse_colour_tokens(source: &str) -> Vec<(String, u16, u16, u16)> {
    scaled_tokens(source)
        .into_iter()
        .filter_map(|(suffix, colour)| {
            let ratio = declaration(source, &format!("ratio-{suffix}")).and_then(whole)?;
            Some((canonical(&suffix), colour.lightness, colour.hue, ratio))
        })
        .collect()
}

pub(super) fn colour_tokens_without_ratio(source: &str) -> Vec<String> {
    scaled_tokens(source)
        .into_iter()
        .filter(|(suffix, _)| declaration(source, &format!("ratio-{suffix}")).is_none())
        .map(|(suffix, _)| canonical(&suffix))
        .collect()
}

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

pub(super) fn text_surface_ceiling(source: &str) -> Option<String> {
    declaration(source, "surface-ceiling")
        .as_deref()
        .map(canonical)
}

pub(super) fn tier_slack(source: &str) -> Option<f64> {
    declaration(source, "tier-slack")?
        .parse::<f64>()
        .ok()
        .filter(|slack| slack.is_finite() && *slack >= 0.0)
}

fn rungs(source: &str) -> Vec<(String, Oklch)> {
    colour_declarations(source)
        .into_iter()
        .filter(|(suffix, _)| is_rung(suffix))
        .map(|(suffix, colour)| (canonical(&suffix), colour))
        .collect()
}

fn scaled_tokens(source: &str) -> Vec<(String, Oklch)> {
    colour_declarations(source)
        .into_iter()
        .filter(|(_, colour)| matches!(colour.chroma, Chroma::Scaled))
        .collect()
}

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

fn nameable(suffix: &str) -> bool {
    !suffix.is_empty() && !suffix.contains('*')
}

fn canonical(suffix: &str) -> String {
    suffix.replace('-', "_").to_uppercase()
}

fn declaration(source: &str, name: &str) -> Option<String> {
    declarations(source)
        .find(|(found, _)| *found == name)
        .map(|(_, value)| value.to_owned())
}

fn declarations(source: &str) -> impl Iterator<Item = (&str, &str)> {
    source.lines().filter_map(|line| {
        let (name, tail) = line.trim().strip_prefix("--")?.split_once(':')?;
        let value = tail.split(';').next()?.trim();
        (!value.is_empty()).then_some((name.trim(), value))
    })
}

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

fn whole(field: impl AsRef<str>) -> Option<u16> {
    field.as_ref().trim_end_matches("px").parse().ok()
}
