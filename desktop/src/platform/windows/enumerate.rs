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
//!
//! **No window handle is minted here.** The enumeration goes through
//! the Zig leaf, because `winsafe`'s `EnumWindows` is not admitted
//! (desktop-SPEC.md section 12.9); the leaf writes each handle the
//! system reports into a `winsafe::HWND` slot this server lent it, so
//! the handles arrive already typed and nothing on this side builds one
//! from a raw pointer (section 8-11). Everything after that —
//! visibility, title, process, rectangle, and every other module's use
//! of the window — goes through `winsafe`'s safe calls.

use winsafe::co;

use super::fault;
use super::geometry::Bounds;
use super::target::Named;
use crate::refusal::Refusal;

/// One window as this module found it: what it is called, and the handle
/// the tools that touch it need.
pub(crate) struct Window {
    pub(crate) named: Named,
    pub(crate) bounds: Bounds,
    pub(crate) handle: winsafe::HWND,
}

/// Every visible, titled top-level window on this desktop.
///
/// # Errors
/// Refuses when the enumeration itself will not run. A window that
/// individually will not answer is dropped from the list instead, since
/// one unreadable window is not a reason to report none.
pub(crate) fn desktop() -> Result<Vec<Window>, Refusal> {
    let handles = desktop_ffi::top_level::windows().map_err(|failure| {
        fault::leaf(
            "list the windows on this desktop",
            "this is the operating system refusing, not the scope file; try again, and if it \
             persists this desktop session may be one no program can enumerate",
            failure,
        )
    })?;
    Ok(handles.into_iter().filter_map(described).collect())
}

/// One handle, answered for — or dropped, when it will not answer.
///
/// A title that cannot be read is dropped like an empty one: a caller
/// cannot name a window by a title nobody could read.
fn described(handle: winsafe::HWND) -> Option<Window> {
    if !handle.IsWindowVisible() {
        return None;
    }
    let title = handle.GetWindowText().ok()?;
    if title.is_empty() {
        return None;
    }
    Some(Window {
        named: Named {
            title,
            process: process(&handle)?,
        },
        bounds: rectangle(&handle).ok()?,
        handle,
    })
}

/// The file name of the process that owns the window.
///
/// This asks for the narrowest right that answers the question —
/// `PROCESS_QUERY_LIMITED_INFORMATION` cannot read the process's memory
/// — and the guard closes the process handle when it goes.
fn process(handle: &winsafe::HWND) -> Option<String> {
    let (_thread, owner) = handle.GetWindowThreadProcessId();
    let opened =
        winsafe::HPROCESS::OpenProcess(co::PROCESS::QUERY_LIMITED_INFORMATION, false, owner)
            .ok()?;
    let full = opened
        .QueryFullProcessImageName(co::PROCESS_NAME::WIN32)
        .ok()?;
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
fn rectangle(handle: &winsafe::HWND) -> Result<Bounds, Refusal> {
    let rect = handle.GetWindowRect().map_err(|err| {
        fault::system(
            "measure where the window is",
            "call `desktop.windows` again; the window may have closed since it was named",
            err,
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
    use super::super::fixture::Opened;
    use super::*;

    /// The contract desktop-SPEC.md section 8-11 holds this row to,
    /// whichever interface answers it: a window this process opens is
    /// listed once, by the title it was given, the file name of this
    /// process, and the rectangle it occupies. The window sits off every
    /// monitor, so the person at this desktop does not see it.
    #[test]
    fn a_window_this_process_opens_is_listed_by_its_title_process_and_bounds() {
        let title = format!("sprawling contract enumerate {}", std::process::id());
        let opened = Opened::at(&title, -20_000, -20_000, None);
        let this_process = std::env::current_exe()
            .unwrap()
            .file_name()
            .unwrap()
            .to_string_lossy()
            .into_owned();
        let listed: Vec<(Named, Bounds)> = desktop()
            .unwrap()
            .into_iter()
            .filter(|window| window.named.title == title)
            .map(|window| (window.named, window.bounds))
            .collect();
        assert_eq!(
            listed,
            vec![(
                Named {
                    title,
                    process: this_process
                },
                opened.bounds()
            )]
        );
    }

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
