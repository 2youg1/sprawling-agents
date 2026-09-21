// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! BiDi `input`: pointer and wheel actions, which are a different module
//! of the protocol from the `script` calls most of this crate sends.
//!
//! A pointer action has to say where it starts, and BiDi names an
//! element origin by the page's own shared id rather than by a selector.
//! So an element-origin drag is two frames on the wire: this crate asks
//! the page for the element ([`crate::act::resolve_frame`]) and builds
//! the input frame from the reply, which [`shared_id_of`] reads.
//!
//! The vocabulary is the desktop connector's, deliberately: `drag` goes
//! from a reference or a point to a point, and `scroll` moves by a
//! delta. One action with two homes would drift the day either side was
//! corrected, so the field names and the meaning of `to` are the same on
//! both sides (`desktop-SPEC.md` section 8-4 and `browser-SPEC.md`
//! section 19-5).

use kernel::{AxCode, AxError};
use serde_json::{Map, Value, json};

use crate::act::{Point, STEPS_MAX};
use crate::port::Frame;
use crate::session::{ContextId, Session};

/// One drag: where it starts (already resolved), where it ends, and how
/// many intermediate moves it makes. The three travel together because
/// they are one gesture, and half a gesture is not a frame.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DragPath {
    pub from: Origin,
    pub to: Point,
    pub steps: u32,
}

/// A resolved origin, in the shape `input.performActions` takes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Origin {
    /// A page element, by the shared id its own realm gave it.
    Element(String),
    /// A viewport point.
    Viewport(Point),
}

/// How deep `shared_id_of` looks for the id. A remote value nests a
/// bounded number of levels; the cap is what keeps the search total.
const SHARED_ID_DEPTH_MAX: u8 = 8;

/// One pointer drag: press at `origin`, move to `to`, release.
///
/// `steps` is how many intermediate moves the pointer makes when the
/// origin is a viewport point. An element origin has no known viewport
/// position here — that is what the resolve frame was for — so it makes
/// the single move to `to`.
///
/// # Errors
/// Refuses a step count outside `1..=STEPS_MAX`, and propagates the
/// frame's own refusal.
pub fn pointer_frame(
    session: &mut Session,
    context: &ContextId,
    path: &DragPath,
) -> Result<Frame, AxError> {
    let (origin, to, steps) = (&path.from, path.to, path.steps);
    if steps == 0 || steps > STEPS_MAX {
        return Err(AxError::failure(
            AxCode::InvalidArgs,
            "drag on a page",
            format!("{steps} steps"),
        )
        .with_recovery(format!(
            "pass between 1 and {STEPS_MAX} intermediate pointer moves"
        )));
    }
    let mut actions: Vec<Value> = Vec::new();
    match origin {
        Origin::Element(shared_id) => {
            actions.push(json!({
                "type": "pointerMove",
                "x": 0,
                "y": 0,
                "origin": { "type": "element", "element": { "sharedId": shared_id } },
            }));
        }
        Origin::Viewport(from) => {
            actions.push(json!({
                "type": "pointerMove",
                "x": from.x,
                "y": from.y,
                "origin": "viewport",
            }));
        }
    }
    actions.push(json!({ "type": "pointerDown", "button": 0 }));
    if let Origin::Viewport(from) = origin {
        let span_x = to.x.checked_sub(from.x).ok_or_else(step_overflow)?;
        let span_y = to.y.checked_sub(from.y).ok_or_else(step_overflow)?;
        for step in 1..steps {
            let along = i64::from(step);
            let count = i64::from(steps);
            let x = from
                .x
                .checked_add(
                    span_x
                        .checked_mul(along)
                        .and_then(|v| v.checked_div(count))
                        .ok_or_else(step_overflow)?,
                )
                .ok_or_else(step_overflow)?;
            let y = from
                .y
                .checked_add(
                    span_y
                        .checked_mul(along)
                        .and_then(|v| v.checked_div(count))
                        .ok_or_else(step_overflow)?,
                )
                .ok_or_else(step_overflow)?;
            actions.push(json!({
                "type": "pointerMove",
                "x": x,
                "y": y,
                "origin": "viewport",
            }));
        }
    }
    actions.push(json!({
        "type": "pointerMove",
        "x": to.x,
        "y": to.y,
        "origin": "viewport",
    }));
    actions.push(json!({ "type": "pointerUp", "button": 0 }));
    session.frame(
        "input.performActions",
        json!({
            "context": context.as_str(),
            "actions": [{
                "type": "pointer",
                "id": "mouse",
                "parameters": { "pointerType": "mouse" },
                "actions": actions,
            }],
        }),
    )
}

/// One wheel turn: `by` in CSS pixels, over `at` when the caller named a
/// point.
///
/// # Errors
/// Propagates the frame's own refusal.
pub fn wheel_frame(
    session: &mut Session,
    context: &ContextId,
    at: Option<Point>,
    by: Point,
) -> Result<Frame, AxError> {
    let mut action = Map::new();
    action.insert("type".to_owned(), Value::String("scroll".to_owned()));
    if let Some(at) = at {
        action.insert("x".to_owned(), Value::from(at.x));
        action.insert("y".to_owned(), Value::from(at.y));
        action.insert("origin".to_owned(), Value::String("viewport".to_owned()));
    }
    action.insert("deltaX".to_owned(), Value::from(by.x));
    action.insert("deltaY".to_owned(), Value::from(by.y));
    session.frame(
        "input.performActions",
        json!({
            "context": context.as_str(),
            "actions": [{
                "type": "wheel",
                "id": "wheel",
                "actions": [Value::Object(action)],
            }],
        }),
    )
}

/// The page's own id for the element a resolve frame returned.
///
/// Looks through the reply's remote value for a `sharedId`, to a bounded
/// depth, because the id sits at a fixed place in the protocol's
/// serialization and the exact nesting is the remote end's to decide.
///
/// # Errors
/// Refuses a reply that carries no shared id, which is a page that
/// answered with something other than the node that was asked for.
pub fn shared_id_of(value: &Value) -> Result<String, AxError> {
    if let Some(found) = seek_shared_id(value, SHARED_ID_DEPTH_MAX) {
        return Ok(found);
    }
    Err(AxError::failure(
        AxCode::WireMismatch,
        "resolve a page element",
        "the reply carried no element id",
    )
    .with_recovery(
        "take a fresh snapshot and act again; an element that left the page cannot be dragged",
    ))
}

fn seek_shared_id(value: &Value, depth: u8) -> Option<String> {
    if depth == 0 {
        return None;
    }
    let next = depth.saturating_sub(1);
    match value {
        Value::Object(map) => {
            if let Some(Value::String(id)) = map.get("sharedId") {
                return Some(id.clone());
            }
            map.values().find_map(|child| seek_shared_id(child, next))
        }
        Value::Array(items) => items.iter().find_map(|child| seek_shared_id(child, next)),
        Value::Null | Value::Bool(_) | Value::Number(_) | Value::String(_) => None,
    }
}

fn step_overflow() -> AxError {
    AxError::failure(
        AxCode::InvalidArgs,
        "drag on a page",
        "the pointer path does not fit a coordinate",
    )
    .with_recovery("drag a shorter distance, or in more than one call")
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

    fn context() -> ContextId {
        ContextId::parse("c1").unwrap()
    }

    fn actions(frame: &Frame) -> &Vec<Value> {
        frame
            .params()
            .get("actions")
            .and_then(Value::as_array)
            .unwrap()
    }

    #[test]
    fn a_point_drag_presses_moves_and_releases() {
        let mut session = Session::new();
        let frame = pointer_frame(
            &mut session,
            &context(),
            &DragPath {
                from: Origin::Viewport(Point { x: 10, y: 20 }),
                to: Point { x: 110, y: 120 },
                steps: 2,
            },
        )
        .unwrap();
        assert_eq!(frame.method(), "input.performActions");
        let pointer = &actions(&frame)[0];
        assert_eq!(pointer.get("type").unwrap(), "pointer");
        let inner = pointer.get("actions").and_then(Value::as_array).unwrap();
        let kinds: Vec<&str> = inner
            .iter()
            .filter_map(|action| action.get("type").and_then(Value::as_str))
            .collect();
        assert_eq!(
            kinds,
            vec![
                "pointerMove",
                "pointerDown",
                "pointerMove",
                "pointerMove",
                "pointerUp"
            ]
        );
        assert_eq!(
            inner.last().and_then(|a| a.get("type")),
            Some(&json!("pointerUp"))
        );
    }

    #[test]
    fn an_element_drag_names_the_element_and_makes_one_move() {
        let mut session = Session::new();
        let frame = pointer_frame(
            &mut session,
            &context(),
            &DragPath {
                from: Origin::Element("n1".to_owned()),
                to: Point { x: 40, y: 50 },
                steps: 8,
            },
        )
        .unwrap();
        let pointer = &actions(&frame)[0];
        let inner = pointer.get("actions").and_then(Value::as_array).unwrap();
        assert_eq!(
            inner.len(),
            4,
            "move to the element, press, move to the point, release"
        );
        let first = &inner[0];
        assert_eq!(first.get("origin").unwrap().get("type").unwrap(), "element");
        assert_eq!(
            first
                .get("origin")
                .unwrap()
                .get("element")
                .unwrap()
                .get("sharedId")
                .unwrap(),
            "n1"
        );
    }

    #[test]
    fn a_wheel_turn_carries_its_delta_and_the_point_it_turns_over() {
        let mut session = Session::new();
        let frame = wheel_frame(
            &mut session,
            &context(),
            Some(Point { x: 5, y: 6 }),
            Point { x: 0, y: -240 },
        )
        .unwrap();
        let wheel = &actions(&frame)[0];
        assert_eq!(wheel.get("type").unwrap(), "wheel");
        let scroll = &wheel.get("actions").and_then(Value::as_array).unwrap()[0];
        assert_eq!(scroll.get("deltaY").unwrap(), &json!(-240));
        assert_eq!(scroll.get("x").unwrap(), &json!(5));
    }

    #[test]
    fn a_step_count_outside_the_bound_is_refused() {
        let mut session = Session::new();
        let err = pointer_frame(
            &mut session,
            &context(),
            &DragPath {
                from: Origin::Viewport(Point { x: 0, y: 0 }),
                to: Point { x: 1, y: 1 },
                steps: STEPS_MAX.saturating_add(1),
            },
        )
        .unwrap_err();
        assert_eq!(err.code(), &AxCode::InvalidArgs);
    }

    #[test]
    fn a_shared_id_is_found_where_the_reply_nests_it() {
        let reply = json!({
            "type": "success",
            "result": { "type": "node", "sharedId": "abc", "value": { "nodeType": 1 } },
            "realm": "r1",
        });
        assert_eq!(shared_id_of(&reply).unwrap(), "abc");
        assert!(
            shared_id_of(&json!({ "type": "success", "result": { "type": "string" } })).is_err()
        );
    }
}
