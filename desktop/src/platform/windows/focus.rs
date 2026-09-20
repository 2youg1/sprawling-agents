// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Which window has the keyboard now, and the refusal when it is not
//! the one this action named.
//!
//! **`SendInput` follows the keyboard, not the decision.** A keystroke
//! carries no window with it: it lands wherever the foreground window
//! is at the instant the event is posted. Everything above this module
//! has judged a window by its title and its process — the scope file,
//! the allowlist, the refusal a caller reads — and none of that reaches
//! `SendInput`, so between choosing a window and typing into it the
//! desktop may have handed the keyboard to another one. A password box
//! that appeared in that gap would receive the text.
//!
//! So an action is sent only while the window it named holds the
//! keyboard. This server asks for the foreground once, waits for the
//! answer, and reads it back; a desktop that will not hand the window
//! over is a refusal, because a caller told "done" would reason onward
//! from keystrokes that went somewhere else.
//!
//! A pointer action is checked twice over: the window the events reach
//! must also be the window under the point they land on. The two
//! questions are different — a window can hold the keyboard while
//! another one covers the place a click was aimed at — and the second
//! is the one that matters for a click on a dialog that opened over the
//! target.

use std::time::Duration;

use windows::Win32::Foundation::{HWND, POINT};
use windows::Win32::UI::WindowsAndMessaging::{
    GA_ROOT, GetAncestor, GetForegroundWindow, SetForegroundWindow, WindowFromPoint,
};

use super::geometry::Point;
use crate::refusal::{Refusal, RefusalCode};

/// How many times the foreground window is read back after this server
/// has asked for it. Windows hands the foreground over asynchronously,
/// so the read immediately after the request usually still reports the
/// window that is on its way out.
const READS_AFTER_ASKING: u8 = 10;

/// How long to wait between those reads. Ten of these is a fifth of a
/// second, which is long enough for a window manager to finish a switch
/// and short enough that a caller reads the refusal as an answer.
const BETWEEN_READS: Duration = Duration::from_millis(20);

/// One window as this check compares it: the address of its handle.
///
/// An address rather than the handle itself, so the comparison this
/// module exists for is a comparison of plain values and can be
/// asserted on a machine with no desktop at all.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Aim(usize);

impl Aim {
    fn of(window: HWND) -> Aim {
        Aim(window.0.addr())
    }
}

/// Refuses unless `window` will receive what is about to be sent.
///
/// `at` is where a pointer action lands, and `None` for the two actions
/// that go to the keyboard alone.
///
/// # Errors
/// Refuses when another window holds the keyboard and will not give it
/// up, and when another window covers the point this action aims at.
/// Nothing has been sent when either refusal is returned.
pub(super) fn hold(window: HWND, at: Option<Point>) -> Result<(), Refusal> {
    let intended = Aim::of(window);
    if foreground() != intended {
        ask_for(window);
        wait_for(intended);
    }
    settled(intended, foreground(), at.map(under))
}

/// Whether the events may be sent: who holds the keyboard, and what
/// lies under the point, against the window this call named.
///
/// Pure over three values, which is what makes the rule this module
/// exists for provable without a desktop (desktop-SPEC.md section 16.2).
fn settled(intended: Aim, holder: Aim, under_pointer: Option<Aim>) -> Result<(), Refusal> {
    if holder != intended {
        return Err(Refusal::new(
            RefusalCode::ToolUnavailable,
            "act on a window",
            "another window holds the keyboard, and this desktop would deliver these events to \
             it rather than to the window this call named"
                .to_owned(),
            "nothing was sent. Bring the window to the front — a lock screen, a UAC prompt or a \
             window opened by somebody at this machine keeps it back — and ask again",
        ));
    }
    match under_pointer {
        None => Ok(()),
        Some(under) if under == intended => Ok(()),
        Some(_covered) => Err(Refusal::new(
            RefusalCode::ToolUnavailable,
            "act on a window",
            "another window covers the place this action would land on".to_owned(),
            "nothing was sent. Take a fresh `desktop.snapshot` and act on a ref from it; the \
             window that moved in front is not one this action may click",
        )),
    }
}

/// Which window has the keyboard at this instant.
#[expect(
    unsafe_code,
    reason = "the foreground window is process-wide state, readable only through the FFI entry point"
)]
fn foreground() -> Aim {
    // SAFETY: `GetForegroundWindow` reads one system-wide handle, takes
    // no argument and writes nothing, so there is no state this call
    // can be handed in a shape it does not accept. A handle that has
    // since closed is compared as a number here and never dereferenced.
    Aim::of(unsafe { GetForegroundWindow() })
}

/// The top-level window under one screen point.
///
/// The ancestor rather than the hit itself: a click lands on a button
/// or a text box, and the question this module asks is which *window*
/// would receive it.
#[expect(
    unsafe_code,
    reason = "hit-testing the desktop and walking to a window's root are FFI entry points with no safe Rust equivalent"
)]
fn under(at: Point) -> Aim {
    // SAFETY: both calls take values rather than pointers — a `POINT`
    // by value, and a handle that `WindowFromPoint` itself returned —
    // and neither writes through anything this process owns. A point on
    // no window answers a null handle, which compares unequal to every
    // real window and is never dereferenced here.
    Aim::of(unsafe { GetAncestor(WindowFromPoint(POINT { x: at.x, y: at.y }), GA_ROOT) })
}

/// Asks the desktop to put one window in front.
///
/// Best effort by design: Windows refuses this to a process that has
/// not been interacted with, and the answer to a refusal is the same
/// either way — read the foreground back and refuse if it is not the
/// window this call named. Acting on the return value would put the
/// decision in two places.
#[expect(
    unsafe_code,
    reason = "raising a window is an FFI entry point; the outcome is read back rather than trusted"
)]
fn ask_for(window: HWND) {
    // SAFETY: the handle was returned by this module's own enumeration
    // a moment ago, and a handle that has closed since makes this call
    // fail rather than act on another window — which the read-back
    // below then reports as a refusal.
    let _asked = unsafe { SetForegroundWindow(window) };
}

/// Waits, bounded, for the desktop to finish handing the keyboard over.
fn wait_for(intended: Aim) {
    for _read in 0..READS_AFTER_ASKING {
        if foreground() == intended {
            return;
        }
        std::thread::sleep(BETWEEN_READS);
    }
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

    const NAMED: Aim = Aim(0x1000);
    const ANOTHER: Aim = Aim(0x2000);

    /// S-11: the window that gets the keystrokes is the window the call
    /// named, or there are no keystrokes. Without this check a `type`
    /// decided against one window is delivered to whatever took the
    /// foreground in between — a password box included.
    #[test]
    fn keystrokes_are_refused_while_another_window_holds_the_keyboard() {
        let refusal = settled(NAMED, ANOTHER, None).expect_err("another window has the keyboard");
        let error = refusal.as_error();
        assert_eq!(error["data"]["code"], "E_TOOL_UNAVAILABLE");
        let recovery = error["data"]["recovery"].as_str().unwrap();
        assert!(recovery.contains("nothing was sent"), "{recovery}");
        assert!(settled(NAMED, NAMED, None).is_ok());
    }

    /// A pointer action asks the second question too: the window under
    /// the point is the window this action may click, whoever holds the
    /// keyboard.
    #[test]
    fn a_click_is_refused_when_another_window_covers_the_place_it_lands() {
        let refusal = settled(NAMED, NAMED, Some(ANOTHER))
            .expect_err("a window in front of the target is not the target");
        let error = refusal.as_error();
        assert_eq!(error["data"]["code"], "E_TOOL_UNAVAILABLE");
        assert!(
            error["data"]["recovery"]
                .as_str()
                .unwrap()
                .contains("desktop.snapshot")
        );
        assert!(settled(NAMED, NAMED, Some(NAMED)).is_ok());
        // The keyboard is judged first, because an action that cannot
        // be delivered at all is not a question about what covers what.
        let first = settled(NAMED, ANOTHER, Some(NAMED)).unwrap_err();
        assert!(
            first.as_error()["data"]["subject"]
                .as_str()
                .unwrap()
                .contains("holds the keyboard")
        );
    }

    /// Two windows are the same window exactly when their handles are,
    /// which is the whole of the identity this check compares.
    #[test]
    fn one_window_is_itself_and_no_other() {
        assert_eq!(NAMED, Aim(0x1000));
        assert_ne!(NAMED, ANOTHER);
    }
}
