// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use winsafe::co;

use crate::leaf;

pub fn declare(awareness: u32) -> Result<(), co::HRESULT> {

    #[expect(unsafe_code, reason = "the one call that declares DPI awareness")]
    let answered = unsafe { leaf::sprawling_desktop_dpi_declare(awareness) };
    if answered == co::HRESULT::S_OK {
        return Ok(());
    }
    Err(answered)
}

pub fn awareness() -> Result<u32, co::HRESULT> {
    let mut awareness = 0;

    #[expect(unsafe_code, reason = "the one call that reads DPI awareness")]
    let answered = unsafe { leaf::sprawling_desktop_dpi_awareness(&raw mut awareness) };
    if answered == co::HRESULT::S_OK {
        return Ok(awareness);
    }
    Err(answered)
}
