// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use windows::Win32::UI::WindowsAndMessaging::WHEEL_DELTA;
use winsafe::co;

use super::geometry::Point;
use super::keys::Modifier;
use crate::refusal::{Refusal, RefusalCode};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Action {
    Click { at: Point },
    Double { at: Point },
    Right { at: Point },
    Drag { from: Point, to: Point },
    Scroll { at: Point, notches: i32 },
    Type { text: String },
    Key { code: co::VK },
}

impl Action {

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Edge {
    Down,
    Up,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Motion {
    Move,
    LeftDown,
    LeftUp,
    RightDown,
    RightUp,

    Wheel {
        delta: i32,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Stroke {

    Key { code: co::VK, edge: Edge },

    Unicode { unit: u16, edge: Edge },

    Pointer { at: Point, motion: Motion },
}

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Released {

    NothingWasHeld,

    All { presses: usize },

    Partly { sent: usize, accepted: usize },
}

const NOTHING_SENT_RECOVERY: &str =
    "check the window with `desktop.screenshot` and try again once it is in the foreground";

const PART_SENT_RECOVERY: &str = "look at the window with `desktop.screenshot` before acting \
                                  again, and do not repeat this action unchanged: part of it \
                                  may already have landed";

pub(crate) fn left_held(strokes: &[Stroke], accepted: usize) -> Vec<Stroke> {
    let mut held: Vec<Held> = Vec::new();
    let mut last_at: Option<Point> = None;
    for stroke in strokes.iter().take(accepted) {
        let (press, edge) = match *stroke {
            Stroke::Key { code, edge } => (Held::Key(code), edge),
            Stroke::Unicode { unit, edge } => (Held::Unicode(unit), edge),
            Stroke::Pointer { at, motion } => {
                last_at = Some(at);
                match motion {
                    Motion::LeftDown => (Held::Left, Edge::Down),
                    Motion::LeftUp => (Held::Left, Edge::Up),
                    Motion::RightDown => (Held::Right, Edge::Down),
                    Motion::RightUp => (Held::Right, Edge::Up),
                    Motion::Move | Motion::Wheel { .. } => continue,
                }
            }
        };
        match edge {
            Edge::Down => held.push(press),
            Edge::Up => {
                if let Some(at) = held.iter().rposition(|was| *was == press) {
                    held.remove(at);
                }
            }
        }
    }

    held.iter()
        .rev()
        .filter_map(|was| match *was {
            Held::Key(code) => Some(Stroke::Key {
                code,
                edge: Edge::Up,
            }),
            Held::Unicode(unit) => Some(Stroke::Unicode {
                unit,
                edge: Edge::Up,
            }),
            Held::Left => last_at.map(|at| Stroke::Pointer {
                at,
                motion: Motion::LeftUp,
            }),
            Held::Right => last_at.map(|at| Stroke::Pointer {
                at,
                motion: Motion::RightUp,
            }),
        })
        .collect()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Held {
    Key(co::VK),
    Unicode(u16),
    Left,
    Right,
}

pub(crate) fn cut_short(accepted: usize, sent: usize, released: Released) -> Refusal {
    if accepted == 0 {
        return Refusal::new(
            RefusalCode::ToolUnavailable,
            "act on a window",
            format!(
                "the desktop took none of the {sent} input events: a lock screen or an elevated \
                 window holds the input"
            ),
            NOTHING_SENT_RECOVERY,
        );
    }
    let afterwards = match released {
        Released::NothingWasHeld => String::new(),
        Released::All { presses } => {
            format!("; the {presses} presses it left held were released")
        }
        Released::Partly { sent, accepted } => format!(
            "; releasing the {sent} presses it left held also stopped after {accepted}, so a \
             key or button may still be held down"
        ),
    };
    Refusal::new(
        RefusalCode::ToolUnavailable,
        "act on a window",
        format!(
            "the desktop took {accepted} of the {sent} input events of this action and then \
             stopped accepting input, so the action may have partly happened{afterwards}"
        ),
        PART_SENT_RECOVERY,
    )
    .effect_unknown()
}

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

    #[test]
    fn each_action_is_the_events_it_says_it_is_and_no_more() {
        let at = Point { x: 10, y: 20 };
        let counted = |action: &Action| of(action, &[]).unwrap().len();
        assert_eq!(counted(&Action::Click { at }), 3);
        assert_eq!(counted(&Action::Right { at }), 3);

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
        assert_eq!(
            counted(&Action::Key {
                code: co::VK::RETURN
            }),
            2
        );
    }

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

    #[test]
    fn only_the_pointer_actions_name_a_place_on_the_screen() {
        let at = Point { x: 10, y: 20 };
        let to = Point { x: 99, y: 99 };
        assert_eq!(Action::Click { at }.lands_at(), Some(at));
        assert_eq!(Action::Scroll { at, notches: -3 }.lands_at(), Some(at));

        assert_eq!(Action::Drag { from: at, to }.lands_at(), Some(at));
        assert_eq!(
            Action::Type {
                text: "hello".to_owned()
            }
            .lands_at(),
            None
        );
        assert_eq!(
            Action::Key {
                code: co::VK::RETURN
            }
            .lands_at(),
            None
        );
    }

    #[test]
    fn modifiers_wrap_the_action_and_come_off_in_the_reverse_order() {
        let key = |code: co::VK, edge: Edge| Stroke::Key { code, edge };
        assert_eq!(
            of(
                &Action::Key {
                    code: co::VK::RETURN
                },
                &[Modifier::Ctrl, Modifier::Shift]
            )
            .unwrap(),
            vec![
                key(Modifier::Ctrl.code(), Edge::Down),
                key(Modifier::Shift.code(), Edge::Down),
                key(co::VK::RETURN, Edge::Down),
                key(co::VK::RETURN, Edge::Up),
                key(Modifier::Shift.code(), Edge::Up),
                key(Modifier::Ctrl.code(), Edge::Up),
            ]
        );
    }

    #[test]
    fn a_batch_cut_short_leaves_nothing_held_that_it_pressed() {
        let key = |code: co::VK, edge: Edge| Stroke::Key { code, edge };
        let all = of(
            &Action::Key {
                code: co::VK::RETURN,
            },
            &[Modifier::Ctrl, Modifier::Shift],
        )
        .unwrap();
        assert_eq!(
            left_held(&all, 3),
            vec![
                key(co::VK::RETURN, Edge::Up),
                key(Modifier::Shift.code(), Edge::Up),
                key(Modifier::Ctrl.code(), Edge::Up),
            ]
        );
        assert_eq!(left_held(&all, 0), vec![]);
        assert_eq!(left_held(&all, 6), vec![]);
        let from = Point { x: 10, y: 20 };
        let dragged = of(
            &Action::Drag {
                from,
                to: Point { x: 99, y: 99 },
            },
            &[],
        )
        .unwrap();
        assert_eq!(
            left_held(&dragged, 2),
            vec![Stroke::Pointer {
                at: from,
                motion: Motion::LeftUp
            }]
        );
        let typed = of(
            &Action::Type {
                text: "ab".to_owned(),
            },
            &[],
        )
        .unwrap();
        assert_eq!(
            left_held(&typed, 3),
            vec![Stroke::Unicode {
                unit: 0x62,
                edge: Edge::Up
            }]
        );
    }

    #[test]
    fn a_batch_cut_short_is_reported_as_partly_done_not_as_nothing() {
        assert_eq!(
            cut_short(3, 6, Released::All { presses: 3 }),
            Refusal::new(
                RefusalCode::ToolUnavailable,
                "act on a window",
                "the desktop took 3 of the 6 input events of this action and then stopped \
                 accepting input, so the action may have partly happened; the 3 presses it \
                 left held were released",
                PART_SENT_RECOVERY,
            )
            .effect_unknown()
        );
        assert_eq!(
            cut_short(
                2,
                6,
                Released::Partly {
                    sent: 2,
                    accepted: 1
                }
            ),
            Refusal::new(
                RefusalCode::ToolUnavailable,
                "act on a window",
                "the desktop took 2 of the 6 input events of this action and then stopped \
                 accepting input, so the action may have partly happened; releasing the 2 \
                 presses it left held also stopped after 1, so a key or button may still be \
                 held down",
                PART_SENT_RECOVERY,
            )
            .effect_unknown()
        );
        assert_eq!(
            cut_short(0, 6, Released::NothingWasHeld),
            Refusal::new(
                RefusalCode::ToolUnavailable,
                "act on a window",
                "the desktop took none of the 6 input events: a lock screen or an elevated \
                 window holds the input",
                NOTHING_SENT_RECOVERY,
            )
        );
    }

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
