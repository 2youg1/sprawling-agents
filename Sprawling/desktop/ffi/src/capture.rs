// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use winsafe::co;

use crate::boundary;
use crate::ended::{self, Failure};
use crate::leaf;
use crate::step::Step;

pub fn pixels(window: &winsafe::HWND, width: i32, height: i32) -> Result<Vec<u8>, Failure> {
    let bytes = boundary::bitmap_bytes(width, height).ok_or(Failure::At {
        step: Step::Measuring,
        code: co::ERROR::SUCCESS,
    })?;
    let mut into = vec![0_u8; bytes];
    let mut code = co::ERROR::SUCCESS;

    #[expect(unsafe_code, reason = "the one call that draws a window into memory")]
    let raw = unsafe {
        leaf::sprawling_desktop_capture(
            window.ptr(),
            width,
            height,
            into.as_mut_ptr(),
            into.len(),
            &raw mut code,
        )
    };
    ended::finished(raw, code).map(|()| into)
}
