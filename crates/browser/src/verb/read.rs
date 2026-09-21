// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Reading one call's arguments.
//!
//! Separate from the frames they become because the two answer to
//! different readers: this is the grammar the model writes against and
//! the schema in `crate::browser_tool` describes, while `verb.rs` owns
//! what a frame looks like. A ninth action is a change here and in
//! `Verb`, and the compiler points at both.

use kernel::{AxCode, AxError, Payload};
use serde_json::Value;

use crate::act::{Action, Origin, Point, STEPS_MAX};

pub(super) fn missing(field: &str) -> AxError {
    AxError::failure(
        AxCode::InvalidArgs,
        "read a browser action",
        format!("no `{field}`"),
    )
    .with_recovery(format!("pass `{field}`"))
}

pub(super) fn text_of(args: &Payload, field: &str) -> Result<String, AxError> {
    args.as_map()
        .get(field)
        .and_then(Value::as_str)
        .map(str::to_owned)
        .ok_or_else(|| missing(field))
}

pub(super) fn number_of(args: &Payload, field: &str) -> Result<u64, AxError> {
    args.as_map()
        .get(field)
        .and_then(Value::as_u64)
        .ok_or_else(|| missing(field))
}

pub(super) fn side_of(args: &Payload, field: &str) -> Result<u32, AxError> {
    u32::try_from(number_of(args, field)?).map_err(|_| {
        AxError::failure(
            AxCode::InvalidArgs,
            "read a browser action",
            format!("`{field}` is larger than a viewport can be"),
        )
        .with_recovery("pass a pixel count that fits in 32 bits")
    })
}

pub(super) fn read_action(args: &Payload) -> Result<Action, AxError> {
    match text_of(args, "kind")?.as_str() {
        "click" => Ok(Action::Click {
            reference: text_of(args, "ref")?,
        }),
        "read" => Ok(Action::Read {
            reference: text_of(args, "ref")?,
        }),
        "type" => Ok(Action::Type {
            reference: text_of(args, "ref")?,
            text: text_of(args, "text")?,
        }),
        "drag" => Ok(Action::Drag {
            from: read_origin(args)?,
            to: point_of(args, "to")?,
            steps: steps_of(args)?,
        }),
        "scroll" => Ok(Action::Scroll {
            at: optional_point(args, "at")?,
            by: point_of(args, "to")?,
        }),
        other => Err(AxError::failure(
            AxCode::InvalidArgs,
            "read a browser action",
            other.to_owned(),
        )
        .with_recovery("one of click, type, read, drag, scroll")),
    }
}

/// Where a drag starts: a reference, or a point, and never both and
/// never neither. The desktop connector's `desktop.act` reads its window
/// actions the same way, so one gesture has one vocabulary.
fn read_origin(args: &Payload) -> Result<Origin, AxError> {
    let reference = args.as_map().get("ref").and_then(Value::as_str);
    let point = match args.as_map().get("point") {
        Some(value) => Some(point_value(value)?),
        None => None,
    };
    match (reference, point) {
        (Some(reference), None) => Ok(Origin::Reference(reference.to_owned())),
        (None, Some(point)) => Ok(Origin::Point(point)),
        (Some(_), Some(_)) => Err(AxError::failure(
            AxCode::InvalidArgs,
            "read a browser action",
            "both `ref` and `point` name the drag's start",
        )
        .with_recovery("pass one of them: `ref` from the snapshot, or `point` in viewport pixels")),
        (None, None) => Err(AxError::failure(
            AxCode::InvalidArgs,
            "read a browser action",
            "a drag names nowhere to start",
        )
        .with_recovery("pass `ref` from the snapshot, or `point` in viewport pixels")),
    }
}

fn point_of(args: &Payload, field: &str) -> Result<Point, AxError> {
    let value = args.as_map().get(field).ok_or_else(|| missing(field))?;
    point_value(value)
}

fn optional_point(args: &Payload, field: &str) -> Result<Option<Point>, AxError> {
    match args.as_map().get(field) {
        Some(value) => point_value(value).map(Some),
        None => Ok(None),
    }
}

/// `{x, y}` in whole pixels. A delta may be negative — a wheel turned
/// up — so both sides are signed.
fn point_value(value: &Value) -> Result<Point, AxError> {
    let object = value.as_object().ok_or_else(|| {
        AxError::failure(
            AxCode::InvalidArgs,
            "read a browser action",
            "a point that is not an object",
        )
        .with_recovery("pass `{x: <integer>, y: <integer>}`")
    })?;
    let side = |name: &str| {
        object.get(name).and_then(Value::as_i64).ok_or_else(|| {
            AxError::failure(
                AxCode::InvalidArgs,
                "read a browser action",
                format!("a point with no whole `{name}`"),
            )
            .with_recovery("pass `{x: <integer>, y: <integer>}`")
        })
    };
    Ok(Point {
        x: side("x")?,
        y: side("y")?,
    })
}

/// How many intermediate pointer moves a drag makes. One is the floor
/// and [`STEPS_MAX`] the ceiling: a drag that needs more than the
/// ceiling is a gesture a page's own listener cannot follow either.
fn steps_of(args: &Payload) -> Result<u32, AxError> {
    let Some(raw) = args.as_map().get("steps") else {
        return Ok(1);
    };
    let number = raw.as_u64().ok_or_else(|| {
        AxError::failure(
            AxCode::InvalidArgs,
            "read a browser action",
            "`steps` that is not a whole number",
        )
        .with_recovery("pass 1 or more intermediate pointer moves")
    })?;
    let steps = u32::try_from(number).map_err(|_| {
        AxError::failure(
            AxCode::InvalidArgs,
            "read a browser action",
            format!("`steps` {number} is larger than a drag can make"),
        )
        .with_recovery(format!("pass between 1 and {STEPS_MAX}"))
    })?;
    if steps == 0 || steps > STEPS_MAX {
        return Err(AxError::failure(
            AxCode::InvalidArgs,
            "read a browser action",
            format!("`steps` {steps} is outside the drag's range"),
        )
        .with_recovery(format!("pass between 1 and {STEPS_MAX}")));
    }
    Ok(steps)
}
