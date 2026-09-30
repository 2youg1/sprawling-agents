// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! This machine's clipboard, as text and as nothing else.
//!
//! The clipboard is one shared object for the whole desktop, and the
//! operating system lends it to one program at a time. Everything here
//! therefore goes through [`Held`], which closes it in `Drop`: a refusal
//! halfway through a read would otherwise leave the clipboard locked
//! against every other program on the machine, and the person whose
//! desktop this is would have no way to know why copying stopped
//! working.
//!
//! Text only, as the tool description promises. An image or a file list
//! on the clipboard is reported as absent rather than converted to some
//! text that stands for it, because a caller told "the clipboard is
//! empty" tries something else, and a caller told "the clipboard says
//! `[image]`" reasons onward from a sentence nobody wrote.

use std::sync::{Mutex, MutexGuard, PoisonError};

use windows::Win32::Foundation::{GlobalFree, HANDLE, HGLOBAL, HWND};
use windows::Win32::System::DataExchange::{
    CloseClipboard, EmptyClipboard, GetClipboardData, IsClipboardFormatAvailable, OpenClipboard,
    SetClipboardData,
};
use windows::Win32::System::Memory::{
    GMEM_MOVEABLE, GlobalAlloc, GlobalLock, GlobalSize, GlobalUnlock,
};
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DestroyWindow, HWND_MESSAGE, WINDOW_EX_STYLE, WINDOW_STYLE,
};
use windows::core::{PCWSTR, w};

use super::fault;
use crate::refusal::{Refusal, RefusalCode};

/// Unicode text, which is the one format this server speaks.
const CF_UNICODETEXT: u32 = 13;

/// One turn with the clipboard per process.
///
/// `OpenClipboard` excludes other processes; it does not exclude the
/// other threads of this one. A second thread here opens the clipboard
/// again and is told it succeeded, and then whichever thread finishes
/// first closes the clipboard under the other: the thread left behind
/// gets `ERROR_CLIPBOARD_NOT_OPEN` from its next call, and — worse — a
/// reader that already holds a handle from `GetClipboardData` walks the
/// block after a writer's `EmptyClipboard` has freed it, which is heap
/// corruption rather than a refusal. This lock is what makes a [`Held`]
/// the only one in this process, so the exclusion the `SAFETY` notes
/// below rely on is real.
static TURN: Mutex<()> = Mutex::new(());

/// The clipboard, open under a window of this process's own, and closed
/// again when this value is dropped.
///
/// The window is a message-only window made for this one turn and
/// destroyed at its end (desktop-SPEC.md section 12.8): `EmptyClipboard`
/// makes the window that opened the clipboard its owner, and a clipboard
/// opened with no window has no owner, so the `SetClipboardData` after it
/// fails. Nothing here waits for a message, so the window needs no pump.
struct Held {
    owner: HWND,
    /// Released after [`Drop::drop`] below has closed the clipboard and
    /// destroyed the owner, because a value's fields are dropped after
    /// its own `Drop` runs.
    _turn: MutexGuard<'static, ()>,
}

impl Held {
    /// # Errors
    /// Refuses when this process cannot make the owner window, and when
    /// another program holds the clipboard. The second is a transient
    /// fact about this desktop rather than anything wrong with the call,
    /// and the recovery says so.
    #[expect(
        unsafe_code,
        reason = "the clipboard is lent to one window at a time, and both the window and the loan are FFI entry points"
    )]
    fn open() -> Result<Held, Refusal> {
        // Waiting here is the point: this thread queues behind the one
        // that holds the clipboard instead of opening it a second time.
        // A panic in another thread poisons a lock that guards no data,
        // and the turn it was holding is over, so the turn is taken.
        let turn = TURN.lock().unwrap_or_else(PoisonError::into_inner);
        // SAFETY: `STATIC` is a class every process has registered, the
        // name is a null pointer the call accepts for "no title", and
        // `HWND_MESSAGE` as the parent makes a message-only window; no
        // menu, instance or creation data is lent to the call.
        let owner = unsafe {
            CreateWindowExW(
                WINDOW_EX_STYLE::default(),
                w!("STATIC"),
                PCWSTR::null(),
                WINDOW_STYLE::default(),
                0,
                0,
                0,
                0,
                Some(HWND_MESSAGE),
                None,
                None,
                None,
            )
        }
        .map_err(|err| {
            fault::win32(
                "make a window to hold the clipboard with",
                "try again; this process could not make a window just now",
                &err,
            )
        })?;
        // SAFETY: `owner` is the message-only window this thread made
        // just above and has not destroyed, and no other thread of this
        // process is between an open and a close, because `turn` is held.
        if let Err(err) = unsafe { OpenClipboard(Some(owner)) } {
            // SAFETY: `owner` is the window this thread made above, and
            // nothing else has been handed it.
            let _destroyed = unsafe { DestroyWindow(owner) };
            return Err(fault::win32(
                "open this machine's clipboard",
                "another program is holding the clipboard for a moment; try again",
                &err,
            ));
        }
        Ok(Held { owner, _turn: turn })
    }
}

impl Drop for Held {
    #[expect(
        unsafe_code,
        reason = "closing the clipboard is what keeps it from being held against every other program on the desktop"
    )]
    fn drop(&mut self) {
        // SAFETY: the clipboard is open at this moment by this thread,
        // under `self.owner`: a `Held` exists only where `OpenClipboard`
        // succeeded, and it is closed exactly once, here. Nothing this
        // module hands out borrows the clipboard past this point: `read`
        // copies the text out before the `Held` is dropped.
        let _closed = unsafe { CloseClipboard() };
        // SAFETY: `self.owner` is the message-only window this thread
        // made in `open` and has not destroyed; the clipboard no longer
        // refers to it, since it was closed above.
        let _destroyed = unsafe { DestroyWindow(self.owner) };
    }
}

/// What is on the clipboard, or `None` when it holds nothing this server
/// reads as text.
///
/// # Errors
/// Refuses when the clipboard cannot be opened or its contents cannot be
/// locked for reading.
#[expect(
    unsafe_code,
    reason = "clipboard contents are reachable only through the FFI entry points; each precondition is stated at its block"
)]
pub(crate) fn read() -> Result<Option<String>, Refusal> {
    let _held = Held::open()?;
    // SAFETY: the clipboard is open for this process, which is what
    // makes this question answerable. It reads one documented format
    // number and writes nothing.
    if unsafe { IsClipboardFormatAvailable(CF_UNICODETEXT) }.is_err() {
        return Ok(None);
    }
    // SAFETY: the clipboard is open and the format was just reported
    // available. The handle that comes back is the clipboard's own and
    // is **not** ours to free — nothing below frees it, and it stays
    // valid until the clipboard is closed, which the `_held` above does
    // after this function has copied what it needs.
    let handle = unsafe { GetClipboardData(CF_UNICODETEXT) }.map_err(|err| {
        fault::win32(
            "read what is on the clipboard",
            "try again; the clipboard changed while it was being read",
            &err,
        )
    })?;
    locked(handle)
}

/// The text behind a clipboard handle, copied out before the lock is
/// released.
///
/// Another program wrote this block, so its terminator is a claim this
/// function does not rely on: the text ends at the first zero unit
/// inside the size `GlobalSize` reports, or at the end of the block.
#[expect(
    unsafe_code,
    reason = "a clipboard handle is global memory, and reading it means locking it, measuring it and borrowing it"
)]
fn locked(handle: HANDLE) -> Result<Option<String>, Refusal> {
    let memory = HGLOBAL(handle.0);
    // SAFETY: `handle` is the clipboard's own handle for
    // `CF_UNICODETEXT`, which is documented to be global memory, so
    // reading it as an `HGLOBAL` is the intended use rather than a
    // reinterpretation. A lock that fails answers null, which the check
    // below reads.
    let text = unsafe { GlobalLock(memory) }.cast::<u16>();
    if text.is_null() {
        return Err(Refusal::new(
            RefusalCode::ToolUnavailable,
            "use the clipboard",
            "the clipboard's text could not be locked for reading",
            "try again; another program may be changing the clipboard",
        ));
    }
    // SAFETY: `memory` is the block locked just above; the call reads
    // its size and writes nothing.
    let size = unsafe { GlobalSize(memory) };
    let copied = match size.checked_div(std::mem::size_of::<u16>()) {
        Some(units) if units > 0 => {
            // SAFETY: `text` is the start of the locked block, global
            // memory is aligned for any `u16`, and `GlobalSize` reported
            // the true size of that block, so `units` of them lie inside
            // it. The slice lives only until the unlock below, and the
            // clipboard stays open meanwhile, so the block cannot move or
            // be freed under it.
            let block = unsafe { std::slice::from_raw_parts(text, units) };
            let end = block
                .iter()
                .position(|unit| *unit == 0)
                .unwrap_or(block.len());
            Ok(Some(String::from_utf16_lossy(
                block.get(..end).unwrap_or(block),
            )))
        }
        Some(_) | None => Err(Refusal::new(
            RefusalCode::ToolUnavailable,
            "use the clipboard",
            "the clipboard's text reported a size of nothing",
            "try again; another program may be changing the clipboard",
        )),
    };
    // SAFETY: `memory` is the handle locked above, and this is that
    // lock's one release. Nothing reads `text` or the slice after it.
    let _unlocked = unsafe { GlobalUnlock(memory) };
    copied
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
#[expect(
    unsafe_code,
    reason = "emptying the clipboard and handing it a block are FFI entry points only"
)]
pub(crate) fn write(text: &str) -> Result<(), Refusal> {
    let mut units: Vec<u16> = text.encode_utf16().collect();
    units.push(0);
    let block = OwnedBlock::filled(&units)?;
    let _held = Held::open()?;
    // SAFETY: the clipboard is open for this process under its own
    // window, which is what makes emptying it this process's right and
    // makes that window the clipboard's owner.
    unsafe { EmptyClipboard() }.map_err(|err| {
        fault::win32(
            "clear the clipboard before writing",
            "try again; another program is contending for the clipboard",
            &err,
        )
    })?;
    // SAFETY: the clipboard is open and emptied under this process's
    // window, and `block.memory` is a moveable block this process
    // allocated, filled and unlocked. On success the clipboard owns the
    // block, and `handed_over` stops `block` from freeing it; on failure
    // `block` still owns it and frees it when dropped.
    match unsafe { SetClipboardData(CF_UNICODETEXT, Some(HANDLE(block.memory.0))) } {
        Ok(_handle) => {
            block.handed_over();
            Ok(())
        }
        Err(err) => Err(Refusal::new(
            RefusalCode::ToolUnavailable,
            "use the clipboard",
            format!(
                "the clipboard was emptied and the new text could not be put on it: {}",
                err.message()
            ),
            "try again; until then the clipboard is empty",
        )),
    }
}

/// A block of global memory holding the new text, freed on every path
/// that does not hand it to the clipboard.
struct OwnedBlock {
    memory: HGLOBAL,
    owner: BlockOwner,
}

/// Who frees the block.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BlockOwner {
    ThisProcess,
    Clipboard,
}

impl OwnedBlock {
    /// A moveable block holding `units`, unlocked and ready to hand over.
    ///
    /// # Errors
    /// Refuses a text too long to measure and a block this machine will
    /// not allocate or lock.
    #[expect(
        unsafe_code,
        reason = "global memory is allocated, locked and filled only through the FFI entry points"
    )]
    fn filled(units: &[u16]) -> Result<OwnedBlock, Refusal> {
        let bytes = units
            .len()
            .checked_mul(std::mem::size_of::<u16>())
            .ok_or_else(|| {
                Refusal::new(
                    RefusalCode::InvalidArgs,
                    "use the clipboard",
                    "the text is longer than this machine can measure",
                    "set a shorter text",
                )
            })?;
        // SAFETY: `GMEM_MOVEABLE` is the allocation kind
        // `SetClipboardData` requires, and `bytes` is the exact size of
        // `units`; the block this returns is owned by the value built
        // from it below, which frees it unless it is handed over.
        let memory = unsafe { GlobalAlloc(GMEM_MOVEABLE, bytes) }.map_err(|err| {
            fault::win32(
                "make room for the new clipboard text",
                "set a shorter text; this machine is out of memory",
                &err,
            )
        })?;
        let block = OwnedBlock {
            memory,
            owner: BlockOwner::ThisProcess,
        };
        // SAFETY: `block.memory` is the live allocation from the line
        // above, held by this function alone.
        let into = unsafe { GlobalLock(block.memory) }.cast::<u16>();
        if into.is_null() {
            return Err(fault::last(
                "lock the new clipboard text",
                "try again; this machine is out of memory",
            ));
        }
        // SAFETY: `into` is the start of a block allocated for exactly
        // `units.len()` `u16`s — `bytes` above is that count times the
        // size of one — and the two regions cannot overlap, since the
        // block is a distinct allocation made after `units`.
        unsafe { std::ptr::copy_nonoverlapping(units.as_ptr(), into, units.len()) };
        // SAFETY: `block.memory` is the handle locked above, and this is
        // that lock's one release. Nothing reads `into` after it.
        let _unlocked = unsafe { GlobalUnlock(block.memory) };
        Ok(block)
    }

    /// The clipboard took the block, so this value no longer frees it.
    fn handed_over(mut self) {
        self.owner = BlockOwner::Clipboard;
    }
}

impl Drop for OwnedBlock {
    #[expect(
        unsafe_code,
        reason = "global memory is freed only through the FFI entry point"
    )]
    fn drop(&mut self) {
        match self.owner {
            // SAFETY: the block is this process's: it was allocated in
            // `filled`, is not locked, and was never taken by the
            // clipboard, so this is its one release.
            BlockOwner::ThisProcess => {
                let _freed = unsafe { GlobalFree(Some(self.memory)) };
            }
            BlockOwner::Clipboard => {}
        }
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

    /// A different fact from [`TURN`], which buys one thread one call:
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
    /// `GlobalLock` of no block needs no desktop and fails the same way.
    #[test]
    fn a_clipboard_block_that_will_not_lock_is_a_refusal_not_an_empty_clipboard() {
        assert_eq!(
            locked(HANDLE(std::ptr::null_mut())),
            Err(Refusal::new(
                RefusalCode::ToolUnavailable,
                "use the clipboard",
                "the clipboard's text could not be locked for reading",
                "try again; another program may be changing the clipboard",
            ))
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
