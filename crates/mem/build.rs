// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Builds `zig/` into the static library this crate links, through the
//! one tool that owns that build graph: `zig build`. Shelling out rather
//! than re-describing the library here is what keeps the Zig tree's build
//! the only place its sources and flags are listed.
//!
//! The optimize mode follows cargo's own profile: `Debug` beside a debug
//! build (every Zig safety check on, sentinels in the tests), `ReleaseFast`
//! beside a release build (mem-SPEC.md section 12, decision 6). The
//! verification that carries the safety story is the suite, not the mode:
//! `zig build -Doptimize=ReleaseFast test` runs the same tests with
//! undefined-behaviour checks.

use std::path::{Path, PathBuf};
use std::process::Command;

fn main() {
    println!("cargo::rerun-if-env-changed=ZIG");
    if let Err(msg) = build_kernel() {
        // cargo >= 1.84: `cargo::error` fails the build loudly instead of
        // leaving the linker to find a stale or missing library.
        println!("cargo::error=zig kernel build failed: {msg}");
    }
}

fn zig_dir() -> Result<PathBuf, String> {
    let manifest = env_or("CARGO_MANIFEST_DIR", ".");
    let joined = Path::new(&manifest).join("..").join("..").join("zig");
    // Canonicalized because a Windows process refuses a working directory
    // that still carries `..`.
    std::fs::canonicalize(&joined)
        .map_err(|err| format!("cannot resolve {}: {err}", joined.display()))
}

fn env_or(name: &str, fallback: &str) -> String {
    std::env::var(name).unwrap_or_else(|_| fallback.to_owned())
}

/// The Zig name for the target cargo builds for. rustc and Zig spell the
/// same target differently (`x86_64-pc-windows-msvc` is
/// `x86_64-windows-msvc` over there), and a static library built for the
/// wrong ABI fails at the final link rather than where the mistake is.
/// The triples this repository builds are spelled out here; a target
/// missing from them is refused instead of guessed.
fn zig_triple(rustc_target: &str) -> Result<&'static str, String> {
    Ok(match rustc_target {
        "x86_64-pc-windows-msvc" => "x86_64-windows-msvc",
        "aarch64-pc-windows-msvc" => "aarch64-windows-msvc",
        "x86_64-unknown-linux-gnu" => "x86_64-linux-gnu",
        "aarch64-unknown-linux-gnu" => "aarch64-linux-gnu",
        "x86_64-unknown-linux-musl" => "x86_64-linux-musl",
        "aarch64-unknown-linux-musl" => "aarch64-linux-musl",
        "x86_64-apple-darwin" => "x86_64-macos",
        "aarch64-apple-darwin" => "aarch64-macos",
        other => {
            return Err(format!(
                "no Zig triple registered for rustc target {other}: add its row to \
                 zig_triple beside the ones above"
            ));
        }
    })
}

fn build_kernel() -> Result<(), String> {
    let zig = env_or("ZIG", "zig");
    let triple = zig_triple(&env_or("TARGET", "unknown"))?;
    let optimize = match env_or("PROFILE", "debug").as_str() {
        "release" => "ReleaseFast",
        _ => "Debug",
    };
    let dir = zig_dir()?;
    for name in ["src/lib.zig", "build.zig", "build.zig.zon"] {
        println!("cargo::rerun-if-changed={}", dir.join(name).display());
    }
    let output = Command::new(&zig)
        .args([
            "build",
            &format!("-Doptimize={optimize}"),
            &format!("-Dtarget={triple}"),
        ])
        .current_dir(&dir)
        .output()
        .map_err(|err| format!("cannot run `{zig}` in {}: {err}", dir.display()))?;
    if !output.status.success() {
        let last = String::from_utf8_lossy(&output.stderr)
            .lines()
            .next_back()
            .unwrap_or("no output")
            .to_owned();
        return Err(format!(
            "`zig build {optimize}` refused: {last}; this crate needs Zig 0.16.0, which `just prereqs` names"
        ));
    }
    let out = dir.join("zig-out").join("lib");
    println!("cargo::rustc-link-search=native={}", out.display());
    println!("cargo::rustc-link-lib=static=mem");
    if triple.contains("windows") {
        // The kernel's panic and stack-trace paths read ntdll exports
        // (`LdrRegisterDllNotification` among them); the final link
        // resolves them against the import library.
        println!("cargo::rustc-link-lib=ntdll");
    }
    Ok(())
}
