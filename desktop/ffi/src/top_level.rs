// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Every top-level window on this desktop, as the handles `EnumWindows`
//! reports, minted on the far side of the boundary.
//!
//! The leaf writes each handle into a slot of a `winsafe::HWND` buffer,
//! so the only place a window handle comes into being is the one
//! `extern` call below: no `HWND::from_ptr` is written anywhere in the
//! server (desktop-SPEC.md section 8-11).

use winsafe::co;
use winsafe::prelude::Handle;

use crate::ended::{self, Failure};
use crate::leaf;
use crate::step::Step;

/// The room the first attempt lends, in handles. A desktop carries a
/// few hundred top-level windows, most of them invisible.
const FIRST_ROOM: usize = 1_024;

/// Room added past the count a short buffer reported, for the windows
/// that open between two attempts.
const SLACK: usize = 64;

/// How many times a short buffer is grown before the listing is refused.
const ATTEMPTS: u8 = 4;

/// Every top-level window, visible or not, in the order the system
/// reports them.
///
/// # Errors
/// [`Failure::At`] with `Listing` when `EnumWindows` refuses, and with
/// `NoRoom` when windows keep opening faster than the buffer grows.
pub fn windows() -> Result<Vec<winsafe::HWND>, Failure> {
    let mut room = FIRST_ROOM;
    let mut found = 0;
    for _attempt in 0..ATTEMPTS {
        let mut into: Vec<winsafe::HWND> = std::iter::repeat_with(|| winsafe::HWND::NULL)
            .take(room)
            .collect();
        let mut code = co::ERROR::SUCCESS;
        // SAFETY: `into` holds `into.len()` initialised slots, lent to
        // this call alone, and that same length is the capacity the
        // leaf is held to, so every write lands inside the vector. Each
        // value written is one `EnumWindows` handed the leaf's callback;
        // a `winsafe::HWND` wraps a pointer it never dereferences, so any
        // value the system hands over is a valid one. `found` and `code`
        // are live locals of the types the leaf writes.
        #[expect(unsafe_code, reason = "the one call that lists this desktop's windows")]
        let raw = unsafe {
            leaf::sprawling_desktop_windows(
                into.as_mut_ptr(),
                into.len(),
                &raw mut found,
                &raw mut code,
            )
        };
        let step = ended::step(raw)?;
        if step == Step::Finished {
            into.truncate(found);
            return Ok(into);
        }
        if step != Step::NoRoom {
            return Err(Failure::At { step, code });
        }
        room = found.saturating_add(SLACK);
    }
    Err(Failure::At {
        step: Step::NoRoom,
        code: co::ERROR::INSUFFICIENT_BUFFER,
    })
}
