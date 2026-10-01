// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The window a contract test opens in its own process, read in this
//! package's own terms.
//!
//! Opening the window is `desktop_ffi::fixture`'s, because it costs
//! `unsafe` and this package holds none, its tests included
//! (desktop-SPEC.md section 12.14). What is left here is the rectangle
//! the window occupies, as the `Bounds` the contracts compare with.

use super::geometry::Bounds;

/// One open window, closed when this is dropped.
pub(crate) struct Opened {
    window: desktop_ffi::fixture::Opened,
    bounds: Bounds,
}

impl Opened {
    /// Opens a window titled `title` whose top-left corner is at
    /// `(left, top)` on the screen, holding one button captioned
    /// `button` when one is given.
    pub(crate) fn at(title: &str, left: i32, top: i32, button: Option<&str>) -> Opened {
        let bounds = Bounds::from_corners(
            left,
            top,
            left.checked_add(desktop_ffi::fixture::WIDTH).unwrap(),
            top.checked_add(desktop_ffi::fixture::HEIGHT).unwrap(),
        )
        .unwrap();
        Opened {
            window: desktop_ffi::fixture::Opened::at(title, left, top, button),
            bounds,
        }
    }

    /// The window's handle, as the address it is.
    pub(crate) fn address(&self) -> usize {
        self.window.address()
    }

    /// The window's handle, as the calls of this package take it.
    pub(crate) fn handle(&self) -> winsafe::HWND {
        self.window.handle()
    }

    /// Where the window is on the screen.
    pub(crate) fn bounds(&self) -> Bounds {
        self.bounds
    }
}
