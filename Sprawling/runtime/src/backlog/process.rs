// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! A member's actual execution owner (`crates/runtime/spec/Tools/Exec/Container.lean`).

use std::process::Child;

use kernel::{AxCode, AxError};

use crate::tools::ContainerLease;

use super::Exit;

pub(super) enum Process {
    Host(Child),
    #[cfg(windows)]
    Native(Box<desktop_ffi::confinement::OwnedProcess>),
    Container(Box<ContainerLease>),
}

impl Process {
    pub(super) fn client(&self) -> Option<&Child> {
        match self {
            Self::Host(child) => Some(child),
            #[cfg(windows)]
            Self::Native(_) => None,
            Self::Container(lease) => lease.client(),
        }
    }

    pub(super) fn poll(&mut self) -> Result<Option<Exit>, AxError> {
        match self {
            Self::Host(child) => Ok(Exit::polled(child)),
            #[cfg(windows)]
            Self::Native(child) => child
                .try_wait()
                .map(|exit| {
                    exit.map(|code| Exit::Ended {
                        code: i32::from_ne_bytes(code.to_ne_bytes()),
                    })
                })
                .map_err(|err| {
                    crate::tools::native_windows::denied("poll the native process", err)
                }),
            Self::Container(lease) => lease.poll(),
        }
    }

    pub(super) fn stop(&mut self) -> Result<(), AxError> {
        match self {
            Self::Host(child) => child.kill().map_err(|err| {
                AxError::failure(
                    AxCode::ToolUnavailable,
                    "stop the backlog process",
                    err.to_string(),
                )
                .with_recovery("retry Halt; the backlog retains this process")
            }),
            #[cfg(windows)]
            Self::Native(child) => child.kill().map_err(|err| {
                crate::tools::native_windows::denied("stop the native process tree", err)
            }),
            Self::Container(lease) => lease.stop(),
        }
    }
}
