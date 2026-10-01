// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! This machine's clipboard, as text and as nothing else.
//!
//! The clipboard is one shared object for the whole desktop, and the
//! operating system lends it to one program at a time. The Zig leaf
//! opens it under a message-only window of its own, closes it on every
//! path, and takes one turn per process (`crates/desktop/Spec.lean` D8
//! and 8-12); this module decides what each of its answers means to a
//! caller.
//!
//! Text only, as the tool description promises. An image or a file list
//! on the clipboard is reported as absent rather than converted to some
//! text that stands for it, because a caller told "the clipboard is
//! empty" tries something else, and a caller told "the clipboard says
//! `[image]`" reasons onward from a sentence nobody wrote.

use desktop_ffi::ended::Failure;
use desktop_ffi::step::Step;

use super::fault;
use crate::refusal::{Refusal, RefusalCode};

/// What every refusal of this module names as the action that failed.
const DOING: &str = "use the clipboard";

/// What is on the clipboard, or `None` when it holds nothing this server
/// reads as text.
///
/// Another program wrote the block, so its terminator is a claim the
/// leaf does not rely on: the text ends at the first zero unit inside
/// the size `GlobalSize` reports, or at the end of the block.
///
/// # Errors
/// Refuses when the clipboard cannot be opened or its contents cannot be
/// locked for reading.
pub(crate) fn read() -> Result<Option<String>, Refusal> {
    let units = desktop_ffi::clipboard::text().map_err(refused)?;
    Ok(units.map(|text| String::from_utf16_lossy(&text)))
}

/// Replaces what is on the clipboard.
///
/// The new text is made ready before the clipboard is emptied, so the
/// only failure that leaves the clipboard empty is the handover itself,
/// and that refusal says so.
///
/// # Errors
/// Refuses when the text cannot be made ready, and when the clipboard
/// cannot be opened, emptied, or given the new text.
pub(crate) fn write(text: &str) -> Result<(), Refusal> {
    let units: Vec<u16> = text.encode_utf16().collect();
    desktop_ffi::clipboard::put_text(&units).map_err(refused)
}

/// The step the leaf stopped at, as the sentence a caller can act on.
fn refused(failure: Failure) -> Refusal {
    let Failure::At { step, code } = failure else {
        return fault::leaf(DOING, "try again", failure);
    };
    let machine = |doing: &str, recovery: &str| fault::system(doing, recovery, code);
    match step {
        Step::Owner => machine(
            "make a window to hold the clipboard with",
            "try again; this process could not make a window just now",
        ),
        // A transient fact about this desktop rather than anything wrong
        // with the call, and the recovery says so.
        Step::Opening => machine(
            "open this machine's clipboard",
            "another program is holding the clipboard for a moment; try again",
        ),
        Step::Fetching => machine(
            "read what is on the clipboard",
            "try again; the clipboard changed while it was being read",
        ),
        // A lock that fails is a refusal to retry, never an empty
        // clipboard, which would send a caller off to try something else.
        Step::Locking => unavailable(
            "the clipboard's text could not be locked for reading",
            "try again; another program may be changing the clipboard",
        ),
        Step::EmptyBlock => unavailable(
            "the clipboard's text reported a size of nothing",
            "try again; another program may be changing the clipboard",
        ),
        Step::NoRoom => unavailable(
            "the clipboard's text kept growing while it was being read",
            "try again once the program writing it has finished",
        ),
        Step::Measuring => Refusal::new(
            RefusalCode::InvalidArgs,
            DOING,
            "the text is longer than this machine can measure",
            "set a shorter text",
        ),
        Step::Allocating => machine(
            "make room for the new clipboard text",
            "set a shorter text; this machine is out of memory",
        ),
        Step::Emptying => machine(
            "clear the clipboard before writing",
            "try again; another program is contending for the clipboard",
        ),
        Step::Handing => unavailable(
            &format!("the clipboard was emptied and the new text could not be put on it: {code}"),
            "try again; until then the clipboard is empty",
        ),
        Step::Finished
        | Step::Absent
        | Step::Listing
        | Step::NoWindow
        | Step::Context
        | Step::Bitmap
        | Step::Selecting
        | Step::Drawing
        | Step::Reading
        | Step::ShortRows => fault::stray(DOING, step),
    }
}

fn unavailable(subject: &str, recovery: &str) -> Refusal {
    Refusal::new(RefusalCode::ToolUnavailable, DOING, subject, recovery)
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
    use std::sync::{Mutex, MutexGuard, PoisonError};

    use super::*;

    /// A different fact from the leaf's turn, which buys one thread one
    /// call:
    /// a test that writes and then asserts on what comes back needs the
    /// clipboard to itself across the whole pair, and the tests below
    /// run on threads of one process.
    static PAIRS: Mutex<()> = Mutex::new(());

    fn pair_turn() -> MutexGuard<'static, ()> {
        PAIRS.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// One call against a clipboard another program may be holding. The
    /// module's contract for contention is a refusal whose recovery says
    /// to try again, so this tries again, boundedly: what these tests
    /// are about is what comes back, never the wait. Any other refusal,
    /// and the refusal met after the last attempt, panics with every
    /// part of it on screen.
    fn despite_contention<T>(mut work: impl FnMut() -> Result<T, Refusal>) -> T {
        let mut last = None;
        for _attempt in 0..200 {
            match work() {
                Ok(answer) => return answer,
                Err(failure) => {
                    let error = failure.as_error();
                    let code = error
                        .get("data")
                        .and_then(|data| data.get("code"))
                        .and_then(|code| code.as_str())
                        .unwrap_or("");
                    assert_eq!(
                        code, "E_TOOL_UNAVAILABLE",
                        "a refusal this helper will not retry: {failure:?}"
                    );
                    last = Some(failure);
                    std::thread::sleep(std::time::Duration::from_millis(5));
                }
            }
        }
        panic!("still contended after the last attempt: {last:?}");
    }

    /// A block that will not lock is a refusal to retry, never an empty
    /// clipboard, which would send a caller off to try something else.
    /// The leaf answers `Locking` for it (its own test locks no block and
    /// sees that step); this is what the step becomes.
    #[test]
    fn a_clipboard_block_that_will_not_lock_is_a_refusal_not_an_empty_clipboard() {
        assert_eq!(
            refused(Failure::At {
                step: Step::Locking,
                code: winsafe::co::ERROR::INVALID_HANDLE,
            }),
            Refusal::new(
                RefusalCode::ToolUnavailable,
                "use the clipboard",
                "the clipboard's text could not be locked for reading",
                "try again; another program may be changing the clipboard",
            )
        );
    }

    /// This one really does touch the machine running the tests, and it
    /// puts back what it found, because a test that leaves somebody's
    /// clipboard changed is a test that damaged their desktop.
    #[test]
    fn text_written_to_the_clipboard_reads_back_as_itself() {
        let _pair = pair_turn();
        let Ok(before) = read() else {
            // A build agent with no window station has no clipboard, and
            // that is a refusal this module already reports honestly.
            return;
        };
        let written = "sprawling desktop connector — round trip 🌍";
        despite_contention(|| write(written));
        assert_eq!(despite_contention(read).as_deref(), Some(written));
        match before {
            Some(original) => despite_contention(|| write(&original)),
            None => despite_contention(|| write("")),
        }
    }

    /// The lock is released on every path, so a hundred reads in a row
    /// do not leave the desktop's clipboard held against other programs.
    #[test]
    fn repeated_reads_do_not_leave_the_clipboard_held() {
        for _attempt in 0..100 {
            let _answered = read();
        }
        assert!(read().is_ok() || read().is_err());
    }

    /// Threads of one process do not corrupt the clipboard for each
    /// other.
    ///
    /// Before `TURN` existed this aborted the whole test binary with
    /// `STATUS_HEAP_CORRUPTION`: `OpenClipboard` let a second thread in,
    /// one thread's `EmptyClipboard` freed the block another thread was
    /// already walking, and the text that came back was whatever the
    /// allocator had put there since. Every read here happens after this
    /// thread's own write, so whatever comes back must be one of the
    /// texts these threads write; garbled bytes are not.
    #[test]
    fn concurrent_threads_each_read_back_a_text_some_thread_wrote() {
        let _pair = pair_turn();
        let Ok(before) = read() else {
            // A build agent with no window station has no clipboard.
            return;
        };
        const THREADS: usize = 4;
        const ROUNDS: usize = 25;
        let texts: Vec<String> = (0..THREADS)
            .map(|index| format!("sprawling clipboard turn {index} 🌍"))
            .collect();
        std::thread::scope(|threads| {
            for mine in &texts {
                let every = &texts;
                threads.spawn(move || {
                    for _round in 0..ROUNDS {
                        // The module's contract for contention is a
                        // refusal whose recovery says to try again, so
                        // the round tries again: what this test is about
                        // is what comes back, never the wait.
                        let taken = despite_contention(|| write(mine).and_then(|()| read()));
                        let seen = taken.expect("a text just written is there");
                        assert!(
                            every.contains(&seen),
                            "read back {seen:?}, which no thread wrote"
                        );
                    }
                });
            }
        });
        match before {
            Some(original) => write(&original).expect("the original goes back"),
            None => write("").expect("an empty clipboard goes back as empty"),
        }
    }
}
