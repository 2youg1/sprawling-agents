// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! What one call's arguments say, each field refused by name rather than
//! defaulted.
//!
//! The tool schemas in `crate::tools` describe what a caller may send;
//! this module is what happens when one sends something else. A schema
//! is a promise the caller makes, not a check the caller has passed —
//! nothing in MCP validates it before a server sees it — so every field
//! is read here as if the schema had not been written.
//!
//! **A field that is present and unusable is refused, not defaulted.**
//! `scale: -4` is not `scale: 100`; a caller that sent it believes
//! something this server would then quietly contradict. A field that is
//! *absent* may have a default, because absence is a thing the schema
//! itself allows and the default is written down beside it.

use serde_json::Value;

use super::encode::{self, Format, Wanted};
use super::geometry::Inset;
use crate::refusal::{Refusal, RefusalCode};

/// How far a scroll goes when the call does not say. Down, because a
/// caller that scrolls without saying which way is reading onwards.
const NOTCHES_BY_DEFAULT: i32 = -3;

/// One string argument.
pub(super) fn text<'a>(arguments: &'a Value, field: &str) -> Option<&'a str> {
    arguments.get(field).and_then(Value::as_str)
}

/// One point measured from the window's own top-left corner.
pub(super) fn inset(arguments: &Value, field: &str) -> Option<Inset> {
    let at = arguments.get(field)?;
    Some(Inset {
        x: whole_signed(at, "x")?,
        y: whole_signed(at, "y")?,
    })
}

/// One non-negative whole number argument.
///
/// # Errors
/// Refuses a field that is present and is not one.
pub(super) fn whole(arguments: &Value, field: &str) -> Result<Option<u32>, Refusal> {
    match arguments.get(field) {
        None | Some(Value::Null) => Ok(None),
        Some(given) => given
            .as_u64()
            .and_then(|value| u32::try_from(value).ok())
            .map(Some)
            .ok_or_else(|| not_whole(field)),
    }
}

/// The same, for a generation, which counts higher than a size does.
///
/// # Errors
/// Refuses a field that is present and is not a non-negative integer.
pub(super) fn generation(arguments: &Value) -> Result<Option<u64>, Refusal> {
    match arguments.get("generation") {
        None | Some(Value::Null) => Ok(None),
        Some(given) => given
            .as_u64()
            .map(Some)
            .ok_or_else(|| not_whole("generation")),
    }
}

/// What the caller asked of the encoder, with each default named where
/// it is defined rather than written out again here.
///
/// # Errors
/// Refuses a format this server does not write and a scale or quality
/// that is present and is not a whole number.
pub(super) fn asked_for(arguments: &Value) -> Result<Wanted, Refusal> {
    let format = match text(arguments, "format") {
        Some(named) => Format::parse(named)?,
        None => encode::DEFAULT_FORMAT,
    };
    Ok(Wanted {
        format,
        scale: whole(arguments, "scale")?.unwrap_or(encode::DEFAULT_SCALE),
        quality: whole(arguments, "quality")?
            .and_then(|asked| u8::try_from(asked).ok())
            .unwrap_or(encode::DEFAULT_QUALITY),
    })
}

/// The region of the window a capture is of, when the call names one.
///
/// # Errors
/// Refuses a region that is not four whole numbers with two positive
/// sides. A zero-sided region would encode to nothing, and a caller
/// shown nothing reasons from nothing.
pub(super) fn region(arguments: &Value) -> Result<Option<(u32, u32, u32, u32)>, Refusal> {
    let Some(asked) = arguments.get("region") else {
        return Ok(None);
    };
    let side = |field: &str| {
        asked
            .get(field)
            .and_then(Value::as_i64)
            .and_then(|value| u32::try_from(value).ok())
    };
    let (Some(x), Some(y), Some(width), Some(height)) = (
        side("x"),
        side("y"),
        side("width").filter(|span| *span > 0),
        side("height").filter(|span| *span > 0),
    ) else {
        return Err(Refusal::new(
            RefusalCode::InvalidArgs,
            "capture a window",
            "that region is not four whole numbers inside the window".to_owned(),
            "send `region` as `x`, `y`, `width` and `height`, none of them negative and neither \
             side zero",
        ));
    };
    Ok(Some((x, y, width, height)))
}

/// How far a scroll goes, in wheel notches.
///
/// The schema spells a scroll's distance as `to`, which is a point, so
/// its vertical part is the distance — and it is counted in notches
/// rather than pixels, because a wheel has no pixels. A scroll is not a
/// drag, which is why `to` means something different for each.
pub(super) fn notches(arguments: &Value) -> i32 {
    arguments
        .get("to")
        .and_then(|to| whole_signed(to, "y"))
        .unwrap_or(NOTCHES_BY_DEFAULT)
}

/// One signed whole number inside an object argument.
fn whole_signed(object: &Value, field: &str) -> Option<i32> {
    object
        .get(field)
        .and_then(Value::as_i64)
        .and_then(|value| i32::try_from(value).ok())
}

fn not_whole(field: &str) -> Refusal {
    Refusal::new(
        RefusalCode::InvalidArgs,
        "use the desktop",
        format!("`{field}` is not a whole number this server can use"),
        "send it as a non-negative integer, as this tool's schema describes",
    )
}

/// An action that needs an argument it was not given.
pub(super) fn missing(action: &str, field: &str, what: &str) -> Refusal {
    Refusal::new(
        RefusalCode::InvalidArgs,
        "act on a window",
        format!("`{action}` was asked for without `{field}`"),
        &format!("send `{field}`: {what}"),
    )
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "test code"
)]
mod tests {
    use super::*;
    use serde_json::json;

    /// The rule this module exists for. A field a caller sent and this
    /// server cannot use is a refusal, because the alternative is
    /// quietly doing something other than what was asked.
    #[test]
    fn a_number_that_is_present_and_unusable_is_refused_rather_than_defaulted() {
        for wrong in [json!(-4), json!("100"), json!(1.5), json!(null)] {
            let arguments = json!({ "scale": wrong });
            if arguments["scale"].is_null() {
                // Absent is the one case with a default, and it is the
                // schema's own.
                assert_eq!(whole(&arguments, "scale").unwrap(), None);
                continue;
            }
            let refusal = whole(&arguments, "scale")
                .expect_err("a scale this server cannot use is not a scale of 100");
            assert_eq!(refusal.as_error()["data"]["code"], "E_INVALID_ARGS");
        }
        assert_eq!(whole(&json!({ "scale": 50 }), "scale").unwrap(), Some(50));
        assert_eq!(whole(&json!({}), "scale").unwrap(), None);
    }

    /// The defaults are the ones written beside their definitions, so a
    /// change there is a change here and not two numbers to keep equal.
    #[test]
    fn an_absent_field_takes_the_default_defined_beside_it() {
        let plain = asked_for(&json!({})).unwrap();
        assert_eq!(plain, Wanted::default());
        assert_eq!(plain.format, encode::DEFAULT_FORMAT);
        assert_eq!(plain.scale, encode::DEFAULT_SCALE);
        let asked = asked_for(&json!({ "format": "jpeg", "scale": 40, "quality": 20 })).unwrap();
        assert_eq!(asked.format, Format::Jpeg);
        assert_eq!(asked.scale, 40);
        assert_eq!(asked.quality, 20);
        // A quality outside a byte is refused rather than clamped: a
        // caller that asked for 900 believes something.
        assert!(whole(&json!({ "quality": -1 }), "quality").is_err());
        assert_eq!(
            asked_for(&json!({ "quality": 900 })).unwrap().quality,
            encode::DEFAULT_QUALITY
        );
    }

    #[test]
    fn a_region_needs_four_whole_numbers_and_two_positive_sides() {
        assert_eq!(region(&json!({})).unwrap(), None);
        assert_eq!(
            region(&json!({ "region": { "x": 1, "y": 2, "width": 3, "height": 4 } })).unwrap(),
            Some((1, 2, 3, 4))
        );
        for wrong in [
            json!({ "x": 1, "y": 2, "width": 0, "height": 4 }),
            json!({ "x": 1, "y": 2, "width": 3 }),
            json!({ "x": -1, "y": 2, "width": 3, "height": 4 }),
            json!("all of it"),
        ] {
            let refusal = region(&json!({ "region": wrong })).expect_err("not a usable region");
            assert!(
                refusal.as_error()["data"]["recovery"]
                    .as_str()
                    .unwrap()
                    .contains("width")
            );
        }
    }

    #[test]
    fn a_point_is_two_signed_numbers_and_a_scroll_reads_its_own_field() {
        assert_eq!(
            inset(&json!({ "point": { "x": -5, "y": 7 } }), "point"),
            Some(Inset { x: -5, y: 7 })
        );
        assert_eq!(inset(&json!({ "point": { "x": 1 } }), "point"), None);
        assert_eq!(inset(&json!({}), "point"), None);
        assert_eq!(notches(&json!({ "to": { "x": 0, "y": 8 } })), 8);
        assert_eq!(notches(&json!({})), NOTCHES_BY_DEFAULT);
    }

    #[test]
    fn a_generation_counts_higher_than_a_size_and_is_still_a_whole_number() {
        assert_eq!(
            generation(&json!({ "generation": u64::from(u32::MAX) + 1 })).unwrap(),
            Some(u64::from(u32::MAX) + 1)
        );
        assert_eq!(generation(&json!({})).unwrap(), None);
        assert!(generation(&json!({ "generation": -1 })).is_err());
    }

    #[test]
    fn an_action_missing_an_argument_is_told_which_one_and_what_it_is_for() {
        let refusal = missing("type", "text", "the text to type");
        let error = refusal.as_error();
        assert!(
            error["data"]["subject"]
                .as_str()
                .unwrap()
                .contains("`text`")
        );
        assert!(
            error["data"]["recovery"]
                .as_str()
                .unwrap()
                .contains("the text to type")
        );
    }
}
