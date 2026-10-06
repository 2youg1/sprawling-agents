// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The languages held to the file rule alone, and where their files are.
//!
//! A file is read whole by the same two readers whatever it is written
//! in and wherever it sits, so the 400-line rule reaches every text
//! source this tree carries: the client, the stylesheet, a skill's page
//! and scripts, the installers, the CI scripts and the Nix flake. They
//! are found by extension rather than by directory, because a list of
//! directories misses the next one somebody creates and becomes a second
//! answer to the question of what this tree ships (tools/xtask/Spec.lean
//! §8-53, D33).
//!
//! The function rule does not reach them: measuring a function means
//! parsing its language, and no parser is added to this workspace for a
//! gate. Documentation, specifications, configuration and data are left
//! out by extension, for the reasons D33 records.

use std::path::{Path, PathBuf};

use crate::report::XtaskError;
use crate::walk;

/// The extensions measured on the file face only.
pub(super) const FILE_FACE: [&str; 15] = [
    "ts", "svelte", "css", "html", "js", "mjs", "cjs", "py", "ps1", "psm1", "sh", "bash", "cmd",
    "bat", "nix",
];

/// Every file in the tree written in a [`FILE_FACE`] language, sorted by
/// its relative path: the build directories, `.git` and nested checkouts
/// are skipped by `walk`, and the isolation zone is skipped here, since
/// the gate testifies about committed objects only.
pub(super) fn sources(root: &Path) -> Result<Vec<PathBuf>, XtaskError> {
    Ok(walk::files_with_ext(root, &FILE_FACE)?
        .into_iter()
        .filter(|file| !walk::in_isolation_zone(&walk::rel(root, file)))
        .collect())
}
