// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! `cargo xtask apisync`: the committed public-surface baselines of the
//! two crates whose surface is read outside this repository equal the
//! live `cargo public-api` surface. Not a gate: the nightly job runs it,
//! and whether an interface change belongs in a SPEC is a reviewer's call
//! (xtask-SPEC §8-32).

use std::path::{Path, PathBuf};
use std::process::Command;

use crate::members;
use crate::report::{Violation, XtaskError};
use crate::walk;

const BASELINE_DIR: &str = "xtask/api-baselines";

/// The crates whose public surface is a seam other code reads across a
/// process or a repository boundary, by lib name; every other crate's
/// surface is held by the compiler at its call sites.
const SEAM_CRATES: [&str; 2] = ["channels", "kernel"];

/// The live surface, normalized to trimmed non-empty lines. Derived and
/// blanket impls are omitted (-sss): they move with the toolchain, not
/// with our decisions.
fn live_api(root: &Path, package: &str) -> Result<String, XtaskError> {
    let output = Command::new("cargo")
        .args([
            "public-api",
            "-p",
            package,
            "--simplified",
            "--omit",
            "blanket-impls,auto-trait-impls,auto-derived-impls",
        ])
        .current_dir(root)
        .output()
        .map_err(|err| XtaskError::Cmd {
            cmd: "cargo public-api".to_owned(),
            msg: format!("{err}; run `just prereqs`, which names every tool this repository needs"),
        })?;
    if !output.status.success() {
        return Err(XtaskError::Cmd {
            cmd: format!("cargo public-api -p {package}"),
            msg: String::from_utf8_lossy(&output.stderr)
                .lines()
                .last()
                .unwrap_or("failed")
                .to_owned(),
        });
    }
    Ok(normalize(&String::from_utf8_lossy(&output.stdout)))
}

fn normalize(raw: &str) -> String {
    let mut lines: Vec<&str> = raw
        .lines()
        .map(str::trim_end)
        .filter(|l| !l.is_empty())
        .collect();
    // The tool's order is deterministic, but sorting makes the baseline
    // immune to ordering changes across tool versions.
    lines.sort_unstable();
    let mut joined = lines.join("\n");
    joined.push('\n');
    joined
}

fn baseline_path(root: &Path, krate: &str) -> PathBuf {
    root.join(BASELINE_DIR).join(format!("{krate}.txt"))
}

pub(crate) fn check(root: &Path) -> Result<Vec<Violation>, XtaskError> {
    let found = members::members(root)?;
    let mut violations = Vec::new();
    for krate in SEAM_CRATES {
        let path = baseline_path(root, krate);
        let rel = walk::rel(root, &path);
        let Ok(committed) = std::fs::read_to_string(&path) else {
            violations.push(Violation {
                gate: "apisync",
                location: rel,
                rule: "every seam crate carries a public-api baseline".to_owned(),
                violation: format!("no baseline for `{krate}`"),
                alternative: "run `cargo xtask apisync --write` and commit the baseline \
                              together with the SPEC"
                    .to_owned(),
            });
            continue;
        };
        let live = live_api(root, &members::find(&found, krate)?.package)?;
        if normalize(&committed) != live {
            violations.push(Violation {
                gate: "apisync",
                location: rel,
                rule: "the committed baseline equals the live public API".to_owned(),
                violation: format!("`{krate}` public API drifted from its baseline"),
                alternative: "run `cargo xtask apisync --write`, review the diff, and \
                              touch the crate SPEC in the same change-set"
                    .to_owned(),
            });
        }
    }

    Ok(violations)
}

/// `cargo xtask apisync --write`: regenerate every seam baseline. The
/// only files this command ever writes.
pub(crate) fn write(root: &Path) -> Result<(), XtaskError> {
    let dir = root.join(BASELINE_DIR);
    std::fs::create_dir_all(&dir).map_err(|source| XtaskError::Io {
        path: dir.display().to_string(),
        source,
    })?;
    let found = members::members(root)?;
    for krate in SEAM_CRATES {
        let live = live_api(root, &members::find(&found, krate)?.package)?;
        let path = baseline_path(root, krate);
        std::fs::write(&path, live).map_err(|source| XtaskError::Io {
            path: path.display().to_string(),
            source,
        })?;
        println!("baseline written: {}", walk::rel(root, &path));
    }
    Ok(())
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, reason = "test code")]
mod tests {
    use super::normalize;

    #[test]
    fn normalization_sorts_trims_and_ends_with_one_newline() {
        let raw = "pub fn b()\n\npub fn a()   \n";
        assert_eq!(normalize(raw), "pub fn a()\npub fn b()\n");
        assert_eq!(normalize(""), "\n");
    }
}
