// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Seatbelt command placement; specified by
//! `crates/runtime/spec/Tools/Exec/NativeMacos.lean`.

use std::path::Path;
use std::process::Command;

use kernel::{AxCode, AxError};

const PROFILE: &str = "(version 1)\n(deny default)\n\
    (allow file-read*)\n\
    (allow process-exec process-fork sysctl-read mach-lookup)\n\
    (allow process-info-setcontrol (target self))\n\
    (allow file-write* (subpath (param \"WORKDIR\")))\n\
    (allow file-write-data (literal \"/dev/null\"))\n\
    (deny network*)\n";

/// Preserves argv and the cleared environment while placing writes in `copy`.
/// The copy must be a real, non-root directory with a UTF-8 canonical path.
pub(super) fn wrap(wrapper: &Path, copy: &Path, command: &Command) -> Result<Command, AxError> {
    let canonical = copy
        .canonicalize()
        .map_err(|error| denied(format!("cannot resolve copy {}: {error}", copy.display())))?;
    if !canonical.is_dir() || canonical.parent().is_none() {
        return Err(denied(format!(
            "copy {} must be a non-root directory",
            canonical.display()
        )));
    }
    let copy = canonical
        .to_str()
        .ok_or_else(|| denied(format!("copy {} is not UTF-8", canonical.display())))?;
    if command.get_program().is_empty() {
        return Err(denied("target program is empty".to_owned()));
    }
    let mut wrapped = Command::new(wrapper);
    wrapped
        .env_clear()
        .current_dir(&canonical)
        .arg("-p")
        .arg(PROFILE)
        .arg("-D")
        .arg(format!("WORKDIR={copy}"))
        .arg("--")
        .arg(command.get_program())
        .args(command.get_args());
    for (name, value) in command.get_envs() {
        match value {
            Some(value) => {
                wrapped.env(name, value);
            }
            None => {
                wrapped.env_remove(name);
            }
        }
    }
    Ok(wrapped)
}

/// Checks this exact policy before a caller launches its target under it.
/// Every invocation still carries the profile if permissions change after this probe.
pub(super) fn probe(wrapper: &Path, copy: &Path) -> Result<(), AxError> {
    let output = wrap(wrapper, copy, &Command::new("/usr/bin/true"))?
        .output()
        .map_err(|error| denied(format!("{}: {error}", wrapper.display())))?;
    if output.status.success() {
        Ok(())
    } else {
        Err(denied(format!(
            "{}: {}: {}",
            wrapper.display(),
            output.status,
            String::from_utf8_lossy(&output.stderr).trim()
        )))
    }
}

fn denied(subject: String) -> AxError {
    AxError::failure(AxCode::SandboxDenied, "initialize macOS Seatbelt confinement", subject)
        .with_recovery("make the macOS native sandbox available, or explicitly select another sandbox arm; the target was not launched by this placement")
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "test code"
)]
mod tests;
