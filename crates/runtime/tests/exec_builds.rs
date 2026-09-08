// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The thing card 11.1 exists for: a resident inside the city can build
//! a Rust program.
//!
//! Before the declaration existed this failed on Windows and nowhere
//! else, which is the worst shape a defect can have. `env_clear()` plus
//! a four-name allowlist takes `%ProgramFiles(x86)%` away; rustc's MSVC
//! linker reads `vswhere.exe` from under that directory to find
//! `link.exe`, and without it falls back to a bare `link.exe` — which on
//! a machine with Git installed is coreutils' `link`, whose answer is
//! `link: missing operand`.
//!
//! The test builds a crate with no dependencies into a target directory
//! of its own, so it neither reaches the network nor waits on this
//! workspace's build lock.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "test code"
)]

use kernel::{Address, EnvVarName, Payload, Tool, ToolCall, ToolName};
use runtime::{Backlog, EchoSandbox, ExecSetup, ExecTool, Fuel};

/// What a building on this machine has to declare for a Rust build to
/// find its linker. Host facts, which is why they are the building's
/// declaration rather than this repository's constant.
#[cfg(windows)]
const DECLARED: [&str; 8] = [
    "ProgramFiles(x86)",
    "ProgramFiles",
    "ProgramData",
    "SystemRoot",
    "SystemDrive",
    "ComSpec",
    "PATHEXT",
    "LOCALAPPDATA",
];

#[cfg(not(windows))]
const DECLARED: [&str; 2] = ["HOME", "USER"];

#[test]
fn a_building_that_declares_the_names_can_build_a_rust_program() {
    let Ok(cargo) = which_cargo() else {
        // No toolchain on this machine: there is nothing to demonstrate
        // and nothing to claim.
        return;
    };
    let work = tempfile::tempdir().unwrap();
    let crate_dir = work.path().join("hello");
    std::fs::create_dir_all(crate_dir.join("src")).unwrap();
    std::fs::write(
        crate_dir.join("Cargo.toml"),
        "[package]\nname = \"hello\"\nversion = \"0.0.0\"\nedition = \"2021\"\n\n[workspace]\n",
    )
    .unwrap();
    std::fs::write(crate_dir.join("src/main.rs"), "fn main() {}\n").unwrap();

    let mut declared: Vec<EnvVarName> = DECLARED
        .iter()
        .map(|name| EnvVarName::parse(name).unwrap())
        .collect();
    // Where the build's own output goes, so it never touches this
    // workspace's target directory or its lock.
    declared.push(EnvVarName::parse("CARGO_TARGET_DIR").unwrap());
    // Also a host fact: rustup's shim reads these, and defaults them
    // from the profile directory when they are absent.
    for optional in ["CARGO_HOME", "RUSTUP_HOME", "USERPROFILE"] {
        if std::env::var(optional).is_ok() {
            declared.push(EnvVarName::parse(optional).unwrap());
        }
    }

    let setup = ExecSetup {
        workdir: crate_dir,
        mounts: Vec::new(),
        python_wasm: None,
        shell: None,
        fuel: Fuel(1_000_000),
        env_passthrough: declared,
        domain: Address::parse("lab").unwrap(),
    };
    let mut tool = ExecTool::new(setup, Box::new(EchoSandbox::new()), Backlog::new()).unwrap();

    let mut args = serde_json::Map::new();
    args.insert(
        "arm".to_owned(),
        serde_json::json!({
            "program": { "path": cargo, "args": ["build", "--offline"] }
        }),
    );
    let outcome = tool
        .invoke(&ToolCall {
            id: "c1".to_owned(),
            name: ToolName::parse("exec").unwrap(),
            args: Payload::new(args).unwrap(),
        })
        .unwrap();
    let result = serde_json::to_value(&outcome.result).unwrap();
    assert_eq!(
        result["exit_code"], 0,
        "a declared building builds Rust: {result}"
    );
}

/// `cargo` itself, resolved off the search path the child would get.
fn which_cargo() -> Result<String, ()> {
    let name = if cfg!(windows) { "cargo.exe" } else { "cargo" };
    let path = std::env::var_os("PATH").ok_or(())?;
    for dir in std::env::split_paths(&path) {
        let candidate = dir.join(name);
        if candidate.is_file() {
            return Ok(candidate.to_string_lossy().into_owned());
        }
    }
    Err(())
}
