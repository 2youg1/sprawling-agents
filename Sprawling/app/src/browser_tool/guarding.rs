// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::net::SocketAddr;
use std::sync::Mutex;

use browser::OwnListeners;

static LISTENING: Mutex<Vec<SocketAddr>> = Mutex::new(Vec::new());

#[must_use = "the address is taken back when this is dropped"]
pub(crate) struct Served(SocketAddr);

pub(crate) fn serve(at: SocketAddr) -> Served {
    LISTENING
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .push(at);
    Served(at)
}

impl Drop for Served {
    fn drop(&mut self) {
        let mut listening = LISTENING
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if let Some(index) = listening.iter().position(|at| *at == self.0) {
            listening.swap_remove(index);
        }
    }
}

pub(super) fn own() -> OwnListeners {
    OwnListeners::new(
        LISTENING
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .iter()
            .copied(),
    )
}
