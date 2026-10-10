// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Builds the console's Zig leaf with the standard library alone: `zig
//! build-lib` into this package's `OUT_DIR`, linked as a static library,
//! on every target the binary ships for (`crates/console_ffi/Spec.lean`
//! D1).
//!
//! Two checks run before anything is compiled, because they are cheaper
//! to read here than as a wrong frame: the scene's words in
//! `zig/part.zig` are the ones `src/part.rs` defines, and the `zig` on
//! the search path is the version `zig-version` pins, which inside this
//! repository is also the version the desktop's leaf pins.

use std::path::{Path, PathBuf};
use std::process::Command;

include!("src/part.rs");

/// The file that pins the Zig version this leaf is built with.
const PIN: &str = "zig-version";

/// The desktop leaf's pin, which a build inside the repository must
/// match; a package built from crates.io has no sibling and skips it.
const SIBLING_PIN: &str = "../desktop/ffi/zig-version";

/// The leaf's library name, which `[package] links` names as well.
const LIBRARY: &str = "sprawling_console_leaf";

fn main() -> Result<(), String> {
    println!("cargo::rerun-if-changed=zig");
    println!("cargo::rerun-if-changed=src/part.rs");
    println!("cargo::rerun-if-changed={PIN}");
    println!("cargo::rerun-if-env-changed=PATH");
    let here = PathBuf::from(variable("CARGO_MANIFEST_DIR")?);
    spelled_alike(&here)?;
    let pinned = pin(&here)?;
    pinned_zig(&pinned)?;
    let out = PathBuf::from(variable("OUT_DIR")?);
    let target = target()?;
    compiled(&here, &out, &target)?;
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
    // exist as changed, and a package built from crates.io, which has no
    // sibling, would compile the leaf again on every build.
    if sibling.exists() {
        println!("cargo::rerun-if-changed={SIBLING_PIN}");
        let theirs = read(&sibling)?;
        if theirs != pinned {
            return Err(format!(
                "{PIN} pins Zig {pinned} and {SIBLING_PIN} pins {theirs}; the workspace builds both leaves with one Zig, so write the same version in both"
            ));
        }
    }
    Ok(pinned)
}

/// Refuses a `zig/part.zig` that is not the rendering of `src/part.rs`.
fn spelled_alike(here: &Path) -> Result<(), String> {
    let zig = here.join("zig").join("part.zig");
    let written =
        std::fs::read_to_string(&zig).map_err(|err| format!("read {}: {err}", zig.display()))?;
    let wanted = rendered();
    if written.replace("\r\n", "\n").contains(&wanted) {
        return Ok(());
    }
    Err(format!(
        "zig/part.zig does not spell the words src/part.rs defines; replace its enums with:\n{wanted}"
    ))
}

/// The Zig enums `src/part.rs` spells, one field per line.
fn rendered() -> String {
    fn one(name: &str, width: &str, fields: Vec<(String, u32)>) -> String {
        let fields: String = fields
            .iter()
            .map(|(field, number)| format!("    {field} = {number},\n"))
            .collect();
        format!("pub const {name} = enum({width}) {{\n{fields}}};\n")
    }
    [
        one(
            "Part",
            "u8",
            Part::ALL
                .iter()
                .map(|it| (format!("{it:?}"), u32::from(it.number())))
                .collect(),
        ),
        one(
            "Outcome",
            "u8",
            Outcome::ALL
                .iter()
                .map(|it| (format!("{it:?}"), u32::from(it.number())))
                .collect(),
        ),
        one(
            "Ending",
            "u8",
            Ending::ALL
                .iter()
                .map(|it| (format!("{it:?}"), u32::from(it.number())))
                .collect(),
        ),
        one(
            "Verdict",
            "u8",
            Verdict::ALL
                .iter()
                .map(|it| (format!("{it:?}"), u32::from(it.number())))
                .collect(),
        ),
        one(
            "Status",
            "u32",
            Status::ALL
                .iter()
                .map(|it| (format!("{it:?}"), it.number()))
                .collect(),
        ),
    ]
    .join("\n")
}

/// Refuses a missing `zig`, or one of another version than the pin.
#[expect(clippy::disallowed_methods, reason = "build script (child D4)")]
fn pinned_zig(pinned: &str) -> Result<(), String> {
    let install = format!(
        "install Zig {pinned} (https://ziglang.org/download/), which builds crates/console_ffi's \
         leaf on every platform; `sprawling doctor` lists it with the other tools"
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

/// The Zig target for this Rust target, the file name its static library
/// takes there, and whether the code must be position-independent.
struct Target {
    triple: String,
    file: String,
    position_independent: bool,
}

fn target() -> Result<Target, String> {
    let arch = match variable("CARGO_CFG_TARGET_ARCH")?.as_str() {
        "x86_64" => "x86_64",
        "aarch64" => "aarch64",
        other => {
            return Err(format!(
                "the console's Zig leaf is built for x86_64 and aarch64, not {other}"
            ));
        }
    };
    let env = variable("CARGO_CFG_TARGET_ENV")?;
    let os = variable("CARGO_CFG_TARGET_OS")?;
    let unix = |system: &str| Target {
        triple: format!("{arch}-{system}"),
        file: format!("lib{LIBRARY}.a"),
        position_independent: true,
    };
    match (os.as_str(), env.as_str()) {
        ("windows", "gnu") => Ok(Target {
            triple: format!("{arch}-windows-gnu"),
            file: format!("lib{LIBRARY}.a"),
            position_independent: false,
        }),
        ("windows", _) => Ok(Target {
            triple: format!("{arch}-windows-msvc"),
            file: format!("{LIBRARY}.lib"),
            position_independent: false,
        }),
        ("linux", "musl") => Ok(unix("linux-musl")),
        ("linux", _) => Ok(unix("linux-gnu")),
        ("macos", _) => Ok(unix("macos")),
        (other, _) => Err(format!(
            "the console's Zig leaf is built for Windows, Linux and macOS, not {other}"
        )),
    }
}

/// `zig build-lib` of `zig/leaf.zig`. `ReleaseSafe` keeps every bounds
/// and overflow check of the leaf in the shipped binary, where a failed
/// one traps.
#[expect(clippy::disallowed_methods, reason = "build script (child D4)")]
fn compiled(here: &Path, out: &Path, target: &Target) -> Result<(), String> {
    let mut zig = Command::new("zig");
    zig.current_dir(here).args([
        "build-lib",
        "-target",
        &target.triple,
        "-O",
        "ReleaseSafe",
        "--name",
        LIBRARY,
    ]);
    if target.position_independent {
        zig.arg("-fPIC");
    }
    let built = zig
        .arg("--cache-dir")
        .arg(out.join("zig-cache"))
        .arg(format!("-femit-bin={}", out.join(&target.file).display()))
        .arg("zig/leaf.zig")
        .output()
        .map_err(|err| format!("zig did not start: {err}"))?;
    if built.status.success() {
        return Ok(());
    }
    Err(format!(
        "zig build-lib refused the console's leaf:\n{}",
        String::from_utf8_lossy(&built.stderr)
    ))
}
