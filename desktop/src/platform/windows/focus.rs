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

use winsafe::co;

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
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct Aim(pub(super) usize);

impl Aim {
    /// What a read that found no window compares as: no real window has
    /// a null handle, so it is never the window a call named.
    const NOWHERE: Aim = Aim(0);

    pub(super) fn of(window: &winsafe::HWND) -> Aim {
        Aim(window.ptr().addr())
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
pub(super) fn hold(window: &winsafe::HWND, at: Option<Point>) -> Result<(), Refusal> {
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
fn foreground() -> Aim {
    winsafe::HWND::GetForegroundWindow().map_or(Aim::NOWHERE, |held| Aim::of(&held))
}

/// The top-level window under one screen point.
///
/// The ancestor rather than the hit itself: a click lands on a button
/// or a text box, and the question this module asks is which *window*
/// would receive it.
fn under(at: Point) -> Aim {
    winsafe::HWND::WindowFromPoint(winsafe::POINT::with(at.x, at.y))
        .and_then(|hit| hit.GetAncestor(co::GA::ROOT))
        .map_or(Aim::NOWHERE, |root| Aim::of(&root))
}

/// Asks the desktop to put one window in front.
///
/// Best effort by design: Windows refuses this to a process that has
/// not been interacted with, and the answer to a refusal is the same
/// either way — read the foreground back and refuse if it is not the
/// window this call named. Acting on the return value would put the
/// decision in two places.
fn ask_for(window: &winsafe::HWND) {
    let _asked: bool = window.SetForegroundWindow();
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

    /// The contract desktop-SPEC.md section 8-11 holds the hit test to,
    /// whichever interface answers it: the window under a point is the
    /// top-level window drawn there. The window is this test's own, kept
    /// on top of every other and never given the keyboard.
    #[test]
    fn the_window_under_a_point_is_the_window_drawn_there() {
        let title = format!("sprawling contract focus {}", std::process::id());
        let opened = super::super::fixture::Opened::at(&title, 0, 0, None);
        let centre = opened.bounds().centre().unwrap();
        assert_eq!(under(centre), Aim(opened.address()));
    }

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
