// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Builds the read window's Zig leaf with the standard library alone:
//! `zig build-lib` into this package's `OUT_DIR`, linked as a static
//! library, on a Windows target and nowhere else
//! (`crates/read_ffi/Spec.lean` D1).
//!
//! The `zig` on the search path must be the version `zig-version` pins,
//! which inside this repository is also the version the other two leaves
//! pin; a package built from crates.io has no sibling and skips that
//! comparison.

use std::path::{Path, PathBuf};
use std::process::Command;

/// The file that pins the Zig version this leaf is built with.
const PIN: &str = "zig-version";

/// The desktop leaf's pin, which a build inside the repository must
/// match; a package built from crates.io has no sibling and skips it.
const SIBLING_PIN: &str = "../desktop/ffi/zig-version";

/// The leaf's library name, which `[package] links` names as well.
const LIBRARY: &str = "sprawling_read_leaf";

fn main() -> Result<(), String> {
    println!("cargo::rerun-if-changed=zig");
    println!("cargo::rerun-if-changed={PIN}");
    let here = PathBuf::from(variable("CARGO_MANIFEST_DIR")?);
    let pinned = pin(&here)?;
    if variable("CARGO_CFG_TARGET_OS")? != "windows" {
        return Ok(());
    }
    pinned_zig(&pinned)?;
    let out = PathBuf::from(variable("OUT_DIR")?);
    let (triple, file) = target()?;
    compiled(&here, &out, &triple, &file)?;
    println!("cargo::rustc-link-search=native={}", out.display());
    println!("cargo::rustc-link-lib=static={LIBRARY}");
    Ok(())
}

fn variable(name: &str) -> Result<String, String> {
    std::env::var(name).map_err(|err| format!("cargo set no {name}: {err}"))
}

/// The pinned version, refused when the desktop's pin beside it differs.
fn pin(here: &Path) -> Result<String, String> {
    let read = |path: &Path| {
        std::fs::read_to_string(path)
            .map(|text| text.trim().to_owned())
            .map_err(|err| format!("read {}: {err}", path.display()))
    };
    let pinned = read(&here.join(PIN))?;
    let sibling = here.join(SIBLING_PIN);
    // Named only when it is there: cargo reads a path that does not
    // exist as changed, and a package from crates.io has no sibling.
    if sibling.exists() {
        println!("cargo::rerun-if-changed={SIBLING_PIN}");
        let theirs = read(&sibling)?;
        if theirs != pinned {
            return Err(format!(
                "{PIN} pins Zig {pinned} and {SIBLING_PIN} pins {theirs}; the workspace builds every leaf with one Zig, so write the same version in both"
            ));
        }
    }
    Ok(pinned)
}

/// Refuses a missing `zig`, or one of another version than the pin.
#[expect(clippy::disallowed_methods, reason = "build script (child D4)")]
fn pinned_zig(pinned: &str) -> Result<(), String> {
    let install = format!(
        "install Zig {pinned} (https://ziglang.org/download/), which builds crates/read_ffi's \
         leaf on Windows; `sprawling doctor` lists it with the other tools"
    );
    let asked = Command::new("zig")
        .arg("version")
        .output()
        .map_err(|err| format!("zig did not start: {err}; {install}"))?;
    let found = String::from_utf8_lossy(&asked.stdout).trim().to_owned();
    if asked.status.success() && found == pinned {
        return Ok(());
    }
    Err(format!(
        "zig answered `{found}` where {PIN} pins `{pinned}`; {install}"
    ))
}

/// The Zig target for this Rust target, and the file name its static
/// library takes there.
fn target() -> Result<(String, String), String> {
    let arch = match variable("CARGO_CFG_TARGET_ARCH")?.as_str() {
        "x86_64" => "x86_64",
        "aarch64" => "aarch64",
        other => {
            return Err(format!(
                "the read window's Zig leaf is built for x86_64 and aarch64, not {other}"
            ));
        }
    };
    let env = variable("CARGO_CFG_TARGET_ENV")?;
    match env.as_str() {
        "gnu" => Ok((format!("{arch}-windows-gnu"), format!("lib{LIBRARY}.a"))),
        _ => Ok((format!("{arch}-windows-msvc"), format!("{LIBRARY}.lib"))),
    }
}

/// `zig build-lib` of `zig/window.zig`. `ReleaseSafe` keeps every bounds
/// and overflow check of the leaf in the shipped binary, where a failed
/// one traps.
#[expect(clippy::disallowed_methods, reason = "build script (child D4)")]
fn compiled(here: &Path, out: &Path, triple: &str, file: &str) -> Result<(), String> {
    let built = Command::new("zig")
        .current_dir(here)
        .args([
            "build-lib",
            "-target",
            triple,
            "-O",
            "ReleaseSafe",
            "--name",
            LIBRARY,
        ])
        .arg("--cache-dir")
        .arg(out.join("zig-cache"))
        .arg(format!("-femit-bin={}", out.join(file).display()))
        .arg("zig/window.zig")
        .output()
        .map_err(|err| format!("zig did not start: {err}"))?;
    if built.status.success() {
        return Ok(());
    }
    Err(format!(
        "zig build-lib refused the read window's leaf:\n{}",
        String::from_utf8_lossy(&built.stderr)
    ))
}
