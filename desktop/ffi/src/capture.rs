// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! One window's own pixels, as `PrintWindow` draws them: top-down,
//! 32-bit BGRA, every row the window has or none.
//!
//! What the pixels mean — that alpha is noise, that an all-black picture
//! is a failure — is the server's to decide; this hands back the bytes
//! GDI wrote and nothing else.

use winsafe::co;

use crate::boundary;
use crate::ended::{self, Failure};
use crate::leaf;
use crate::step::Step;

/// The window's pixels at `width` by `height`.
///
/// # Errors
/// [`Failure::At`] with `Measuring` for a side that is not positive or a
/// size that overflows, `NoWindow` for a null handle, and the step of
/// the GDI call that refused otherwise; `ShortRows` when GDI read back
/// fewer rows than the window has.
pub fn pixels(window: &winsafe::HWND, width: i32, height: i32) -> Result<Vec<u8>, Failure> {
    let bytes = boundary::bitmap_bytes(width, height).ok_or(Failure::At {
        step: Step::Measuring,
        code: co::ERROR::SUCCESS,
    })?;
    let mut into = vec![0_u8; bytes];
    let mut code = co::ERROR::SUCCESS;
    // SAFETY: `into` is `bytes` initialised bytes lent to this call
    // alone, and `bytes` is what the leaf's own rule gives for `width`
    // by `height`, which the leaf checks again before GDI writes a row.
    // `window.ptr()` is only handed to GDI, which answers a handle that
    // names no window with a failure rather than reading through it.
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
