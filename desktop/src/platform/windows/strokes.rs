// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The events one action is, as plain values.
//!
//! An action is a short sequence of key edges, typed UTF-16 units and
//! pointer motions, and which sequence it is decides everything that can
//! go wrong with it: the order modifiers come off in, how far a wheel
//! turns, and what is still held down when the desktop takes only part
//! of the batch. None of that needs Win32, so it is decided here, where a
//! machine with no desktop can check it; `super::act` only turns each
//! stroke into the event Win32 wants and sends the batch.

use windows::Win32::UI::WindowsAndMessaging::WHEEL_DELTA;

use super::geometry::Point;
use super::keys::Modifier;
use crate::refusal::{Refusal, RefusalCode};

/// What one call asks to happen. Exhaustive, and each arm already
/// carries what it needs, so an action that reached this module with a
/// missing argument cannot be spelled.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Action {
    Click { at: Point },
    Double { at: Point },
    Right { at: Point },
    Drag { from: Point, to: Point },
    Scroll { at: Point, notches: i32 },
    Type { text: String },
    Key { code: u16 },
}

impl Action {
    /// Where on the screen this action lands, when it lands anywhere.
    ///
    /// `type` and `key` go wherever the keyboard is, so they name no
    /// place; a drag names the place it starts from, because that is
    /// the window it picks something up in. `super::focus` reads this
    /// to decide whether a second question has to be asked before the
    /// events are sent.
    pub(super) fn lands_at(&self) -> Option<Point> {
        match self {
            Action::Click { at }
            | Action::Double { at }
            | Action::Right { at }
            | Action::Scroll { at, .. } => Some(*at),
            Action::Drag { from, .. } => Some(*from),
            Action::Type { .. } | Action::Key { .. } => None,
        }
    }
}

/// Which way a key or a typed unit moves.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Edge {
    Down,
    Up,
}

/// What the pointer does at a place.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Motion {
    Move,
    LeftDown,
    LeftUp,
    RightDown,
    RightUp,
    /// A turn of the wheel, in the units Win32 counts it in.
    Wheel {
        delta: i32,
    },
}

/// One event of a batch.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Stroke {
    /// A virtual key.
    Key { code: u16, edge: Edge },
    /// One UTF-16 unit typed as itself, whatever the keyboard layout.
    Unicode { unit: u16, edge: Edge },
    /// The pointer at an absolute place on the virtual screen.
    Pointer { at: Point, motion: Motion },
}

/// The whole batch for one action with `modifiers` held around it.
///
/// Modifiers go down first and come off after the action in the reverse
/// order, which is what a hand does and what an application watching
/// the release order expects.
///
/// # Errors
/// Refuses a scroll further than a wheel count can hold.
pub(crate) fn of(action: &Action, modifiers: &[Modifier]) -> Result<Vec<Stroke>, Refusal> {
    let modifier = |held: &Modifier, edge: Edge| Stroke::Key {
        code: held.code(),
        edge,
    };
    let mut strokes: Vec<Stroke> = modifiers
        .iter()
        .map(|held| modifier(held, Edge::Down))
        .collect();
    strokes.extend(spelled(action)?);
    strokes.extend(modifiers.iter().rev().map(|held| modifier(held, Edge::Up)));
    Ok(strokes)
}

/// The strokes one action is, in the order they happen.
fn spelled(action: &Action) -> Result<Vec<Stroke>, Refusal> {
    let pointer = |at: Point, motion: Motion| Stroke::Pointer { at, motion };
    Ok(match action {
        Action::Click { at } => vec![
            pointer(*at, Motion::Move),
            pointer(*at, Motion::LeftDown),
            pointer(*at, Motion::LeftUp),
        ],
        Action::Double { at } => vec![
            pointer(*at, Motion::Move),
            pointer(*at, Motion::LeftDown),
            pointer(*at, Motion::LeftUp),
            pointer(*at, Motion::LeftDown),
            pointer(*at, Motion::LeftUp),
        ],
        Action::Right { at } => vec![
            pointer(*at, Motion::Move),
            pointer(*at, Motion::RightDown),
            pointer(*at, Motion::RightUp),
        ],
        Action::Drag { from, to } => vec![
            pointer(*from, Motion::Move),
            pointer(*from, Motion::LeftDown),
            pointer(*to, Motion::Move),
            pointer(*to, Motion::LeftUp),
        ],
        Action::Scroll { at, notches } => vec![
            pointer(*at, Motion::Move),
            pointer(
                *at,
                Motion::Wheel {
                    delta: wheel(*notches)?,
                },
            ),
        ],
        Action::Type { text } => text
            .encode_utf16()
            .flat_map(|unit| {
                [
                    Stroke::Unicode {
                        unit,
                        edge: Edge::Down,
                    },
                    Stroke::Unicode {
                        unit,
                        edge: Edge::Up,
                    },
                ]
            })
            .collect(),
        Action::Key { code } => vec![
            Stroke::Key {
                code: *code,
                edge: Edge::Down,
            },
            Stroke::Key {
                code: *code,
                edge: Edge::Up,
            },
        ],
    })
}

/// How far `notches` turn the wheel, counted in `WHEEL_DELTA`.
///
/// The product is taken in `i64`, where it always fits, and then has to
/// fit the `i32` Win32 reads: a scroll that does not is refused rather
/// than wrapped into one that scrolls the other way.
fn wheel(notches: i32) -> Result<i32, Refusal> {
    i64::from(notches)
        .checked_mul(i64::from(WHEEL_DELTA))
        .and_then(|delta| i32::try_from(delta).ok())
        .ok_or_else(|| {
            Refusal::new(
                RefusalCode::InvalidArgs,
                "act on a window",
                format!("{notches} notches is further than a wheel turns"),
                "scroll in smaller steps and look at the window between them",
            )
        })
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

    /// Nothing here presses a key on the machine running the tests: the
    /// strokes are built and counted, never sent. Whether a click landed
    /// is what an operator checks on a real desktop
    /// (desktop-SPEC.md §16.2).
    #[test]
    fn each_action_is_the_events_it_says_it_is_and_no_more() {
        let at = Point { x: 10, y: 20 };
        let counted = |action: &Action| of(action, &[]).unwrap().len();
        assert_eq!(counted(&Action::Click { at }), 3);
        assert_eq!(counted(&Action::Right { at }), 3);
        // A double click is one more down-up pair than a click, which is
        // what makes it a double click rather than two calls.
        assert_eq!(counted(&Action::Double { at }), 5);
        assert_eq!(
            counted(&Action::Drag {
                from: at,
                to: Point { x: 99, y: 99 }
            }),
            4
        );
        assert_eq!(
            of(&Action::Scroll { at, notches: -3 }, &[]).unwrap(),
            vec![
                Stroke::Pointer {
                    at,
                    motion: Motion::Move
                },
                Stroke::Pointer {
                    at,
                    motion: Motion::Wheel { delta: -360 }
                },
            ]
        );
        assert_eq!(counted(&Action::Key { code: 0x0D }), 2);
    }

    /// One character is one press and one release, so a string is
    /// exactly twice its UTF-16 length — including the two units an
    /// emoji takes, which is the case a `chars()` count gets wrong.
    #[test]
    fn typing_is_one_press_and_one_release_per_utf16_unit() {
        let typed = |text: &str| {
            of(
                &Action::Type {
                    text: text.to_owned(),
                },
                &[],
            )
            .unwrap()
        };
        assert_eq!(typed("hello").len(), 10);
        assert_eq!(typed("a🌍").len(), 6);
        assert!(typed("").is_empty());
    }

    /// The two actions that go to the keyboard alone name no place, so
    /// nothing asks what lies under a point they never touch.
    #[test]
    fn only_the_pointer_actions_name_a_place_on_the_screen() {
        let at = Point { x: 10, y: 20 };
        let to = Point { x: 99, y: 99 };
        assert_eq!(Action::Click { at }.lands_at(), Some(at));
        assert_eq!(Action::Scroll { at, notches: -3 }.lands_at(), Some(at));
        // A drag is judged where it picks something up.
        assert_eq!(Action::Drag { from: at, to }.lands_at(), Some(at));
        assert_eq!(
            Action::Type {
                text: "hello".to_owned()
            }
            .lands_at(),
            None
        );
        assert_eq!(Action::Key { code: 0x0D }.lands_at(), None);
    }

    /// A modifier is pressed before the action and released after it,
    /// and several are released in the reverse order — which is what a
    /// hand does and what an application watching the order expects.
    #[test]
    fn modifiers_wrap_the_action_and_come_off_in_the_reverse_order() {
        let key = |code: u16, edge: Edge| Stroke::Key { code, edge };
        assert_eq!(
            of(
                &Action::Key { code: 0x0D },
                &[Modifier::Ctrl, Modifier::Shift]
            )
            .unwrap(),
            vec![
                key(Modifier::Ctrl.code(), Edge::Down),
                key(Modifier::Shift.code(), Edge::Down),
                key(0x0D, Edge::Down),
                key(0x0D, Edge::Up),
                key(Modifier::Shift.code(), Edge::Up),
                key(Modifier::Ctrl.code(), Edge::Up),
            ]
        );
    }

    /// A scroll far enough to overflow the wheel count is a refusal
    /// rather than a wrapped one that would scroll the other way.
    #[test]
    fn a_scroll_too_far_to_count_is_refused_rather_than_wrapped() {
        let refusal = of(
            &Action::Scroll {
                at: Point { x: 0, y: 0 },
                notches: i32::MAX,
            },
            &[],
        )
        .unwrap_err();
        assert_eq!(refusal.as_error()["data"]["code"], "E_INVALID_ARGS");
    }
}
