// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! This machine's clipboard as UTF-16 text, one turn per process.
//!
//! `OpenClipboard` excludes other processes, not the other threads of
//! this one: a second thread opens the clipboard again and is told it
//! succeeded, and a writer's `EmptyClipboard` then frees the block a
//! reader is walking. [`TURN`] makes one call at a time the only one in
//! this process, which is the precondition both calls below state.

use std::sync::{Mutex, PoisonError};

use winsafe::co;

use crate::ended::{self, Failure};
use crate::leaf;
use crate::step::Step;

/// One turn with the clipboard per process.
static TURN: Mutex<()> = Mutex::new(());

/// The room the first read lends, in UTF-16 units.
const FIRST_ROOM: usize = 4_096;

/// How many times a short buffer is grown before the read is refused:
/// the text can change between two turns, and a clipboard that grows on
/// every one is refused rather than chased.
const ATTEMPTS: u8 = 4;

/// The clipboard's text without its terminator, or `None` when it holds
/// no text.
///
/// # Errors
/// [`Failure::At`] with the step that stopped the read: another program
/// holding the clipboard, a block that will not lock or reports no size,
/// or a text that kept growing.
pub fn text() -> Result<Option<Vec<u16>>, Failure> {
    // A panic in another thread poisons a lock that guards no data, and
    // the turn it was holding is over, so the turn is taken.
    let _turn = TURN.lock().unwrap_or_else(PoisonError::into_inner);
    let mut room = FIRST_ROOM;
    let mut found = 0;
    for _attempt in 0..ATTEMPTS {
        let mut into = vec![0_u16; room];
        let mut code = co::ERROR::SUCCESS;
        // SAFETY: `into` is `into.len()` initialised units lent to this
        // call alone, and that same length is the capacity the leaf
        // copies up to. No other thread of this process is between an
        // open and a close of the clipboard while `_turn` is held, so
        // the block the leaf reads cannot be emptied under it from here.
        #[expect(unsafe_code, reason = "the one call that reads the clipboard")]
        let raw = unsafe {
            leaf::sprawling_desktop_clipboard_read(
                into.as_mut_ptr(),
                into.len(),
                &raw mut found,
                &raw mut code,
            )
        };
        let step = ended::step(raw)?;
        if step == Step::Finished {
            into.truncate(found);
            return Ok(Some(into));
        }
        if step == Step::Absent {
            return Ok(None);
        }
        if step != Step::NoRoom {
            return Err(Failure::At { step, code });
        }
        room = found;
    }
    Err(Failure::At {
        step: Step::NoRoom,
        code: co::ERROR::INSUFFICIENT_BUFFER,
    })
}

/// Replaces the clipboard's contents with `units`, which carry no
/// terminator: the leaf writes exactly one.
///
/// # Errors
/// [`Failure::At`] with the step that stopped the write. `Handing` is
/// the one that leaves the clipboard empty: it was emptied, and then it
/// would not take the new text.
pub fn put_text(units: &[u16]) -> Result<(), Failure> {
    let _turn = TURN.lock().unwrap_or_else(PoisonError::into_inner);
    let mut code = co::ERROR::SUCCESS;
    // SAFETY: `units` is a live slice of `units.len()` units the leaf
    // only reads, into a block of its own that it frees unless the
    // clipboard takes it. `_turn` is held, so no other thread of this
    // process opens the clipboard in between.
    #[expect(unsafe_code, reason = "the one call that writes the clipboard")]
    let raw = unsafe {
        leaf::sprawling_desktop_clipboard_write(units.as_ptr(), units.len(), &raw mut code)
    };
    ended::finished(raw, code)
}
