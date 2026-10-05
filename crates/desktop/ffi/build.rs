// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Builds the Zig leaf with the standard library alone: `zig build-lib`
//! into this package's `OUT_DIR`, linked as a static library, on a
//! Windows target and nowhere else (`crates/desktop/Spec.lean` D12).
//!
//! Three checks run before anything is compiled, because they are cheaper
//! to read here than as a link error: the leaf's step vocabulary in
//! `zig/step.zig` is the one `src/step.rs` defines, and the `zig` on
//! the search path is the version `zig-version` pins; the native Record
//! fields, order and types also match the Rust declaration (desktop_ffi D5).

use std::path::{Path, PathBuf};
use std::process::Command;

include!("src/step.rs");

/// The file that pins the Zig version, read by this script, by the
/// doctor's `zig` row and by CI's install step.
const PIN: &str = "zig-version";

/// The leaf's library name, which `[package] links` names as well.
const LIBRARY: &str = "sprawling_desktop_leaf";

/// The Windows libraries the leaf's `extern` declarations name.
const SYSTEM: [&str; 6] = [
    "user32", "gdi32", "kernel32", "shcore", "advapi32", "userenv",
];

fn main() -> Result<(), String> {
    println!("cargo::rerun-if-changed=zig");
    println!("cargo::rerun-if-changed=src/step.rs");
    println!("cargo::rerun-if-changed=src/confinement.rs");
    println!("cargo::rerun-if-changed={PIN}");
    println!("cargo::rerun-if-env-changed=PATH");
    let here = PathBuf::from(variable("CARGO_MANIFEST_DIR")?);
    spelled_alike(&here)?;
    let rust_record = std::fs::read_to_string(here.join("src/confinement.rs"))
        .map_err(|err| format!("read native Rust record: {err}"))?;
    let zig_record = std::fs::read_to_string(here.join("zig/confinement_api.zig"))
        .map_err(|err| format!("read native Zig record: {err}"))?;
    record_matches(&rust_record, &zig_record)?;
    if variable("CARGO_CFG_TARGET_OS")? != "windows" {
        return Ok(());
    }
    let pinned =
        std::fs::read_to_string(here.join(PIN)).map_err(|err| format!("read {PIN}: {err}"))?;
    let pinned = pinned.trim();
    pinned_zig(pinned)?;
    let out = PathBuf::from(variable("OUT_DIR")?);
    let (triple, file) = target()?;
    compiled(&here, &out, &triple, &file)?;
    println!("cargo::rustc-link-search=native={}", out.display());
    println!("cargo::rustc-link-lib=static={LIBRARY}");
    for system in SYSTEM {
        println!("cargo::rustc-link-lib=dylib={system}");
    }
    Ok(())
}

fn variable(name: &str) -> Result<String, String> {
    std::env::var(name).map_err(|err| format!("cargo set no {name}: {err}"))
}

/// Refuses a `zig/step.zig` that is not the rendering of [`Step::ALL`].
fn spelled_alike(here: &Path) -> Result<(), String> {
    let zig = here.join("zig").join("step.zig");
    let written =
        std::fs::read_to_string(&zig).map_err(|err| format!("read {}: {err}", zig.display()))?;
    let wanted = rendered();
    if written.replace("\r\n", "\n").contains(&wanted) {
        return Ok(());
    }
    Err(format!(
        "zig/step.zig does not spell the steps src/step.rs defines; replace its enum with:\n{wanted}"
    ))
}

/// The Zig enum [`Step::ALL`] spells, one field per line.
fn rendered() -> String {
    let fields: String = Step::ALL
        .iter()
        .map(|step| format!("    {step:?} = {},\n", step.number()))
        .collect();
    format!("pub const Step = enum(u32) {{\n{fields}}};\n")
}

/// Refuses a missing `zig`, or one of another version than the pin.
fn pinned_zig(pinned: &str) -> Result<(), String> {
    let install = format!(
        "install Zig {pinned} (`winget install --id zig.zig -e --version {pinned}`), which \
         builds crates/desktop/ffi's leaf; `sprawling doctor` lists it with the other tools"
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
                "the Zig leaf is built for x86_64 and aarch64, not {other}"
            ));
        }
    };
    Ok(match variable("CARGO_CFG_TARGET_ENV")?.as_str() {
        "gnu" => (format!("{arch}-windows-gnu"), format!("lib{LIBRARY}.a")),
        _msvc => (format!("{arch}-windows-msvc"), format!("{LIBRARY}.lib")),
    })
}

/// `zig build-lib` of `zig/leaf.zig`. `ReleaseSafe` keeps every bounds
/// and overflow check of the leaf in the shipped binary, where a failed
/// one traps.
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
        .arg("zig/leaf.zig")
        .output()
        .map_err(|err| format!("zig did not start: {err}"))?;
    if built.status.success() {
        return Ok(());
    }
    Err(format!(
        "zig build-lib refused the leaf:\n{}",
        String::from_utf8_lossy(&built.stderr)
    ))
}

fn record_matches(rust: &str, zig: &str) -> Result<(), String> {
    let body = rust
        .split_once("struct Record {")
        .filter(|(prefix, _)| {
            prefix.rsplit("\n\n").next().is_some_and(|attributes| {
                attributes.trim().starts_with("#[repr(C)]")
                    && attributes
                        .lines()
                        .map(str::trim)
                        .filter(|line| !line.is_empty())
                        .all(|line| line == "#[repr(C)]" || line.starts_with("#[derive("))
            })
        })
        .and_then(|(_, tail)| tail.split_once('}'))
        .map(|(body, _)| body)
        .ok_or_else(|| {
            "read native Record: src/confinement.rs has no Record declaration".to_owned()
        })?;
    let mut fields = String::new();
    for field in body.lines().map(str::trim).filter(|line| !line.is_empty()) {
        let (name, ty) = field
            .strip_suffix(',')
            .and_then(|field| field.split_once(':'))
            .ok_or_else(|| format!("read native Record field `{field}`: expected name: type,"))?;
        let ty = match ty.trim() {
            "usize" => "usize".to_owned(),
            array => {
                let length = array
                    .strip_prefix("[u8;")
                    .and_then(|array| array.strip_suffix(']'))
                    .ok_or_else(|| {
                        format!("render native Record field `{name}`: unsupported type `{array}`")
                    })?
                    .trim()
                    .parse::<usize>()
                    .map_err(|err| {
                        format!("render native Record field `{name}` array length: {err}")
                    })?;
                format!("[{length}]u8")
            }
        };
        fields.push_str(&format!("    {}: {ty},\n", name.trim()));
    }
    let wanted = format!("pub const Record = extern struct {{\n{fields}}};");
    if zig.replace("\r\n", "\n").contains(&wanted) {
        return Ok(());
    }
    Err(format!(
        "native Record layout differs from src/confinement.rs; replace zig/confinement_api.zig's Record with:\n{wanted}"
    ))
}

#[cfg(test)]
mod tests {
    use super::record_matches;

    #[test]
    fn swapped_same_width_handles_are_rejected() {
        let rust = "#[repr(C)]
struct Record {
    job: usize,
    process: usize,
}";
        let zig = "pub const Record = extern struct {
    process: usize,
    job: usize,
};";
        assert!(record_matches(rust, zig).is_err());
    }

    #[test]
    fn matching_field_order_and_types_are_accepted() {
        let rust = "#[repr(C)]
struct Record {
    job: usize,
    phase: [u8; 32],
}";
        let zig = "pub const Record = extern struct {
    job: usize,
    phase: [32]u8,
};";
        assert_eq!(record_matches(rust, zig), Ok(()));
    }
}
