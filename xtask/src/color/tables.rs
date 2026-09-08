// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Reading the tables of `web::theme`: the gate parses the source it
//! judges rather than keeping a copy of it.

use super::{GRAY_CHROMA, HUE_ALERT, HUE_AXIS};

/// `("TEXT", 928, 90),`
pub(super) fn parse_text_tokens(source: &str) -> Vec<(String, u16, u16)> {
    table_body(source, "pub const TEXT_TOKENS")
        .lines()
        .filter_map(|line| {
            let mut fields = row_fields(line)?;
            let name = fields.next()?.trim_matches('"').to_owned();
            let lightness = resolve(fields.next()?)?;
            let tier = resolve(fields.next()?)?;
            Some((name, lightness, tier))
        })
        .collect()
}

/// `("body", 14, 400, 90),`
pub(super) fn parse_type_scale(source: &str) -> Vec<(String, u16, u16, u16)> {
    table_body(source, "pub const TYPE_SCALE")
        .lines()
        .filter_map(|line| {
            let mut fields = row_fields(line)?;
            let name = fields.next()?.trim_matches('"').to_owned();
            let px = resolve(fields.next()?)?;
            let weight = resolve(fields.next()?)?;
            let tier = resolve(fields.next()?)?;
            Some((name, px, weight, tier))
        })
        .collect()
}

pub(super) fn text_surface_ceiling(source: &str) -> Option<String> {
    let after = source
        .split_once("pub const TEXT_SURFACE_CEILING")
        .map(|(_, rest)| rest)?;
    let quoted = after.split_once('"').map(|(_, rest)| rest)?;
    quoted.split_once('"').map(|(name, _)| name.to_owned())
}

/// `("G0", 145),`
pub(crate) fn grey_ramp(source: &str) -> Vec<(String, u16)> {
    table_body(source, "pub const GRAY_RAMP")
        .lines()
        .filter_map(|line| {
            let mut fields = row_fields(line)?;
            let name = fields.next()?.trim_matches('"').to_owned();
            let lightness = fields.next()?.parse().ok()?;
            Some((name, lightness))
        })
        .collect()
}

/// `("ACCENT", 680, HUE_AXIS, ACCENT_CHROMA_PERCENT),` - symbolic fields are
/// resolved against the constants this gate already knows, because a table
/// that spelled the numbers would defeat the point of the ratio.
pub(super) fn parse_colour_tokens(source: &str) -> Vec<(String, u16, u16, u16)> {
    table_body(source, "pub const COLOUR_TOKENS")
        .lines()
        .filter_map(|line| {
            let mut fields = row_fields(line)?;
            let name = fields.next()?.trim_matches('"').to_owned();
            let lightness = resolve(fields.next()?)?;
            let hue = resolve(fields.next()?)?;
            let ratio = resolve(fields.next()?)?;
            Some((name, lightness, hue, ratio))
        })
        .collect()
}

fn resolve(field: &str) -> Option<u16> {
    match field {
        "HUE_AXIS" => Some(HUE_AXIS),
        "HUE_ALERT" => Some(HUE_ALERT),
        "GRAY_CHROMA" => Some(GRAY_CHROMA),
        "ACCENT_CHROMA_PERCENT" => Some(90),
        "ALERT_CHROMA_PERCENT" => Some(55),
        other => other.parse().ok(),
    }
}

fn row_fields(line: &str) -> Option<impl Iterator<Item = &str>> {
    let trimmed = line.trim();
    let inner = trimmed.strip_prefix('(')?.split_once(')')?.0;
    Some(inner.split(',').map(str::trim).filter(|f| !f.is_empty()))
}

/// Takes the rows between `= [` and the closing bracket. The `= ` matters:
/// splitting on the first `[` would land inside the type annotation, which
/// is exactly the bug the first run of this gate found.
fn table_body<'a>(source: &'a str, marker: &str) -> &'a str {
    let Some(after) = source.split_once(marker).map(|(_, rest)| rest) else {
        return "";
    };
    let Some(open) = after.split_once("= [").map(|(_, rest)| rest) else {
        return "";
    };
    open.split_once("];").map_or("", |(body, _)| body)
}
