// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! `SendInput`: one action landing on one window, never a script of
//! them.
//!
//! One call, one action. The tool description promises no retry, no
//! chaining and no fallback to a nearby element, and this module is
//! where that promise is kept: it builds the events for exactly what was
//! asked and sends them in one batch, so a partially-applied action is
//! not a state this server can leave a desktop in.
//!
//! Mouse coordinates go out as absolute positions on the **virtual**
//! desktop, normalised to the range Win32 wants. Relative movement would
//! depend on where the pointer happened to be, and a click that depends
//! on the previous click is exactly the chaining this tool does not do.
//!
//! A `SendInput` that reports fewer events than it was given is a
//! refusal here rather than a silent partial action. The usual reason is
//! that another program holds the input desktop — a UAC prompt, a
//! screen lock — and a caller told "done" while nothing moved would
//! reason onward from an action that did not happen.

use windows::Win32::UI::Input::KeyboardAndMouse::{
    INPUT, INPUT_0, INPUT_KEYBOARD, INPUT_MOUSE, KEYBD_EVENT_FLAGS, KEYBDINPUT, KEYEVENTF_KEYUP,
    KEYEVENTF_UNICODE, MOUSE_EVENT_FLAGS, MOUSEEVENTF_ABSOLUTE, MOUSEEVENTF_LEFTDOWN,
    MOUSEEVENTF_LEFTUP, MOUSEEVENTF_MOVE, MOUSEEVENTF_RIGHTDOWN, MOUSEEVENTF_RIGHTUP,
    MOUSEEVENTF_VIRTUALDESK, MOUSEEVENTF_WHEEL, MOUSEINPUT, SendInput, VIRTUAL_KEY,
};
use windows::Win32::UI::WindowsAndMessaging::{
    GetSystemMetrics, SM_CXVIRTUALSCREEN, SM_CYVIRTUALSCREEN, SM_XVIRTUALSCREEN, SM_YVIRTUALSCREEN,
};

use super::fault;
use super::geometry::Point;
use super::keys::Modifier;
use crate::refusal::{Refusal, RefusalCode};

/// How far one notch of the wheel turns, in the units Win32 counts them
/// in. This is Microsoft's own `WHEEL_DELTA`, restated because the
/// binding does not export it under that name.
const ONE_NOTCH: i32 = 120;

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

/// Carries out one action, holding `modifiers` down for the whole of it.
///
/// # Errors
/// Refuses when this desktop will not accept input, which is what a
/// locked screen or an elevated window in the foreground looks like from
/// here.
pub(crate) fn perform(action: &Action, modifiers: &[Modifier]) -> Result<(), Refusal> {
    let mut events: Vec<INPUT> = modifiers
        .iter()
        .map(|held| key(held.code(), true))
        .collect();
    events.extend(spelled(action)?);
    // Released in the reverse of the order they were pressed, which is
    // what a keyboard does and what an application watching for the
    // release order expects.
    events.extend(modifiers.iter().rev().map(|held| key(held.code(), false)));
    send(&events)
}

/// The events one action is, in the order they happen.
fn spelled(action: &Action) -> Result<Vec<INPUT>, Refusal> {
    Ok(match action {
        Action::Click { at } => vec![
            mouse(*at, MOUSEEVENTF_MOVE, 0)?,
            mouse(*at, MOUSEEVENTF_LEFTDOWN, 0)?,
            mouse(*at, MOUSEEVENTF_LEFTUP, 0)?,
        ],
        Action::Double { at } => vec![
            mouse(*at, MOUSEEVENTF_MOVE, 0)?,
            mouse(*at, MOUSEEVENTF_LEFTDOWN, 0)?,
            mouse(*at, MOUSEEVENTF_LEFTUP, 0)?,
            mouse(*at, MOUSEEVENTF_LEFTDOWN, 0)?,
            mouse(*at, MOUSEEVENTF_LEFTUP, 0)?,
        ],
        Action::Right { at } => vec![
            mouse(*at, MOUSEEVENTF_MOVE, 0)?,
            mouse(*at, MOUSEEVENTF_RIGHTDOWN, 0)?,
            mouse(*at, MOUSEEVENTF_RIGHTUP, 0)?,
        ],
        Action::Drag { from, to } => vec![
            mouse(*from, MOUSEEVENTF_MOVE, 0)?,
            mouse(*from, MOUSEEVENTF_LEFTDOWN, 0)?,
            mouse(*to, MOUSEEVENTF_MOVE, 0)?,
            mouse(*to, MOUSEEVENTF_LEFTUP, 0)?,
        ],
        Action::Scroll { at, notches } => vec![
            mouse(*at, MOUSEEVENTF_MOVE, 0)?,
            mouse(
                *at,
                MOUSEEVENTF_WHEEL,
                notches.checked_mul(ONE_NOTCH).ok_or_else(|| {
                    Refusal::new(
                        RefusalCode::InvalidArgs,
                        "act on a window",
                        format!("{notches} notches is further than a wheel turns"),
                        "scroll in smaller steps and look at the window between them",
                    )
                })?,
            )?,
        ],
        Action::Type { text } => text.encode_utf16().flat_map(unicode).collect(),
        Action::Key { code } => vec![key(*code, true), key(*code, false)],
    })
}

/// Hands the whole batch to Win32 at once.
#[expect(
    unsafe_code,
    reason = "SendInput is how a keystroke or a click reaches a desktop; there is no safe Rust for it, and the precondition is stated at the block"
)]
fn send(events: &[INPUT]) -> Result<(), Refusal> {
    let size = i32::try_from(std::mem::size_of::<INPUT>()).map_err(|_| {
        fault::last(
            "measure one input event",
            "this is a defect in this server rather than in the call; report it",
        )
    })?;
    // SAFETY: `events` is a live slice this function owns, and the
    // binding passes both its pointer and its length, so the count Win32
    // reads cannot disagree with the memory that exists. `size` is the
    // size of the very type the slice holds, which is the one thing this
    // call gets wrong when it is got wrong.
    let accepted = unsafe { SendInput(events, size) };
    if usize::try_from(accepted).is_ok_and(|count| count == events.len()) {
        return Ok(());
    }
    Err(fault::last(
        "send this action to the desktop",
        "this desktop is not accepting input right now — a lock screen or an elevated window \
         blocks it. Nothing was half-done: check the window with `desktop.screenshot` and try \
         again once it is in the foreground",
    ))
}

/// One mouse event at an absolute position on the virtual desktop.
fn mouse(at: Point, flags: MOUSE_EVENT_FLAGS, data: i32) -> Result<INPUT, Refusal> {
    let (x, y) = normalised(at)?;
    Ok(INPUT {
        r#type: INPUT_MOUSE,
        Anonymous: INPUT_0 {
            mi: MOUSEINPUT {
                dx: x,
                dy: y,
                mouseData: u32::from_ne_bytes(data.to_ne_bytes()),
                dwFlags: flags | MOUSEEVENTF_ABSOLUTE | MOUSEEVENTF_VIRTUALDESK,
                time: 0,
                dwExtraInfo: 0,
            },
        },
    })
}

/// A screen point in the 0..=65535 range `MOUSEEVENTF_ABSOLUTE` counts
/// in, measured across the virtual desktop rather than the primary
/// monitor — which is what makes a second monitor reachable.
#[expect(
    unsafe_code,
    reason = "the size of the virtual desktop is a system metric, readable only through the FFI entry point"
)]
fn normalised(at: Point) -> Result<(i32, i32), Refusal> {
    // SAFETY: `GetSystemMetrics` reads one documented system-wide
    // number for the index it is given and writes nothing. Every index
    // used here is a constant from the binding, so there is no index it
    // could be handed that it does not define.
    let (left, top, width, height) = unsafe {
        (
            GetSystemMetrics(SM_XVIRTUALSCREEN),
            GetSystemMetrics(SM_YVIRTUALSCREEN),
            GetSystemMetrics(SM_CXVIRTUALSCREEN),
            GetSystemMetrics(SM_CYVIRTUALSCREEN),
        )
    };
    let across = |value: i32, origin: i32, span: i32| {
        let offset = value.checked_sub(origin)?;
        // 65535 is the full-scale value Win32 documents for this field.
        offset
            .checked_mul(65_535)
            .and_then(|scaled| scaled.checked_div(span.checked_sub(1).filter(|rest| *rest > 0)?))
    };
    let (Some(x), Some(y)) = (across(at.x, left, width), across(at.y, top, height)) else {
        return Err(Refusal::new(
            RefusalCode::InvalidArgs,
            "act on a window",
            format!("({}, {}) is not a place on this desktop", at.x, at.y),
            "call `desktop.windows` again and act inside the bounds it reports",
        ));
    };
    Ok((x, y))
}

/// One key going down or coming up.
fn key(code: u16, down: bool) -> INPUT {
    INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: VIRTUAL_KEY(code),
                wScan: 0,
                dwFlags: if down {
                    KEYBD_EVENT_FLAGS(0)
                } else {
                    KEYEVENTF_KEYUP
                },
                time: 0,
                dwExtraInfo: 0,
            },
        },
    }
}

/// One UTF-16 unit typed as itself.
///
/// `KEYEVENTF_UNICODE` is what makes this independent of the keyboard
/// layout: the character arrives as a character, so a model typing `@`
/// does not have to know whether this machine is on a British layout.
fn unicode(unit: u16) -> [INPUT; 2] {
    let event = |flags: KEYBD_EVENT_FLAGS| INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: VIRTUAL_KEY(0),
                wScan: unit,
                dwFlags: KEYEVENTF_UNICODE | flags,
                time: 0,
                dwExtraInfo: 0,
            },
        },
    };
    [event(KEYBD_EVENT_FLAGS(0)), event(KEYEVENTF_KEYUP)]
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
    /// events are built and counted, never sent. Whether a click landed
    /// is what an operator checks on a real desktop
    /// (desktop-SPEC.md §16.2).
    #[test]
    fn each_action_is_the_events_it_says_it_is_and_no_more() {
        let at = Point { x: 10, y: 20 };
        let counted = |action: &Action| spelled(action).unwrap().len();
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
        assert_eq!(counted(&Action::Scroll { at, notches: -3 }), 2);
        assert_eq!(counted(&Action::Key { code: 0x0D }), 2);
    }

    /// One character is one press and one release, so a string is
    /// exactly twice its UTF-16 length — including the two units an
    /// emoji takes, which is the case a `chars()` count gets wrong.
    #[test]
    fn typing_is_one_press_and_one_release_per_utf16_unit() {
        let plain = Action::Type {
            text: "hello".to_owned(),
        };
        assert_eq!(spelled(&plain).unwrap().len(), 10);
        let paired = Action::Type {
            text: "a🌍".to_owned(),
        };
        assert_eq!(spelled(&paired).unwrap().len(), 6);
        let empty = Action::Type {
            text: String::new(),
        };
        assert!(spelled(&empty).unwrap().is_empty());
    }

    /// A modifier is pressed before the action and released after it,
    /// and several are released in the reverse order — which is what a
    /// hand does and what an application watching the order expects.
    #[expect(
        unsafe_code,
        reason = "test code reads back the union arm it wrote one line earlier"
    )]
    #[test]
    fn modifiers_wrap_the_action_and_come_off_in_the_reverse_order() {
        let mut events: Vec<INPUT> = [Modifier::Ctrl, Modifier::Shift]
            .iter()
            .map(|held| key(held.code(), true))
            .collect();
        events.extend(spelled(&Action::Key { code: 0x0D }).unwrap());
        events.extend(
            [Modifier::Ctrl, Modifier::Shift]
                .iter()
                .rev()
                .map(|held| key(held.code(), false)),
        );
        assert_eq!(events.len(), 6);
        // SAFETY: every event built above is `INPUT_KEYBOARD`, so the
        // keyboard arm of the union is the arm that was written.
        let code = |at: usize| unsafe { events[at].Anonymous.ki.wVk.0 };
        assert_eq!(code(0), Modifier::Ctrl.code());
        assert_eq!(code(1), Modifier::Shift.code());
        assert_eq!(code(4), Modifier::Shift.code());
        assert_eq!(code(5), Modifier::Ctrl.code());
    }

    /// A scroll far enough to overflow the wheel count is a refusal
    /// rather than a wrapped one that would scroll the other way.
    #[test]
    fn a_scroll_too_far_to_count_is_refused_rather_than_wrapped() {
        let overflowing = spelled(&Action::Scroll {
            at: Point { x: 0, y: 0 },
            notches: i32::MAX,
        });
        // `INPUT` has no `Debug`, so the refusal is taken out by hand
        // rather than through `expect_err`.
        let Err(refusal) = overflowing else {
            panic!("a wheel count that overflows is not a smaller scroll")
        };
        assert_eq!(refusal.as_error()["data"]["code"], "E_INVALID_ARGS");
    }
}
