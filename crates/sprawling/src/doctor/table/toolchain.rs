// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The Rust tools this repository's own recipes call (sprawling-SPEC.md
//! section 8-58).
//!
//! Data, with no branch in it. Two of them close `just check` and are
//! therefore required; the rest are called by a recipe a person runs on
//! purpose - `just gates`, `just mutants`, `just fuzz` - and each says
//! which recipe that is, so an absence reads as "this recipe will not
//! run" rather than as a fault.
//!
//! `rustup component add` is the install for the two that are parts of
//! the toolchain rather than packages; the others are one
//! `cargo install --locked` each, which is a per-user install asking
//! for no elevation.

use crate::doctor::{Detection, Need, PerPlatform, Recipe, Requirement, Tier};

use super::NOWHERE;

/// One command, spelled once and answered on all three platforms.
/// Every tool below installs the same way everywhere, so the three
/// columns would otherwise be one line copied three times.
const fn same_command(program: &'static str, args: &'static [&'static str]) -> PerPlatform<Recipe> {
    PerPlatform {
        windows: Recipe::Command { program, args },
        macos: Recipe::Command { program, args },
        linux: Recipe::Command { program, args },
    }
}

/// The two that close `just check`, then the tools one recipe each
/// calls on purpose.
pub(super) const RUSTFMT: Requirement = Requirement {
    name: "rustfmt",
    tier: Tier::Develop,
    need: Need::Required,
    enables: "`cargo fmt --check`, the first half of `just check`",
    detect: Detection::Program {
        program: "rustfmt",
        version_arg: "--version",
        places: NOWHERE,
    },
    homepage: Some("https://github.com/rust-lang/rustfmt"),
    recipe: same_command("rustup", &["component", "add", "rustfmt"]),
};

pub(super) const CLIPPY: Requirement = Requirement {
    name: "clippy",
    tier: Tier::Develop,
    need: Need::Required,
    enables: "`cargo clippy -D warnings`, which is where this workspace's lints bite",
    detect: Detection::Program {
        program: "cargo-clippy",
        version_arg: "--version",
        places: NOWHERE,
    },
    homepage: Some("https://doc.rust-lang.org/clippy/"),
    recipe: same_command("rustup", &["component", "add", "clippy"]),
};

pub(super) const CARGO_DENY: Requirement = Requirement {
    name: "cargo-deny",
    tier: Tier::Develop,
    need: Need::Optional,
    enables: "`cargo deny check`, which `just gates` runs over deny.toml when it is here",
    detect: Detection::Program {
        program: "cargo-deny",
        version_arg: "--version",
        places: NOWHERE,
    },
    homepage: Some("https://embarkstudios.github.io/cargo-deny/"),
    recipe: same_command("cargo", &["install", "cargo-deny", "--locked"]),
};

pub(super) const CARGO_AUDIT: Requirement = Requirement {
    name: "cargo-audit",
    tier: Tier::Develop,
    need: Need::Optional,
    enables: "`cargo audit`, which reads this lockfile against the RustSec advisories",
    detect: Detection::Program {
        program: "cargo-audit",
        version_arg: "--version",
        places: NOWHERE,
    },
    homepage: Some("https://rustsec.org/"),
    recipe: same_command("cargo", &["install", "cargo-audit", "--locked"]),
};

pub(super) const CARGO_MUTANTS: Requirement = Requirement {
    name: "cargo-mutants",
    tier: Tier::Develop,
    need: Need::Optional,
    enables: "`just mutants`, which asks whether the tests notice a changed kernel",
    detect: Detection::Program {
        program: "cargo-mutants",
        version_arg: "--version",
        places: NOWHERE,
    },
    homepage: Some("https://mutants.rs/"),
    recipe: same_command("cargo", &["install", "cargo-mutants", "--locked"]),
};

pub(super) const CARGO_FUZZ: Requirement = Requirement {
    name: "cargo-fuzz",
    tier: Tier::Develop,
    need: Need::Optional,
    enables: "`just fuzz <target>`, which needs a nightly toolchain as well",
    detect: Detection::Program {
        program: "cargo-fuzz",
        version_arg: "--version",
        places: NOWHERE,
    },
    homepage: Some("https://rust-fuzz.github.io/book/cargo-fuzz.html"),
    recipe: same_command("cargo", &["install", "cargo-fuzz", "--locked"]),
};

pub(super) const CARGO_LLVM_COV: Requirement = Requirement {
    name: "cargo-llvm-cov",
    tier: Tier::Develop,
    need: Need::Optional,
    enables: "a coverage reading of this workspace, which no gate asks for",
    detect: Detection::Program {
        program: "cargo-llvm-cov",
        version_arg: "--version",
        places: NOWHERE,
    },
    homepage: Some("https://github.com/taiki-e/cargo-llvm-cov"),
    recipe: same_command("cargo", &["install", "cargo-llvm-cov", "--locked"]),
};

pub(super) const KANI: Requirement = Requirement {
    name: "kani",
    tier: Tier::Develop,
    need: Need::Optional,
    enables: "proving a property against real MIR, on Linux and nowhere else today",
    detect: Detection::Program {
        program: "cargo-kani",
        version_arg: "--version",
        places: NOWHERE,
    },
    homepage: Some("https://model-checking.github.io/kani/"),
    recipe: PerPlatform {
        windows: Recipe::Manual("kani runs on Linux; a WSL installation is where it goes here"),
        macos: Recipe::Manual("kani runs on Linux; this platform has no build today"),
        linux: Recipe::Print("cargo install --locked kani-verifier && cargo kani setup"),
    },
};
