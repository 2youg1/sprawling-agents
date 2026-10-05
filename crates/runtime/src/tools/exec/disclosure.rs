// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Execution described from its current confinement and shell.

use super::{ExecSetup, confinement::Confined};

pub(super) fn describe(setup: &ExecSetup, confinement: &Confined) -> String {
    format!(
        "Run a program, a Python snippet, or a shell line. A program or shell \
         line runs in this machine's confinement, {}. Ask for `where: host` to \
         run one outside it. Use `read` and `search` for what is already written here; a \
         command that prints it comes back without the version `edit` guards on.{}",
        confinement.statement(),
        setup.shell.statement()
    )
}
