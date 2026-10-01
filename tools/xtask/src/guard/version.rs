// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The version every one of the workspace's own packages is pinned to
//! (xtask-SPEC.md section 8-49).

use crate::report::Violation;

pub(super) fn pins(_workspace: &toml::Value, _members: &[(String, toml::Value)]) -> Vec<Violation> {
    Vec::new()
}

#[cfg(test)]
mod tests;
