// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use serde_json::Value;

use super::encode::{self, Format, Wanted};
use super::geometry::Inset;
use super::keys::Modifier;
use crate::refusal::{Refusal, RefusalCode};

const NOTCHES_BY_DEFAULT: i32 = -3;

pub(super) fn text<'a>(arguments: &'a Value, field: &str) -> Option<&'a str> {
    arguments.get(field).and_then(Value::as_str)
}

pub(super) fn held(arguments: &Value) -> Result<Vec<Modifier>, Refusal> {
    let Some(named) = arguments.get("modifiers") else {
        return Ok(Vec::new());
    };
    let Some(listed) = named.as_array() else {
        return Err(not_a_modifier("`modifiers` is not a list"));
    };
    listed
        .iter()
        .map(|one| {
            one.as_str()
                .ok_or_else(|| not_a_modifier("a modifier in the list is not a name"))
                .and_then(Modifier::parse)
        })
        .collect()
}

fn not_a_modifier(subject: &str) -> Refusal {
    Refusal::new(
        RefusalCode::InvalidArgs,
        "act on a window",
        subject.to_owned(),
        "send `modifiers` as an array of ctrl, alt, shift or win",
    )
}

pub(super) fn inset(arguments: &Value, field: &str) -> Option<Inset> {
    let at = arguments.get(field)?;
    Some(Inset {
        x: whole_signed(at, "x")?,
        y: whole_signed(at, "y")?,
    })
}

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

pub(super) fn generation(arguments: &Value) -> Result<Option<u64>, Refusal> {
    match arguments.get("generation") {
        None | Some(Value::Null) => Ok(None),
        Some(given) => given
            .as_u64()
            .map(Some)
            .ok_or_else(|| not_whole("generation")),
    }
}

pub(super) fn asked_for(arguments: &Value) -> Result<Wanted, Refusal> {
    let format = match text(arguments, "format") {
        Some(named) => Format::parse(named)?,
        None => encode::DEFAULT_FORMAT,
    };
    Ok(Wanted {
        format,
        scale: match whole(arguments, "scale")? {
            None => encode::DEFAULT_SCALE,
            Some(asked) if asked == 0 || asked > encode::SCALE_MAX => {
                return Err(outside_domain("scale", asked, 1, encode::SCALE_MAX));
            }
            Some(asked) => asked,
        },
        quality: match whole(arguments, "quality")? {
            None => encode::DEFAULT_QUALITY,
            Some(asked) => admitted_quality(asked)?,
        },
    })
}

fn admitted_quality(asked: u32) -> Result<u8, Refusal> {
    kernel::consts_policy::IMAGE_QUALITY
        .admit(asked)
        .map_err(|refused| {
            Refusal::new(
                RefusalCode::InvalidArgs,
                refused.action(),
                refused.subject(),
                refused.recovery(),
            )
        })?;
    u8::try_from(asked).map_err(|_| {
        Refusal::new(
            RefusalCode::InvalidArgs,
            "encode a picture",
            format!("quality {asked} is not a byte"),
            "pass a quality the tool's schema describes",
        )
    })
}

fn outside_domain(field: &str, asked: u32, low: u32, high: u32) -> Refusal {
    let subject = format!("`{field}` {asked} is outside {low}..={high}");
    let recovery =
        format!("pass `{field}` between {low} and {high}, as this tool's schema describes");
    Refusal::new(
        RefusalCode::InvalidArgs,
        "use the desktop",
        subject,
        &recovery,
    )
}

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

pub(super) fn notches(arguments: &Value) -> i32 {
    arguments
        .get("to")
        .and_then(|to| whole_signed(to, "y"))
        .unwrap_or(NOTCHES_BY_DEFAULT)
}

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

    #[test]
    fn a_number_that_is_present_and_unusable_is_refused_rather_than_defaulted() {
        for wrong in [json!(-4), json!("100"), json!(1.5), json!(null)] {
            let arguments = json!({ "scale": wrong });
            if arguments["scale"].is_null() {

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

        assert!(whole(&json!({ "quality": -1 }), "quality").is_err());
        let refused = asked_for(&json!({ "quality": 900 })).unwrap_err();
        assert_eq!(
            refused.as_error()["data"]["code"],
            json!(RefusalCode::InvalidArgs.code().as_str())
        );

        assert!(asked_for(&json!({ "scale": 0 })).is_err());
        assert!(asked_for(&json!({ "scale": encode::SCALE_MAX + 1 })).is_err());
        assert_eq!(
            asked_for(&json!({ "scale": encode::SCALE_MAX }))
                .unwrap()
                .scale,
            encode::SCALE_MAX
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
