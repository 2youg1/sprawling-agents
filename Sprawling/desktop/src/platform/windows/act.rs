// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use winsafe::co;

use super::geometry::Point;
use super::keys::Modifier;
use super::strokes::{self, Action, Edge, Motion, Released, Stroke};
use crate::refusal::{Refusal, RefusalCode};

pub(crate) fn perform(action: &Action, modifiers: &[Modifier]) -> Result<(), Refusal> {
    let strokes = strokes::of(action, modifiers)?;
    let screen = Screen::now();
    let accepted = send(&events(&strokes, screen)?)?;
    if accepted == strokes.len() {
        return Ok(());
    }
    let releases = strokes::left_held(&strokes, accepted);
    let released = if releases.is_empty() {
        Released::NothingWasHeld
    } else {
        match send(&events(&releases, screen)?)? {
            lifted if lifted == releases.len() => Released::All {
                presses: releases.len(),
            },
            lifted => Released::Partly {
                sent: releases.len(),
                accepted: lifted,
            },
        }
    };
    Err(strokes::cut_short(accepted, strokes.len(), released))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Screen {
    left: i32,
    top: i32,
    width: i32,
    height: i32,
}

impl Screen {
    fn now() -> Screen {
        Screen {
            left: winsafe::GetSystemMetrics(co::SM::XVIRTUALSCREEN),
            top: winsafe::GetSystemMetrics(co::SM::YVIRTUALSCREEN),
            width: winsafe::GetSystemMetrics(co::SM::CXVIRTUALSCREEN),
            height: winsafe::GetSystemMetrics(co::SM::CYVIRTUALSCREEN),
        }
    }

    fn normalised(self, at: Point) -> Result<(i32, i32), Refusal> {
        let across = |value: i32, origin: i32, span: i32| {
            let offset = value.checked_sub(origin)?;

            offset.checked_mul(65_535).and_then(|scaled| {
                scaled.checked_div(span.checked_sub(1).filter(|rest| *rest > 0)?)
            })
        };
        let (Some(x), Some(y)) = (
            across(at.x, self.left, self.width),
            across(at.y, self.top, self.height),
        ) else {
            return Err(Refusal::new(
                RefusalCode::InvalidArgs,
                "act on a window",
                format!("({}, {}) is not a place on this desktop", at.x, at.y),
                "call `desktop.windows` again and act inside the bounds it reports",
            ));
        };
        Ok((x, y))
    }
}

fn events(strokes: &[Stroke], screen: Screen) -> Result<Vec<winsafe::HwKbMouse>, Refusal> {
    strokes.iter().map(|stroke| event(stroke, screen)).collect()
}

fn event(stroke: &Stroke, screen: Screen) -> Result<winsafe::HwKbMouse, Refusal> {
    Ok(match *stroke {
        Stroke::Key { code, edge } => keyboard(code, 0, edge_flag(edge)),

        Stroke::Unicode { unit, edge } => keyboard(
            co::VK::NoValue,
            unit,
            co::KEYEVENTF::UNICODE | edge_flag(edge),
        ),
        Stroke::Pointer { at, motion } => {
            let (flags, data) = match motion {
                Motion::Move => (co::MOUSEEVENTF::MOVE, 0),
                Motion::LeftDown => (co::MOUSEEVENTF::LEFTDOWN, 0),
                Motion::LeftUp => (co::MOUSEEVENTF::LEFTUP, 0),
                Motion::RightDown => (co::MOUSEEVENTF::RIGHTDOWN, 0),
                Motion::RightUp => (co::MOUSEEVENTF::RIGHTUP, 0),
                Motion::Wheel { delta } => (co::MOUSEEVENTF::WHEEL, delta),
            };
            let (x, y) = screen.normalised(at)?;
            winsafe::HwKbMouse::Mouse(winsafe::MOUSEINPUT {
                dx: x,
                dy: y,

                mouseData: u32::from_ne_bytes(data.to_ne_bytes()),
                dwFlags: flags | co::MOUSEEVENTF::ABSOLUTE | co::MOUSEEVENTF::VIRTUALDESK,
                time: 0,
                dwExtraInfo: 0,
            })
        }
    })
}

fn edge_flag(edge: Edge) -> co::KEYEVENTF {
    match edge {
        Edge::Down => co::KEYEVENTF::NoValue,
        Edge::Up => co::KEYEVENTF::KEYUP,
    }
}

fn keyboard(key: co::VK, unit: u16, flags: co::KEYEVENTF) -> winsafe::HwKbMouse {
    winsafe::HwKbMouse::Kb(winsafe::KEYBDINPUT {
        wVk: key,
        wScan: unit,
        dwFlags: flags,
        time: 0,
        dwExtraInfo: 0,
    })
}

fn send(events: &[winsafe::HwKbMouse]) -> Result<usize, Refusal> {
    if events.is_empty() {
        return Ok(0);
    }
    let accepted = winsafe::SendInput(events).unwrap_or(0);
    usize::try_from(accepted).map_err(|_| {
        Refusal::new(
            RefusalCode::ToolUnavailable,
            "count the input events the desktop took",
            format!("Win32 reported {accepted} events, more than this machine can count"),
            "this is a defect in this server rather than in the call; report it",
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
    fn each_stroke_becomes_the_event_it_names() {
        let screen = Screen {
            left: -1920,
            top: 0,
            width: 3841,
            height: 1081,
        };
        let far = Point { x: 1920, y: 1080 };
        let near = Point { x: -1920, y: 0 };
        let spelled: Vec<Result<winsafe::KEYBDINPUT, winsafe::MOUSEINPUT>> = events(
            &[
                Stroke::Key {
                    code: co::VK::RETURN,
                    edge: Edge::Down,
                },
                Stroke::Key {
                    code: co::VK::RETURN,
                    edge: Edge::Up,
                },
                Stroke::Unicode {
                    unit: 0x62,
                    edge: Edge::Up,
                },
                Stroke::Pointer {
                    at: far,
                    motion: Motion::LeftDown,
                },
                Stroke::Pointer {
                    at: near,
                    motion: Motion::Wheel { delta: -120 },
                },
            ],
            screen,
        )
        .unwrap()
        .into_iter()
        .map(|event| match event {
            winsafe::HwKbMouse::Kb(key) => Ok(key),
            winsafe::HwKbMouse::Mouse(pointer) => Err(pointer),
            winsafe::HwKbMouse::Hw(_) => panic!("no stroke is a hardware event"),
        })
        .collect();
        let key = |key: co::VK, unit: u16, flags: co::KEYEVENTF| {
            Ok(winsafe::KEYBDINPUT {
                wVk: key,
                wScan: unit,
                dwFlags: flags,
                time: 0,
                dwExtraInfo: 0,
            })
        };
        let absolute = co::MOUSEEVENTF::ABSOLUTE | co::MOUSEEVENTF::VIRTUALDESK;
        let expected = vec![
            key(co::VK::RETURN, 0, co::KEYEVENTF::NoValue),
            key(co::VK::RETURN, 0, co::KEYEVENTF::KEYUP),
            key(
                co::VK::NoValue,
                0x62,
                co::KEYEVENTF::UNICODE | co::KEYEVENTF::KEYUP,
            ),
            Err(winsafe::MOUSEINPUT {
                dx: 65_535,
                dy: 65_535,
                mouseData: 0,
                dwFlags: co::MOUSEEVENTF::LEFTDOWN | absolute,
                time: 0,
                dwExtraInfo: 0,
            }),
            Err(winsafe::MOUSEINPUT {
                dx: 0,
                dy: 0,
                mouseData: u32::from_ne_bytes((-120_i32).to_ne_bytes()),
                dwFlags: co::MOUSEEVENTF::WHEEL | absolute,
                time: 0,
                dwExtraInfo: 0,
            }),
        ];

        assert!(spelled == expected, "a stroke became another event");
    }
}
