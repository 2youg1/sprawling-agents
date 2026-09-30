// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! A route that is a command the person wrote (remote_access-SPEC.md §8-9).

use std::net::SocketAddr;
use std::path::PathBuf;

use kernel::{AxCode, AxError, TimeoutMs};

use super::{Opened, Permanence, Route};

/// The environment variable that carries `local` to the command.
pub const LOCAL_ENV: &str = "SPRAWLING_REMOTE_LOCAL";

/// The command, as the person configured it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RouteCommand {
    pub program: PathBuf,
    pub args: Vec<String>,
}

/// The command route.
pub struct CommandRoute {
    command: RouteCommand,
    permanence: Permanence,
    patience: TimeoutMs,
}

impl CommandRoute {
    #[must_use]
    pub fn new(command: RouteCommand, permanence: Permanence, patience: TimeoutMs) -> Self {
        Self {
            command,
            permanence,
            patience,
        }
    }
}

impl Route for CommandRoute {
    fn open(&mut self, _local: SocketAddr) -> Result<Opened, AxError> {
        Err(not_built(&format!(
            "{:?} {:?} {}",
            self.command.program, self.permanence, self.patience.0
        )))
    }

    fn close(&mut self) -> Result<(), AxError> {
        Err(not_built("close"))
    }
}

fn not_built(subject: &str) -> AxError {
    AxError::failure(AxCode::StorageFatal, "open a route", subject)
        .with_recovery("the route is not built yet")
}
