// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! One refusal shape for every way a layer can fail to be read, so the
//! recovery line is written once and cannot drift between callers.

use kernel::{AxCode, AxError};

/// The refusal a layer answers with, naming what this build reads.
///
/// `subject` is the caller's: which file, and which key in it.
pub(super) fn refuse(subject: String) -> AxError {
    // The settings are `Effort`'s to list. A recovery line that spelled
    // them again would be the second place a seventh setting has to be
    // remembered, and the one nobody remembers.
    let efforts: Vec<&str> = kernel::Effort::ALL.iter().map(|one| one.as_str()).collect();
    AxError::failure(AxCode::ConfigInvalid, "read a configuration layer", subject).with_recovery(
        format!(
            "this version reads four sections: `[model] name = \"<model id>\"`, \
             `[model] effort = \"{}\"`, ",
            efforts.join("|")
        ) + "\
         `[sandbox] shell = <bool>, fuel = <integer>, mounts = [<path>], \
         env_passthrough = [<variable name>], trusted = [<server label>]`, \
         `[[mcp]] label = <lowercase>, and either command = <program> with args = [<argument>] \
         and env = { NAME = \"value\" }, or url = <https url> with transport = \"http\"|\"sse\" \
         and headers = { Name = \"value\" }`, and \
         `[skills] shelves = [<directory>]`; a value on either table may be a \
         `secret:realm/name` reference",
    )
}
