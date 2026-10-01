// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! This process's DPI awareness, through shcore.
//!
//! The awareness value is the caller's: the server names the one it
//! wants from the `windows` binding's definition, and the leaf passes it
//! through, so the number is written once.

use winsafe::co;

use crate::leaf;

/// Declares `awareness` for this process.
///
/// # Errors
/// The HRESULT the operating system refused with, which includes the
/// answer for a process whose awareness was already set.
pub fn declare(awareness: u32) -> Result<(), co::HRESULT> {
    // SAFETY: the call takes one integer by value and lends the leaf no
    // memory; what it changes is this process's awareness, which is the
    // operating system's to refuse.
    #[expect(unsafe_code, reason = "the one call that declares DPI awareness")]
    let answered = unsafe { leaf::sprawling_desktop_dpi_declare(awareness) };
    if answered == co::HRESULT::S_OK {
        return Ok(());
    }
    Err(answered)
}

/// This process's awareness now.
///
/// # Errors
/// The HRESULT the operating system refused to answer with.
pub fn awareness() -> Result<u32, co::HRESULT> {
    let mut awareness = 0;
    // SAFETY: `awareness` is a live local the leaf writes one `u32`
    // into, and only when the operating system answers.
    #[expect(unsafe_code, reason = "the one call that reads DPI awareness")]
    let answered = unsafe { leaf::sprawling_desktop_dpi_awareness(&raw mut awareness) };
    if answered == co::HRESULT::S_OK {
        return Ok(awareness);
    }
    Err(answered)
}
