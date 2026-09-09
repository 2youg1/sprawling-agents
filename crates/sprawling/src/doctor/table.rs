// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The two tiers, item by item (sprawling-SPEC.md section 8-40).
//!
//! Data, with no branch in it: editing this table is editing what
//! `sprawling doctor` checks and what `--install` offers. Every command
//! here installs for one user and none of them asks for elevation; the
//! two that would need a script piped into a shell are `Print`, so a
//! person reads them and decides.

use super::{Detection, Need, PerPlatform, Recipe, Requirement, Tier};

/// The `wasm-bindgen` CLI version the workspace manifest pins. The CLI
/// version must equal the crate version or the client build breaks
/// quietly, so this is the one place the installer spells it.
pub(crate) const WASM_BINDGEN_VERSION: &str = "0.2.127";

/// The environment variable a person may point at a CPython-WASI
/// component with. Spelled here and nowhere else: the exec tool asks
/// `doctor::host`, which reads this table. It is the compatibility
/// path for a machine that set it before the component directory
/// existed, and it wins over the directory when set.
const PYTHON_WASM_VARIABLE: &str = "SPRAWLING_PYTHON_WASM";

/// The file a CPython-WASI component is kept as, under
/// `~/.sprawling/components/python-wasi/`.
const PYTHON_WASM_FILE: &str = "python.wasm";

/// The names other modules of this binary ask `doctor::host` by. A name
/// here that is not a row below is caught by the tests.
pub(crate) const FIREFOX: &str = "firefox";
pub(crate) const CHROMEDRIVER: &str = "chromedriver";
pub(crate) const PYTHON_WASI: &str = "python-wasi";
pub(crate) const SHELL: &str = "shell";
pub(crate) const SANDBOX_ENGINE: &str = "sandbox-engine";
pub(crate) const SPRAWLING_DESKTOP: &str = "sprawling-desktop";
pub(crate) const FFMPEG: &str = "ffmpeg";

/// A program nobody installs outside the search path.
const NOWHERE: PerPlatform<&[&str]> = PerPlatform {
    windows: &[],
    macos: &[],
    linux: &[],
};

/// Where each platform puts Firefox when it is not on the search path -
/// which on Windows and macOS is the usual case.
const FIREFOX_PLACES: PerPlatform<&[&str]> = PerPlatform {
    windows: &[
        r"C:\Program Files\Mozilla Firefox\firefox.exe",
        r"C:\Program Files (x86)\Mozilla Firefox\firefox.exe",
    ],
    macos: &["/Applications/Firefox.app/Contents/MacOS/firefox"],
    linux: &["/usr/bin/firefox", "/snap/bin/firefox"],
};

/// Everything this city asks of the machine it runs on.
pub(crate) const REQUIREMENTS: &[Requirement] = &[
    Requirement {
        name: FIREFOX,
        tier: Tier::Use,
        need: Need::Required,
        enables: "the WebUI, and the browser tool a resident drives",
        detect: Detection::Program {
            program: "firefox",
            version_arg: "--version",
            places: FIREFOX_PLACES,
        },
        recipe: PerPlatform {
            windows: Recipe::Command {
                program: "winget",
                args: &["install", "--id", "Mozilla.Firefox", "-e"],
            },
            macos: Recipe::Command {
                program: "brew",
                args: &["install", "--cask", "firefox"],
            },
            linux: Recipe::Print("sudo apt install firefox"),
        },
    },
    Requirement {
        name: "rustup",
        tier: Tier::Develop,
        need: Need::Required,
        enables: "the toolchain rust-toolchain.toml pins, installed on demand",
        detect: Detection::Program {
            program: "rustup",
            version_arg: "--version",
            places: NOWHERE,
        },
        recipe: PerPlatform {
            windows: Recipe::Command {
                program: "winget",
                args: &["install", "--id", "Rustlang.Rustup", "-e"],
            },
            macos: Recipe::Print("curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"),
            linux: Recipe::Print("curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"),
        },
    },
    Requirement {
        name: "just",
        tier: Tier::Develop,
        need: Need::Required,
        enables: "`just check`, the closing condition of every change here",
        detect: Detection::Program {
            program: "just",
            version_arg: "--version",
            places: NOWHERE,
        },
        recipe: PerPlatform {
            windows: CARGO_INSTALL_JUST,
            macos: CARGO_INSTALL_JUST,
            linux: CARGO_INSTALL_JUST,
        },
    },
    Requirement {
        name: "cargo-nextest",
        tier: Tier::Develop,
        need: Need::Required,
        enables: "the test runner every gate in this repository calls",
        detect: Detection::Program {
            program: "cargo-nextest",
            version_arg: "--version",
            places: NOWHERE,
        },
        recipe: PerPlatform {
            windows: CARGO_INSTALL_NEXTEST,
            macos: CARGO_INSTALL_NEXTEST,
            linux: CARGO_INSTALL_NEXTEST,
        },
    },
    Requirement {
        name: "wasm-bindgen",
        tier: Tier::Develop,
        need: Need::Required,
        enables: "the client bundle; a CLI older than the crate breaks it quietly",
        detect: Detection::Program {
            program: "wasm-bindgen",
            version_arg: "--version",
            places: NOWHERE,
        },
        recipe: PerPlatform {
            windows: CARGO_INSTALL_BINDGEN,
            macos: CARGO_INSTALL_BINDGEN,
            linux: CARGO_INSTALL_BINDGEN,
        },
    },
    Requirement {
        name: "bun",
        tier: Tier::Develop,
        need: Need::Required,
        enables: "the JavaScript and TypeScript work beside this workspace",
        detect: Detection::Program {
            program: "bun",
            version_arg: "--version",
            places: NOWHERE,
        },
        recipe: PerPlatform {
            windows: Recipe::Command {
                program: "winget",
                args: &["install", "--id", "Oven-sh.Bun", "-e"],
            },
            macos: Recipe::Command {
                program: "brew",
                args: &["install", "oven-sh/bun/bun"],
            },
            // Printed, never run: a script piped into a shell is code
            // nobody read, and this city does not read it for anybody.
            linux: Recipe::Print("curl -fsSL https://bun.sh/install | bash"),
        },
    },
    Requirement {
        name: CHROMEDRIVER,
        tier: Tier::Develop,
        need: Need::Optional,
        enables: "the browser tool against Chromium; Firefox needs no driver",
        detect: Detection::Program {
            program: "chromedriver",
            version_arg: "--version",
            places: NOWHERE,
        },
        recipe: PerPlatform {
            windows: Recipe::Manual(
                "take the build matching your Chromium from \
                 https://googlechromelabs.github.io/chrome-for-testing/",
            ),
            macos: Recipe::Command {
                program: "brew",
                args: &["install", "chromedriver"],
            },
            linux: Recipe::Print("sudo apt install chromium-chromedriver"),
        },
    },
    Requirement {
        name: "git",
        tier: Tier::Develop,
        need: Need::Required,
        enables: "restoration: a discarded file points at a checkpoint commit",
        detect: Detection::Program {
            program: "git",
            version_arg: "--version",
            places: NOWHERE,
        },
        recipe: PerPlatform {
            windows: Recipe::Command {
                program: "winget",
                args: &["install", "--id", "Git.Git", "-e"],
            },
            macos: Recipe::Command {
                program: "brew",
                args: &["install", "git"],
            },
            linux: Recipe::Print("sudo apt install git"),
        },
    },
    Requirement {
        name: SANDBOX_ENGINE,
        tier: Tier::Use,
        need: Need::Optional,
        enables: "the exec tool's program and python arms, fuel-metered and with no socket",
        detect: Detection::Built {
            carried: super::host::ENGINE_CARRIED,
        },
        recipe: PerPlatform {
            windows: ENGINE_BY_BUILD,
            macos: ENGINE_BY_BUILD,
            linux: ENGINE_BY_BUILD,
        },
    },
    Requirement {
        name: PYTHON_WASI,
        tier: Tier::Use,
        need: Need::Optional,
        enables: "the exec tool's python arm, which runs in the sandbox with no socket",
        detect: Detection::Component {
            variable: PYTHON_WASM_VARIABLE,
            file: PYTHON_WASM_FILE,
        },
        recipe: PerPlatform {
            windows: PYTHON_WASI_BY_HAND,
            macos: PYTHON_WASI_BY_HAND,
            linux: PYTHON_WASI_BY_HAND,
        },
    },
    Requirement {
        name: SHELL,
        tier: Tier::Use,
        need: Need::Optional,
        enables: "the exec tool's shell arm, where a building's CONFIG.toml asks for it",
        detect: Detection::Interpreter {
            variable: PerPlatform {
                windows: "COMSPEC",
                macos: "SHELL",
                linux: "SHELL",
            },
            fallback: PerPlatform {
                windows: "cmd.exe",
                macos: "/bin/sh",
                linux: "/bin/sh",
            },
        },
        recipe: PerPlatform {
            windows: Recipe::Manual("set COMSPEC to a command interpreter"),
            macos: Recipe::Manual("set SHELL to a shell, or restore /bin/sh"),
            linux: Recipe::Manual("set SHELL to a shell, or restore /bin/sh"),
        },
    },
    Requirement {
        name: SPRAWLING_DESKTOP,
        tier: Tier::Use,
        need: Need::Optional,
        enables: "the desktop connector, for a building whose rules say `desktop: true`",
        detect: Detection::Program {
            program: "sprawling-desktop",
            version_arg: "--version",
            places: NOWHERE,
        },
        recipe: PerPlatform {
            windows: DESKTOP_BY_HAND,
            macos: DESKTOP_BY_HAND,
            linux: DESKTOP_BY_HAND,
        },
    },
    Requirement {
        name: FFMPEG,
        tier: Tier::Use,
        need: Need::Optional,
        enables: "an mp4 from the desktop connector's recorder; without it, a frame sequence",
        detect: Detection::Program {
            program: "ffmpeg",
            version_arg: "-version",
            places: NOWHERE,
        },
        recipe: PerPlatform {
            windows: Recipe::Command {
                program: "winget",
                args: &["install", "--id", "Gyan.FFmpeg", "-e"],
            },
            macos: Recipe::Command {
                program: "brew",
                args: &["install", "ffmpeg"],
            },
            linux: Recipe::Print("sudo apt install ffmpeg"),
        },
    },
];

const CARGO_INSTALL_JUST: Recipe = Recipe::Command {
    program: "cargo",
    args: &["install", "just", "--locked"],
};

const CARGO_INSTALL_NEXTEST: Recipe = Recipe::Command {
    program: "cargo",
    args: &["install", "cargo-nextest", "--locked"],
};

const CARGO_INSTALL_BINDGEN: Recipe = Recipe::Command {
    program: "cargo",
    args: &[
        "install",
        "wasm-bindgen-cli",
        "--version",
        WASM_BINDGEN_VERSION,
        "--locked",
    ],
};

/// No package manager ships the component and python.org publishes no
/// wasi binary, so the honest answer names the place this city looks
/// rather than a download nothing here can verify.
const PYTHON_WASI_BY_HAND: Recipe =
    Recipe::Manual("put a CPython wasi build at ~/.sprawling/components/python-wasi/python.wasm");

/// The engine is a feature of this binary, not a package on this
/// machine.
const ENGINE_BY_BUILD: Recipe =
    Recipe::Manual("install a build of sprawling with the `sandbox` feature");

/// The connector is built from this repository's `desktop/` package.
const DESKTOP_BY_HAND: Recipe = Recipe::Manual(
    "build `desktop/` from this repository and put sprawling-desktop on the search path",
);
