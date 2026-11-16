// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Confirmed resource deletion (`crates/runtime/spec/Tools/Exec/Container.lean`, D52).

use std::path::PathBuf;

use kernel::AxError;
use serde::{Deserialize, Serialize};

use super::{ContainerEngine, ContainerRuntime, control, denied};

#[derive(Clone, Copy)]
pub(super) enum Confirmation {
    Absence,
    Deletion,
}

#[derive(Clone, Serialize, Deserialize)]
pub(super) struct Cleanup {
    pub(super) program: PathBuf,
    engine: ContainerEngine,
    pub(super) name: String,
    pub(super) copy: PathBuf,
}

impl Cleanup {
    pub(super) fn registered(runtime: &ContainerRuntime, name: String, copy: PathBuf) -> Self {
        Self {
            program: runtime.program.clone(),
            engine: runtime.engine,
            name,
            copy,
        }
    }

    pub(super) fn remove(&self, confirmation: Confirmation) -> Result<(), AxError> {
        let mut remove = child::command(&self.program);
        remove.args(["rm", "--force", "--volumes"]);
        match self.engine {
            ContainerEngine::Docker => {}
            ContainerEngine::Podman => {
                remove.args(["--time", "0"]);
            }
        }
        remove.arg(&self.name);
        let removed = control::output(&mut remove, control::Wait::Bounded)?;
        if !removed.status.success() {
            if matches!(confirmation, Confirmation::Deletion) {
                return Err(denied(
                    "remove a possibly late-created container",
                    &self.name,
                ));
            }
            let mut inventory = child::command(&self.program);
            inventory.args(["container", "ls", "--all", "--format", "{{.Names}}"]);
            let names = control::checked(&mut inventory)?;
            let names = std::str::from_utf8(&names)
                .map_err(|err| denied("confirm container removal", err))?;
            if names.lines().any(|name| name.trim() == self.name) {
                return Err(denied(
                    "remove the owned container",
                    format!(
                        "{}: {}",
                        self.name,
                        String::from_utf8_lossy(&removed.stderr).trim()
                    ),
                ));
            }
        }
        Ok(())
    }

    pub(super) fn release_copy(&self) -> Result<(), AxError> {
        match std::fs::remove_dir_all(&self.copy) {
            Ok(()) => Ok(()),
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(err) => Err(denied("release the container copy", err)),
        }
    }
}
