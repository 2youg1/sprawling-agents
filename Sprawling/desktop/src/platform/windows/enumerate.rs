// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use winsafe::co;

use super::fault;
use super::geometry::Bounds;
use super::target::Named;
use crate::refusal::Refusal;

pub(crate) struct Window {
    pub(crate) named: Named,
    pub(crate) bounds: Bounds,
    pub(crate) handle: winsafe::HWND,
}

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

fn process(handle: &winsafe::HWND) -> Option<String> {
    let (_thread, owner) = handle.GetWindowThreadProcessId();
    let opened =
        winsafe::HPROCESS::OpenProcess(co::PROCESS::QUERY_LIMITED_INFORMATION, false, owner)
            .ok()?;
    let full = opened
        .QueryFullProcessImageName(co::PROCESS_NAME::WIN32)
        .ok()?;

    Some(
        full.rsplit(['\\', '/'])
            .next()
            .unwrap_or(full.as_str())
            .to_owned(),
    )
}

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

    #[test]
    fn a_process_is_named_the_way_the_scope_file_names_one() {
        for window in desktop().expect("enumerating this desktop answers") {
            let process = window.named.process;
            assert!(!process.contains('\\'), "{process} is a path");
            assert!(!process.contains('/'), "{process} is a path");
        }
    }
}
