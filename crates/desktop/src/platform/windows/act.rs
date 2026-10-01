// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! `SendInput`: one action landing on one window, never a script of
//! them.
//!
//! One call, one action. The tool description promises no retry, no
//! chaining and no fallback to a nearby element, and this module is
//! where that promise is kept: `super::strokes` decides which events the
//! action is, and this module turns each into the event Win32 reads and
//! sends them in one batch, through `winsafe`'s safe `SendInput`
//! (desktop-SPEC.md section 12.9).
//!
//! Mouse coordinates go out as absolute positions on the **virtual**
//! desktop, normalised to the range Win32 wants. Relative movement would
//! depend on where the pointer happened to be, and a click that depends
//! on the previous click is exactly the chaining this tool does not do.
//! The virtual desktop is measured once per action, so every event of
//! one batch is normalised against the same geometry.
//!
//! A batch is inserted in order, and Win32 reports only how many of its
//! events it inserted; it does not take back the ones it did. The usual
//! reason for a short count is that another program holds the input
//! desktop — a UAC prompt, a screen lock. When only part went in, this
//! module says so, with the counts, and sends one more batch: the
//! releases of whatever that part left held down, and nothing else
//! (desktop-SPEC.md section 12.4).

use winsafe::co;

use super::geometry::Point;
use super::keys::Modifier;
use super::strokes::{self, Action, Edge, Motion, Released, Stroke};
use crate::refusal::{Refusal, RefusalCode};

/// Carries out one action, holding `modifiers` down for the whole of it.
///
/// # Errors
/// Refuses when this desktop will not accept input, which is what a
/// locked screen or an elevated window in the foreground looks like from
/// here.
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

/// The virtual desktop, as one batch is normalised against it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Screen {
    left: i32,
    top: i32,
    width: i32,
    height: i32,
}

impl Screen {
    /// The virtual desktop at this instant, which spans every monitor.
    fn now() -> Screen {
        Screen {
            left: winsafe::GetSystemMetrics(co::SM::XVIRTUALSCREEN),
            top: winsafe::GetSystemMetrics(co::SM::YVIRTUALSCREEN),
            width: winsafe::GetSystemMetrics(co::SM::CXVIRTUALSCREEN),
            height: winsafe::GetSystemMetrics(co::SM::CYVIRTUALSCREEN),
        }
    }

    /// A screen point in the 0..=65535 range `MOUSEEVENTF_ABSOLUTE`
    /// counts in, measured across the virtual desktop rather than the
    /// primary monitor — which is what makes a second monitor reachable.
    fn normalised(self, at: Point) -> Result<(i32, i32), Refusal> {
        let across = |value: i32, origin: i32, span: i32| {
            let offset = value.checked_sub(origin)?;
            // 65535 is the full-scale value Win32 documents for this field.
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

/// The Win32 events a run of strokes is.
fn events(strokes: &[Stroke], screen: Screen) -> Result<Vec<winsafe::HwKbMouse>, Refusal> {
    strokes.iter().map(|stroke| event(stroke, screen)).collect()
}

/// The Win32 event one stroke is.
fn event(stroke: &Stroke, screen: Screen) -> Result<winsafe::HwKbMouse, Refusal> {
    Ok(match *stroke {
        Stroke::Key { code, edge } => keyboard(code, 0, edge_flag(edge)),
        // `KEYEVENTF_UNICODE` is what makes typing independent of the
        // keyboard layout: the character arrives as a character, so a
        // model typing `@` does not have to know this machine's layout.
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
                // A wheel turn away from the user is negative, and the
                // field is the same 32 bits read as unsigned.
                mouseData: u32::from_ne_bytes(data.to_ne_bytes()),
                dwFlags: flags | co::MOUSEEVENTF::ABSOLUTE | co::MOUSEEVENTF::VIRTUALDESK,
                time: 0,
                dwExtraInfo: 0,
            })
        }
    })
}

/// The flag a key event carries for the way it moves.
fn edge_flag(edge: Edge) -> co::KEYEVENTF {
    match edge {
        Edge::Down => co::KEYEVENTF::NoValue,
        Edge::Up => co::KEYEVENTF::KEYUP,
    }
}

/// One keyboard event: a virtual key, or a UTF-16 unit when the flags
/// say `KEYEVENTF_UNICODE`.
fn keyboard(key: co::VK, unit: u16, flags: co::KEYEVENTF) -> winsafe::HwKbMouse {
    winsafe::HwKbMouse::Kb(winsafe::KEYBDINPUT {
        wVk: key,
        wScan: unit,
        dwFlags: flags,
        time: 0,
        dwExtraInfo: 0,
    })
}

/// Hands the whole batch to Win32 at once, and answers how many of its
/// events Win32 inserted.
///
/// Win32 answers a batch it took none of with a zero and the reason on
/// the calling thread, and `winsafe` turns that zero into an error. The
/// zero is the fact section 12.4 reads, and the reason is always the
/// one `strokes::cut_short` already tells — another program holds the
/// input desktop — so the error is read back as the count it stands for.
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

    /// The contract desktop-SPEC.md section 8-11 holds input to: each
    /// stroke becomes the one event it names, keys by their code,
    /// typed units as units whatever the layout, and pointer events at
    /// an absolute place on the whole virtual desktop, so the corners of
    /// that desktop are the ends of the absolute range. Nothing is sent.
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
        // `winsafe`'s event structs compare but do not print, so the
        // comparison is spelled out.
        assert!(spelled == expected, "a stroke became another event");
    }
}
