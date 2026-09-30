// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Reproducible-build fixture: build the release binary twice from the
//! same tree and compare bytes. Release item three.
//!
//! Scope is stated rather than implied: the default rebuilds the
//! `sprawling` crate only (dependencies stay cached), which proves the
//! final compile, the client embed and the link are deterministic.
//! `--full` clears the whole target first - the CI-grade variant, at
//! several minutes.
//!
//! The embed chain is deterministic by construction: the gzip mtime is
//! zeroed and the tables are sorted. The link step is not deterministic
//! on its own, and neither is the debug information: link.exe mints a
//! PE header timestamp and a debug-directory PDB GUID per run, and a
//! compile records the absolute path of every source it read. Those
//! three are settled by `/Brepro` and `--remap-path-prefix` in
//! `.cargo/config.toml`, where every build of this tree reads them,
//! rather than by the two commands below - a switch this fixture passed
//! to its own builds would prove an artifact no release ships.
//!
//! `platforms.yml` runs this nightly, which is the only automated
//! reader: two links are minutes, and nobody waits on the verdict.

use std::path::Path;
use std::process::Command;

use crate::report::XtaskError;

pub(crate) fn run(root: &Path, full: bool) -> Result<String, XtaskError> {
    let first = build_and_hash(root, full)?;
    let second = build_and_hash(root, false)?;
    if first == second {
        let scope = if full {
            "full tree rebuilt once, then the crate alone"
        } else {
            "crate + embed + link (dependencies cached)"
        };
        Ok(format!("reproducible: {first} twice ({scope})"))
    } else {
        Err(XtaskError::Doc {
            file: "target/release/sprawling".to_owned(),
            msg: format!(
                "two builds of one tree differ: {first} then {second}; a nondeterministic \
                 input (path, timestamp, env) has entered the build"
            ),
        })
    }
}

fn build_and_hash(root: &Path, full_clean_first: bool) -> Result<String, XtaskError> {
    if full_clean_first {
        drive(root, &["clean", "--release"])?;
    } else {
        drive(root, &["clean", "--release", "-p", "sprawling"])?;
    }
    drive(root, &["build", "--release", "-p", "sprawling", "--locked"])?;
    let binary = ["sprawling.exe", "sprawling"]
        .into_iter()
        .map(|name| root.join("target").join("release").join(name))
        .find(|p| p.is_file())
        .ok_or_else(|| XtaskError::Doc {
            file: "target/release".to_owned(),
            msg: "no release binary after a successful build".to_owned(),
        })?;
    let bytes = std::fs::read(&binary).map_err(|source| XtaskError::Io {
        path: binary.display().to_string(),
        source,
    })?;
    Ok(kernel::B3Hash::digest(&bytes).to_string())
}

fn drive(root: &Path, args: &[&str]) -> Result<(), XtaskError> {
    let status = Command::new("cargo")
        .args(args)
        .current_dir(root)
        .status()
        .map_err(|source| XtaskError::Io {
            path: format!("cargo {}", args.join(" ")),
            source,
        })?;
    if status.success() {
        Ok(())
    } else {
        Err(XtaskError::Doc {
            file: format!("cargo {}", args.join(" ")),
            msg: "the build failed; fix it before judging reproducibility".to_owned(),
        })
    }
}
