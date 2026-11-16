// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use super::geometry::Bounds;

pub(crate) struct Opened {
    window: desktop_ffi::fixture::Opened,
    bounds: Bounds,
}

impl Opened {

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

    pub(crate) fn address(&self) -> usize {
        self.window.address()
    }

    pub(crate) fn handle(&self) -> winsafe::HWND {
        self.window.handle()
    }

    pub(crate) fn bounds(&self) -> Bounds {
        self.bounds
    }
}
