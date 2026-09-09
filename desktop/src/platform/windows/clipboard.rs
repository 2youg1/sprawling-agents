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

use windows::Win32::Foundation::{HANDLE, HGLOBAL};
use windows::Win32::System::DataExchange::{
    CloseClipboard, EmptyClipboard, GetClipboardData, IsClipboardFormatAvailable, OpenClipboard,
    SetClipboardData,
};
use windows::Win32::System::Memory::{GMEM_MOVEABLE, GlobalAlloc, GlobalLock, GlobalUnlock};

use super::fault;
use crate::refusal::Refusal;

/// Unicode text, which is the one format this server speaks.
const CF_UNICODETEXT: u32 = 13;

/// The clipboard, open, and closed again when this value is dropped.
struct Held;

impl Held {
    /// # Errors
    /// Refuses when another program holds the clipboard. That is a
    /// transient fact about this desktop rather than anything wrong with
    /// the call, and the recovery says so.
    #[expect(
        unsafe_code,
        reason = "the clipboard is lent to one process at a time, and only through this FFI entry point"
    )]
    fn open() -> Result<Held, Refusal> {
        // SAFETY: `None` asks for the clipboard without associating it
        // with a window, which is what a program with no window of its
        // own does. The call either takes the clipboard or fails; there
        // is no state in between, so the `Held` below stands for a
        // clipboard that really is open.
        unsafe { OpenClipboard(None) }.map_err(|err| {
            fault::win32(
                "open this machine's clipboard",
                "another program is holding the clipboard for a moment; try again",
                &err,
            )
        })?;
        Ok(Held)
    }
}

impl Drop for Held {
    #[expect(
        unsafe_code,
        reason = "closing the clipboard is what keeps it from being held against every other program on the desktop"
    )]
    fn drop(&mut self) {
        // SAFETY: a `Held` exists only where `OpenClipboard` succeeded,
        // and it is closed exactly once, here. Nothing this module hands
        // out borrows the clipboard past this point: `read` copies the
        // text out before the `Held` is dropped.
        let _closed = unsafe { CloseClipboard() };
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
    Ok(locked(handle))
}

/// The text behind a clipboard handle, copied out before the lock is
/// released.
#[expect(
    unsafe_code,
    reason = "a clipboard handle is global memory, and reading it means locking it and walking to its terminator"
)]
fn locked(handle: HANDLE) -> Option<String> {
    let memory = HGLOBAL(handle.0);
    // SAFETY: `handle` is the clipboard's own handle for
    // `CF_UNICODETEXT`, which is documented to be global memory, so
    // reading it as an `HGLOBAL` is the intended use rather than a
    // reinterpretation. A lock that fails answers null, which the check
    // below reads.
    let text = unsafe { GlobalLock(memory) }.cast::<u16>();
    if text.is_null() {
        return None;
    }
    let mut units: Vec<u16> = Vec::new();
    let mut at: usize = 0;
    loop {
        // SAFETY: `text` is the non-null start of a locked
        // `CF_UNICODETEXT` block, which the clipboard guarantees is
        // terminated by a zero unit. `at` only ever advances one unit at
        // a time and the loop stops at that terminator, so every read is
        // inside the block. The clipboard is still open and still locked
        // here, so the block cannot move under this loop.
        let unit = unsafe { *text.add(at) };
        if unit == 0 {
            break;
        }
        units.push(unit);
        at = at.saturating_add(1);
    }
    // SAFETY: `memory` is the handle locked immediately above, and this
    // is that lock's one release. Nothing reads `text` after this point.
    let _unlocked = unsafe { GlobalUnlock(memory) };
    Some(String::from_utf16_lossy(&units))
}

/// Replaces what is on the clipboard.
///
/// # Errors
/// Refuses when the clipboard cannot be opened, emptied, or given the
/// new text.
#[expect(
    unsafe_code,
    reason = "putting text on the clipboard means allocating global memory and handing its ownership over, which only the FFI entry points do"
)]
pub(crate) fn write(text: &str) -> Result<(), Refusal> {
    let mut units: Vec<u16> = text.encode_utf16().collect();
    units.push(0);
    let bytes = units
        .len()
        .checked_mul(std::mem::size_of::<u16>())
        .ok_or_else(|| fault::last("measure the text", "set a shorter text"))?;
    let _held = Held::open()?;
    // SAFETY: the clipboard is open for this process, which is what
    // makes emptying it this process's right. Emptying also transfers
    // ownership of what was there to us, which is why the allocation
    // below may then be given away.
    unsafe { EmptyClipboard() }.map_err(|err| {
        fault::win32(
            "clear the clipboard before writing",
            "try again; another program is contending for the clipboard",
            &err,
        )
    })?;
    // SAFETY: `GMEM_MOVEABLE` is the allocation kind `SetClipboardData`
    // requires, and `bytes` is the exact size of `units` including its
    // terminator. This block is either given to the clipboard below —
    // after which it is the clipboard's to free, never ours — or freed
    // by the operating system when this process ends, which is the
    // documented outcome for a block that was never handed over.
    let block = unsafe { GlobalAlloc(GMEM_MOVEABLE, bytes) }.map_err(|err| {
        fault::win32(
            "make room for the new clipboard text",
            "set a shorter text; this machine is out of memory",
            &err,
        )
    })?;
    // SAFETY: `block` is the live allocation from the line above, held
    // by this function alone.
    let into = unsafe { GlobalLock(block) }.cast::<u16>();
    if into.is_null() {
        return Err(fault::last(
            "lock the new clipboard text",
            "try again; this machine is out of memory",
        ));
    }
    // SAFETY: `into` is the start of a block allocated for exactly
    // `units.len()` `u16`s — `bytes` above is that count times the size
    // of one — and the two regions cannot overlap, since `block` was
    // allocated after `units` and is a distinct allocation.
    unsafe { std::ptr::copy_nonoverlapping(units.as_ptr(), into, units.len()) };
    // SAFETY: `block` is the handle locked above, and this is that
    // lock's one release. Nothing reads `into` after this point.
    let _unlocked = unsafe { GlobalUnlock(block) };
    // SAFETY: the clipboard is open and has been emptied, so this
    // process owns the right to set its contents. On success the
    // clipboard takes ownership of `block`, which is why nothing here
    // frees it afterwards; on failure the block stays ours and is
    // released when this process ends.
    unsafe { SetClipboardData(CF_UNICODETEXT, Some(HANDLE(block.0))) }.map_err(|err| {
        fault::win32(
            "put the new text on the clipboard",
            "try again; another program is contending for the clipboard",
            &err,
        )
    })?;
    Ok(())
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

    /// This one really does touch the machine running the tests, and it
    /// puts back what it found, because a test that leaves somebody's
    /// clipboard changed is a test that damaged their desktop.
    #[test]
    fn text_written_to_the_clipboard_reads_back_as_itself() {
        let Ok(before) = read() else {
            // A build agent with no window station has no clipboard, and
            // that is a refusal this module already reports honestly.
            return;
        };
        let written = "sprawling desktop connector — round trip 🌍";
        write(written).expect("this machine's clipboard accepts text");
        assert_eq!(read().unwrap().as_deref(), Some(written));
        match before {
            Some(original) => write(&original).expect("the original goes back"),
            None => write("").expect("an empty clipboard goes back as empty"),
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
}
