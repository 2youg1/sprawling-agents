// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::path::{Path, PathBuf};
use std::process::Command;

const LIBRARY: &str = "sprawling_calyx_shell";
const FACE: &str = "calyx";

fn main() -> Result<(), String> {
    println!("cargo::rerun-if-env-changed=DOCS_RS");
    if std::env::var_os("DOCS_RS").is_some() {
        return Ok(());
    }
    println!("cargo::rerun-if-changed=build.zig");
    println!("cargo::rerun-if-changed=shell");
    println!("cargo::rerun-if-changed=ffi/zig");

    let here = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").map_err(|e| e.to_string())?);
    let out = PathBuf::from(std::env::var("OUT_DIR").map_err(|e| e.to_string())?);
    built(&here, &out)?;
    println!(
        "cargo::rustc-link-search=native={}",
        out.join("lib").display()
    );
    println!("cargo::rustc-link-lib=static={LIBRARY}");
    let target_os = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    if target_os == "linux" || target_os == "macos" {
        println!("cargo::rustc-link-lib=dylib=c");
        println!("cargo::rustc-link-lib=dylib=pthread");
        println!("cargo::rustc-link-lib=dylib=m");
    }
    install_face(&out)?;
    Ok(())
}

#[expect(clippy::disallowed_methods, reason = "build script (child D4)")]
fn built(here: &Path, out: &Path) -> Result<(), String> {
    let status = Command::new("zig")
        .current_dir(here)
        .args(["build", "-Doptimize=safe", "--prefix"])
        .arg(out)
        .status()
        .map_err(|err| format!("zig did not start: {err}"))?;
    if status.success() {
        let lib = out.join("lib");
        if lib.join(format!("lib{LIBRARY}.a")).exists()
            || lib.join(format!("{LIBRARY}.lib")).exists()
        {
            return Ok(());
        }
        return Err(format!(
            "zig build succeeded but {LIBRARY} was not installed under {}",
            lib.display()
        ));
    }
    Err(format!(
        "zig build refused the Calyx shell (status {status})"
    ))
}

fn install_face(out: &Path) -> Result<(), String> {
    let name = if cfg!(windows) {
        format!("{FACE}.exe")
    } else {
        FACE.to_owned()
    };
    let built = out.join("bin").join(&name);
    if !built.is_file() {
        return Err(format!(
            "zig build succeeded but `{name}` was not installed under {}",
            out.join("bin").display()
        ));
    }
    let profile = profile_dir(out)?;
    let dest = profile.join(&name);
    std::fs::copy(&built, &dest).map_err(|err| {
        format!(
            "copy Zig `{name}` into cargo target dir {}: {err}",
            dest.display()
        )
    })?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt as _;
        let mut perms = std::fs::metadata(&dest)
            .map_err(|err| format!("stat {}: {err}", dest.display()))?
            .permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(&dest, perms)
            .map_err(|err| format!("chmod {}: {err}", dest.display()))?;
    }
    println!("cargo::rerun-if-changed={}", built.display());
    Ok(())
}

fn profile_dir(out: &Path) -> Result<PathBuf, String> {
    // OUT_DIR = <target>/<profile>/build/<pkg>/out → profile dir is ../../..
    out.join("../../..")
        .canonicalize()
        .map_err(|err| format!("resolve cargo profile dir from {}: {err}", out.display()))
}
