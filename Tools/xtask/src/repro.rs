// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

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

#[expect(clippy::disallowed_methods, reason = "developer tool (child D4)")]
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
