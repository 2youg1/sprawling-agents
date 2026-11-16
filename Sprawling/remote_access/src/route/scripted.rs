// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::net::SocketAddr;

use kernel::AxError;

use super::{Opened, Route};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RouteCall {
    Open(SocketAddr),
    Close,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScriptedRoute {
    answer: Opened,
    calls: Vec<RouteCall>,
}

impl ScriptedRoute {
    #[must_use]
    pub fn new(answer: Opened) -> Self {
        Self {
            answer,
            calls: Vec::new(),
        }
    }

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
