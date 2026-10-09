// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The addresses this city listens on, registered by its listeners and
//! read by both browser tools before they act
//! (`crates/sprawling/spec/BrowserBidi.lean` §8-45-6, sprawling D74).
//!
//! The tools are built per run through a function pointer that carries no
//! state of the assembly, and the remote listener opens and closes while
//! the city runs, so the register is the process's: a listener adds its
//! address when it is bound and takes it back when it is dropped.

use std::net::SocketAddr;
use std::sync::Mutex;

use browser::OwnListeners;

/// Every address registered and not yet taken back, once per registration.
static LISTENING: Mutex<Vec<SocketAddr>> = Mutex::new(Vec::new());

/// One listener's place in the register; dropping it takes the address
/// back.
#[must_use = "the address is taken back when this is dropped"]
pub(crate) struct Served(SocketAddr);

/// Registers `at` as one of this city's listeners until the returned
/// value is dropped.
///
/// A register left locked by a dead thread is taken over here and in
/// [`own`], because the list is whole after any push or removal; refusing
/// instead would turn one dead thread into a guard that lets everything
/// through or a browser that refuses everything.
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

/// The listeners registered at this moment.
pub(super) fn own() -> OwnListeners {
    OwnListeners::new(
        LISTENING
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .iter()
            .copied(),
    )
}
