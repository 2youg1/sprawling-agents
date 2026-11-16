// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use winsafe::co;
use winsafe::prelude::Handle;

use crate::ended::{self, Failure};
use crate::leaf;
use crate::step::Step;

const FIRST_ROOM: usize = 1_024;

const SLACK: usize = 64;

const ATTEMPTS: u8 = 4;

pub fn windows() -> Result<Vec<winsafe::HWND>, Failure> {
    let mut room = FIRST_ROOM;
    let mut found = 0;
    for _attempt in 0..ATTEMPTS {
        let mut into: Vec<winsafe::HWND> = std::iter::repeat_with(|| winsafe::HWND::NULL)
            .take(room)
            .collect();
        let mut code = co::ERROR::SUCCESS;

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
