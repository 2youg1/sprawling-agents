// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! `EnumWindows`: which top-level windows exist, and the title, process
//! and bounds of each.
//!
//! What is deliberately not here: any judgement. This module hands back
//! every visible, titled top-level window it can measure, and
//! `crate::scope` and `super::target` decide which of them a call may
//! see or touch. Filtering here as well would put the allowlist in two
//! places, and the copy that is not the scope file's is the one that
//! would drift.
//!
//! A window that will not answer one of the three questions is **left
//! out rather than reported with a blank**: a caller cannot name a
//! window it has no title for, and a row of empty strings would only
//! look like something it could name.

use windows::Win32::Foundation::{CloseHandle, HWND, LPARAM, MAX_PATH, RECT};
use windows::Win32::System::Threading::{
    OpenProcess, PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION, QueryFullProcessImageNameW,
};
use windows::Win32::UI::WindowsAndMessaging::{
    EnumWindows, GetWindowRect, GetWindowTextLengthW, GetWindowTextW, GetWindowThreadProcessId,
    IsWindowVisible,
};
use windows::core::{BOOL, PWSTR};

use super::fault;
use super::geometry::Bounds;
use super::target::Named;
use crate::refusal::Refusal;

/// One window as this module found it: what it is called, and the handle
/// the tools that touch it need.
pub(crate) struct Window {
    pub(crate) named: Named,
    pub(crate) bounds: Bounds,
    pub(crate) handle: HWND,
}

/// Every visible, titled top-level window on this desktop.
///
/// # Errors
/// Refuses when the enumeration itself will not run. A window that
/// individually will not answer is dropped from the list instead, since
/// one unreadable window is not a reason to report none.
#[expect(
    unsafe_code,
    reason = "EnumWindows is a callback API, so the list it fills has to travel to the callback as an address"
)]
pub(crate) fn desktop() -> Result<Vec<Window>, Refusal> {
    let mut handles: Vec<HWND> = Vec::new();
    // The address travels as a number because that is the only thing an
    // `LPARAM` is. `expose_provenance` is the spelling that says so, and
    // it pairs with the `with_exposed_provenance_mut` in the callback:
    // going through `as` would say the same thing while hiding that a
    // pointer's provenance is what is being carried.
    let address = std::ptr::from_mut(&mut handles).expose_provenance();
    let Ok(carried) = isize::try_from(address) else {
        return Err(fault::last(
            "list the windows on this desktop",
            "this is a defect in this server rather than in the call; report it",
        ));
    };
    let collecting = LPARAM(carried);
    // SAFETY: `collect` is the `extern "system"` function immediately
    // below, so `EnumWindows` calls back into this module and nothing
    // else. The pointer inside `collecting` is derived from `handles`,
    // which is a live local for the whole of this call and has no second
    // alias while the call runs — `handles` is not touched again until
    // `EnumWindows` has returned, and `EnumWindows` is documented as
    // synchronous, so the callback cannot outlive the borrow.
    let walked = unsafe { EnumWindows(Some(collect), collecting) };
    walked.map_err(|err| {
        fault::win32(
            "list the windows on this desktop",
            "this is the operating system refusing, not the scope file; try again, and if it \
             persists this desktop session may be one no program can enumerate",
            &err,
        )
    })?;
    Ok(handles.into_iter().filter_map(described).collect())
}

/// The callback `EnumWindows` drives. It does the least a callback can:
/// puts one handle in a list, and decides nothing.
///
/// Returning `TRUE` means "keep going"; there is no early stop, because
/// stopping early would make the listing depend on enumeration order.
#[expect(
    unsafe_code,
    reason = "this is the callback EnumWindows drives; recovering the list it was given is the one thing it does"
)]
extern "system" fn collect(handle: HWND, into: LPARAM) -> BOOL {
    let Ok(address) = usize::try_from(into.0) else {
        return BOOL(1);
    };
    let into = std::ptr::with_exposed_provenance_mut::<Vec<HWND>>(address);
    // SAFETY: `into` is the address `desktop` passed to `EnumWindows` in
    // this same call, and `EnumWindows` passes it through unchanged, so
    // it addresses that function's live `Vec<HWND>` with that vector's
    // own provenance. Nothing else in this package registers this
    // callback, so there is no other provenance it could arrive with,
    // and `EnumWindows` drives the callback on the calling thread before
    // it returns, so this is the only reference to that vector alive.
    let handles = unsafe { into.as_mut() };
    if let Some(handles) = handles {
        handles.push(handle);
    }
    BOOL(1)
}

/// One handle, answered for — or dropped, when it will not answer.
#[expect(
    unsafe_code,
    reason = "whether a window is visible is a question only the FFI entry point answers"
)]
fn described(handle: HWND) -> Option<Window> {
    // SAFETY: `handle` came from `EnumWindows` in this same call.
    // `IsWindowVisible` takes any HWND, including one whose window has
    // closed since, and answers false for it rather than misbehaving.
    if !unsafe { IsWindowVisible(handle) }.as_bool() {
        return None;
    }
    let title = title(handle)?;
    if title.is_empty() {
        return None;
    }
    Some(Window {
        named: Named {
            title,
            process: process(handle)?,
        },
        bounds: rectangle(handle).ok()?,
        handle,
    })
}

/// The window's title, or `None` when it has none this call can read.
#[expect(
    unsafe_code,
    reason = "a window title is read into a caller-provided buffer, which is an FFI convention with no safe wrapper"
)]
fn title(handle: HWND) -> Option<String> {
    // SAFETY: `handle` is an HWND from this call's enumeration.
    // `GetWindowTextLengthW` reads only the window it names and returns
    // zero for one that has closed; it writes nothing.
    let length = unsafe { GetWindowTextLengthW(handle) };
    let length = usize::try_from(length).ok()?.checked_add(1)?;
    let mut text: Vec<u16> = vec![0; length];
    // SAFETY: `text` is a live, uniquely owned buffer of exactly
    // `length` `u16`s, and the binding takes it as a slice, so the
    // length Win32 is told is the length that exists. Win32 writes at
    // most that many units and terminates within them.
    let written = unsafe { GetWindowTextW(handle, &mut text) };
    let written = usize::try_from(written).ok()?;
    text.get(..written).map(String::from_utf16_lossy)
}

/// The file name of the process that owns the window.
#[expect(
    unsafe_code,
    reason = "the owning process is found by opening it and asking for its image name, both FFI entry points"
)]
fn process(handle: HWND) -> Option<String> {
    let mut owner: u32 = 0;
    // SAFETY: `handle` is an HWND from this call's enumeration, and
    // `owner` is a live local `u32` that outlives the call, so the
    // pointer Win32 writes one `u32` through is valid and uniquely ours.
    unsafe { GetWindowThreadProcessId(handle, Some(std::ptr::from_mut(&mut owner))) };
    // SAFETY: this asks for the narrowest right that answers the
    // question — `PROCESS_QUERY_LIMITED_INFORMATION` cannot read the
    // process's memory. `owner` is a process id read immediately above;
    // a process that has since exited makes this fail, which is the
    // `.ok()?` below rather than undefined behaviour.
    let opened = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, owner) }.ok()?;
    let mut path: Vec<u16> = vec![0; usize::try_from(MAX_PATH).ok()?];
    let mut length = u32::try_from(path.len()).ok()?;
    // SAFETY: `opened` is a handle this function owns and closes below.
    // `path` holds `length` `u16`s and is uniquely owned here, and
    // `length` is that same count, so Win32 writes inside the buffer; it
    // writes the count it used back through `length`, which is a live
    // local.
    let queried = unsafe {
        QueryFullProcessImageNameW(
            opened,
            PROCESS_NAME_WIN32,
            PWSTR(path.as_mut_ptr()),
            std::ptr::from_mut(&mut length),
        )
    };
    // SAFETY: `opened` is the handle `OpenProcess` returned above and
    // has not been closed; this is its one close, and nothing borrows it
    // afterwards.
    let _closed = unsafe { CloseHandle(opened) };
    queried.ok()?;
    let full = path.get(..usize::try_from(length).ok()?)?;
    let full = String::from_utf16_lossy(full);
    // The scope file lists `notepad.exe`, not a path: a person writing
    // an allowlist should not have to know where a program was installed.
    Some(
        full.rsplit(['\\', '/'])
            .next()
            .unwrap_or(full.as_str())
            .to_owned(),
    )
}

/// Where the window is on the virtual screen.
#[expect(
    unsafe_code,
    reason = "a window rectangle is written back through an out-pointer, which is an FFI convention with no safe wrapper"
)]
fn rectangle(handle: HWND) -> Result<Bounds, Refusal> {
    let mut rect = RECT::default();
    // SAFETY: `handle` is an HWND from this call's enumeration, and
    // `rect` is a live, uniquely owned local of exactly the type Win32
    // writes through this pointer. A window that closed makes the call
    // fail, which the `?` below turns into a refusal.
    unsafe { GetWindowRect(handle, std::ptr::from_mut(&mut rect)) }.map_err(|err| {
        fault::win32(
            "measure where the window is",
            "call `desktop.windows` again; the window may have closed since it was named",
            &err,
        )
    })?;
    Bounds::from_corners(rect.left, rect.top, rect.right, rect.bottom)
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

    /// What can be asserted on a machine with no desktop of its own —
    /// a build server, say — is that enumerating answers rather than
    /// crashing, and that everything it answers with is nameable. What
    /// *is* open is this machine's business and is not asserted
    /// (desktop-SPEC.md §16.2).
    #[test]
    fn every_window_this_lists_can_be_named_by_a_caller() {
        let listed = desktop().expect("enumerating this desktop answers");
        for window in &listed {
            assert!(!window.named.title.is_empty(), "a window with no title");
            assert!(!window.named.process.is_empty(), "a window with no process");
            assert!(window.bounds.width() > 0);
            assert!(window.bounds.height() > 0);
        }
    }

    /// A process is reported by its file name, because that is what a
    /// person writes in `DESKTOP.toml`. A full path there would make the
    /// allowlist depend on where a program was installed.
    #[test]
    fn a_process_is_named_the_way_the_scope_file_names_one() {
        for window in desktop().expect("enumerating this desktop answers") {
            let process = window.named.process;
            assert!(!process.contains('\\'), "{process} is a path");
            assert!(!process.contains('/'), "{process} is a path");
        }
    }
}
