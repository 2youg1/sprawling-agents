// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::sync::{Mutex, PoisonError};

use winsafe::co;

use crate::ended::{self, Failure};
use crate::leaf;
use crate::step::Step;

static TURN: Mutex<()> = Mutex::new(());

const FIRST_ROOM: usize = 4_096;

const ATTEMPTS: u8 = 4;

pub fn text() -> Result<Option<Vec<u16>>, Failure> {
    let _turn = TURN.lock().unwrap_or_else(PoisonError::into_inner);
    let mut room = FIRST_ROOM;
    let mut found = 0;
    for _attempt in 0..ATTEMPTS {
        let mut into = vec![0_u16; room];
        let mut code = co::ERROR::SUCCESS;

        #[expect(unsafe_code, reason = "the one call that reads the clipboard")]
        let raw = unsafe {
            leaf::sprawling_desktop_clipboard_read(
                into.as_mut_ptr(),
                into.len(),
                &raw mut found,
                &raw mut code,
            )
        };
        let step = ended::step(raw)?;
        if step == Step::Finished {
            into.truncate(found);
            return Ok(Some(into));
        }
        if step == Step::Absent {
            return Ok(None);
        }
        if step != Step::NoRoom {
            return Err(Failure::At { step, code });
        }
        room = found;
    }
    Err(Failure::At {
        step: Step::NoRoom,
        code: co::ERROR::INSUFFICIENT_BUFFER,
    })
}

pub fn put_text(units: &[u16]) -> Result<(), Failure> {
    let _turn = TURN.lock().unwrap_or_else(PoisonError::into_inner);
    let mut code = co::ERROR::SUCCESS;

    #[expect(unsafe_code, reason = "the one call that writes the clipboard")]
    let raw = unsafe {
        leaf::sprawling_desktop_clipboard_write(units.as_ptr(), units.len(), &raw mut code)
    };
    ended::finished(raw, code)
}
