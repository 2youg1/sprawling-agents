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
//! sends them in one batch.
//!
//! Mouse coordinates go out as absolute positions on the **virtual**
//! desktop, normalised to the range Win32 wants. Relative movement would
//! depend on where the pointer happened to be, and a click that depends
//! on the previous click is exactly the chaining this tool does not do.
//!
//! A batch is inserted in order, and Win32 reports only how many of its
//! events it inserted; it does not take back the ones it did. The usual
//! reason for a short count is that another program holds the input
//! desktop — a UAC prompt, a screen lock. When only part went in, this
//! module says so, with the counts, and sends one more batch: the
//! releases of whatever that part left held down, and nothing else
//! (desktop-SPEC.md section 12.4).

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
    let accepted = send(&inputs(&strokes)?)?;
    if accepted == strokes.len() {
        return Ok(());
    }
    let releases = strokes::left_held(&strokes, accepted);
    let released = if releases.is_empty() {
        Released::NothingWasHeld
    } else {
        match send(&inputs(&releases)?)? {
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

/// The Win32 events a run of strokes is.
fn inputs(strokes: &[Stroke]) -> Result<Vec<INPUT>, Refusal> {
    strokes.iter().map(input).collect()
}

/// The Win32 event one stroke is.
fn input(stroke: &Stroke) -> Result<INPUT, Refusal> {
    Ok(match *stroke {
        Stroke::Key { code, edge } => keyboard(VIRTUAL_KEY(code), 0, edge_flag(edge)),
        // `KEYEVENTF_UNICODE` is what makes typing independent of the
        // keyboard layout: the character arrives as a character, so a
        // model typing `@` does not have to know this machine's layout.
        Stroke::Unicode { unit, edge } => {
            keyboard(VIRTUAL_KEY(0), unit, KEYEVENTF_UNICODE | edge_flag(edge))
        }
        Stroke::Pointer { at, motion } => {
            let (flags, data) = match motion {
                Motion::Move => (MOUSEEVENTF_MOVE, 0),
                Motion::LeftDown => (MOUSEEVENTF_LEFTDOWN, 0),
                Motion::LeftUp => (MOUSEEVENTF_LEFTUP, 0),
                Motion::RightDown => (MOUSEEVENTF_RIGHTDOWN, 0),
                Motion::RightUp => (MOUSEEVENTF_RIGHTUP, 0),
                Motion::Wheel { delta } => (MOUSEEVENTF_WHEEL, delta),
            };
            mouse(at, flags, data)?
        }
    })
}

/// The flag a key event carries for the way it moves.
fn edge_flag(edge: Edge) -> KEYBD_EVENT_FLAGS {
    match edge {
        Edge::Down => KEYBD_EVENT_FLAGS(0),
        Edge::Up => KEYEVENTF_KEYUP,
    }
}

/// Hands the whole batch to Win32 at once, and answers how many of its
/// events Win32 inserted.
#[expect(
    unsafe_code,
    reason = "SendInput is how a keystroke or a click reaches a desktop; there is no safe Rust for it, and the precondition is stated at the block"
)]
fn send(events: &[INPUT]) -> Result<usize, Refusal> {
    let size = i32::try_from(std::mem::size_of::<INPUT>()).map_err(|_| {
        fault::last(
            "measure one input event",
            "this is a defect in this server rather than in the call; report it",
        )
    })?;
    // SAFETY: `events` is a slice this function borrows for the whole
    // call, and the binding passes its pointer and its length together,
    // so the count Win32 reads cannot disagree with the memory that
    // exists; `size` is the size of this slice's element type.
    let accepted = unsafe { SendInput(events, size) };
    usize::try_from(accepted).map_err(|_| {
        Refusal::new(
            RefusalCode::ToolUnavailable,
            "count the input events the desktop took",
            format!("Win32 reported {accepted} events, more than this machine can count"),
            "this is a defect in this server rather than in the call; report it",
        )
    })
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

/// One keyboard event: a virtual key, or a UTF-16 unit when the flags
/// say `KEYEVENTF_UNICODE`.
fn keyboard(key: VIRTUAL_KEY, unit: u16, flags: KEYBD_EVENT_FLAGS) -> INPUT {
    INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: key,
                wScan: unit,
                dwFlags: flags,
                time: 0,
                dwExtraInfo: 0,
            },
        },
    }
}
