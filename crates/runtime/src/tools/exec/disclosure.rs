// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Execution described from its current confinement and shell.

use super::{ExecSetup, confinement::Confined};

pub(super) fn describe(setup: &ExecSetup, confinement: &Confined) -> String {
    format!(
        "Executes a command, or a Python script. A command runs in this machine's \
         confinement, {}. Ask for `where: host` to run one outside it. Use `read` and \
         `search` for what is already written here, and `edit` to change a file: a file a \
         command prints or writes does not count as read for `edit`.{}",
        confinement.statement(),
        setup.shell.statement()
    )
}
