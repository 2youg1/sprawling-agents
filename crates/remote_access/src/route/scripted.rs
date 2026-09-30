// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! A route that answers a fixed address and starts nothing
//! (remote_access-SPEC.md §8-7): the second implementation of the seam,
//! for tests of whatever opens and closes routes. It records every call
//! in the order it was made, so such a test can see that it opened the
//! route before pairing and closed it when the door closed.

use std::net::SocketAddr;

use kernel::AxError;

use super::{Opened, Route};

/// One call a scripted route received.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RouteCall {
    Open(SocketAddr),
    Close,
}

/// The scripted route.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScriptedRoute {
    answer: Opened,
    calls: Vec<RouteCall>,
}

impl ScriptedRoute {
    /// A route whose every `open` answers `answer`.
    #[must_use]
    pub fn new(answer: Opened) -> Self {
        Self {
            answer,
            calls: Vec::new(),
        }
    }

    /// The calls received so far, oldest first.
    #[must_use]
    pub fn calls(&self) -> &[RouteCall] {
        &self.calls
    }
}

impl Route for ScriptedRoute {
    fn open(&mut self, local: SocketAddr) -> Result<Opened, AxError> {
        self.calls.push(RouteCall::Open(local));
        Ok(self.answer.clone())
    }

    fn close(&mut self) -> Result<(), AxError> {
        self.calls.push(RouteCall::Close);
        Ok(())
    }
}
