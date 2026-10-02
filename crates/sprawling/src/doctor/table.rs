// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The two tiers, item by item (`crates/sprawling/spec/Accounting/Worker.lean` §8-40).
//!
//! Data, with no branch in it: editing this table is editing what
//! `sprawling doctor` checks and what `--install` offers. Every command
//! here installs for one user and none of them asks for elevation; the
//! two that would need a script piped into a shell are `Print`, so a
//! person reads them and decides.
//!
//! The rows a person meets first are the browser engines, which are
//! families rather than brands (`doctor::family`), and the Rust tools
//! this repository's own recipes call (`table::toolchain`). Both live
//! beside this file because the row count, not the row shape, is what
//! grows.

mod toolchain;

use super::family::{CHROMIUM_ROW, GECKO_ROW, WEBKIT_ROW};
use super::{Detection, Need, PerPlatform, Pin, Platform, Recipe, Requirement, Tier, Upstream};
use kernel::{AxCode, AxError};
use wire::DoctorUnread;

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
/// here that is not a row below is caught by the tests. The three
/// browser families are named in `doctor::family`, beside their
/// members.
pub(crate) const CHROMEDRIVER: &str = "chromedriver";
pub(crate) const MSEDGEDRIVER: &str = "msedgedriver";
pub(crate) const PYTHON_WASI: &str = "python-wasi";
pub(crate) const SHELL: &str = "shell";
pub(crate) const SANDBOX_ENGINE: &str = "sandbox-engine";
pub(crate) const FFMPEG: &str = "ffmpeg";

/// How this build installs `item` on this platform: the table's own
/// query, handed to the worker (`crates/sprawling/Spec.lean`, `doctor_install`).
///
/// # Errors
/// `InvalidArgs` for a name the table does not carry - the page asked
/// about an item this build does not know, so re-reading what the city
/// answered is the way out - and `ToolUnavailable` on a platform this
/// project has no recipe for.
pub(crate) fn recipe_for(item: &str) -> Result<&'static Recipe, AxError> {
    let requirement = REQUIREMENTS
        .iter()
        .find(|requirement| requirement.name == item)
        .ok_or_else(|| {
            AxError::failure(
                AxCode::InvalidArgs,
                "install a tool",
                format!("{item}: this city checks for no such item"),
            )
            .with_recovery("ask this machine again and install one of the items it answered with")
        })?;
    let Some(platform) = Platform::current() else {
        return Err(AxError::failure(
            AxCode::ToolUnavailable,
            "install a tool",
            format!("{item}: this platform has no recipe in this project"),
        )
        .with_recovery("install it the way this operating system installs software"));
    };
    Ok(requirement.recipe.at(platform))
}

/// The develop tier as `just prereqs` reads it, which is the whole of
/// `prereqs.tsv` beside this file (`crates/sprawling/spec/Doctor.lean` §8-58).
///
/// One line per row, in install order:
/// `class<TAB>name<TAB>probe<TAB>windows<TAB>macos<TAB>linux<TAB>purpose`,
/// where `probe` is the shell line that succeeds when the row is here,
/// or `-` for a row only this binary can look for. Called by the test
/// that holds the file to it, which prints the file it wants.
#[cfg(test)]
pub(crate) fn prereqs() -> String {
    let header = concat!(
        "# generated from crates/sprawling/src/doctor/table.rs by `table::prereqs`; ",
        "edit the table, not this file\n"
    );
    REQUIREMENTS
        .iter()
        .filter(|requirement| requirement.tier == Tier::Develop)
        .fold(header.to_owned(), |mut file, requirement| {
            let class = match requirement.need {
                Need::Required | Need::OneOf(_) => "required",
                Need::Optional => "optional",
            };
            let recipe = |platform| requirement.recipe.at(platform).spelled();
            file.push_str(&format!(
                "{class}\t{}\t{}\t{}\t{}\t{}\t{}\n",
                requirement.name,
                shell_probe(&requirement.detect),
                recipe(Platform::Windows),
                recipe(Platform::MacOs),
                recipe(Platform::Linux),
                requirement.enables
            ));
            file
        })
}

/// The shell line that asks what `detect` asks, or `-` where only this
/// binary can ask it.
#[cfg(test)]
fn shell_probe(detect: &Detection) -> String {
    match detect {
        Detection::Program { program, .. } => format!("command -v {program}"),
        Detection::Listed {
            program,
            args,
            line,
        } => match *line {
            "" => format!("{program} {} | grep -q .", args.join(" ")),
            pin => format!("{program} {} | grep -q '^{pin}'", args.join(" ")),
        },
        Detection::Component { .. }
        | Detection::Interpreter { .. }
        | Detection::Built { .. }
        | Detection::Family(_) => "-".to_owned(),
    }
}

/// A program nobody installs outside the search path.
const NOWHERE: PerPlatform<&[&str]> = PerPlatform {
    windows: &[],
    macos: &[],
    linux: &[],
};

/// Everything this city asks of the machine it runs on.
pub(crate) const REQUIREMENTS: &[Requirement] = &[
    GECKO_ROW,
    CHROMIUM_ROW,
    WEBKIT_ROW,
    Requirement {
        name: CHROMEDRIVER,
        tier: Tier::Use,
        need: Need::OneOf(super::Group::BrowserEngine),
        enables: "the browser tool against Chrome, Brave, Chromium or Vivaldi",
        detect: Detection::Program {
            program: "chromedriver",
            version_arg: "--version",
            places: NOWHERE,
        },
        homepage: Some("https://googlechromelabs.github.io/chrome-for-testing/"),
        recipe: PerPlatform {
            windows: Recipe::Manual(
                "take the build matching your Chromium's major version from \
                 https://googlechromelabs.github.io/chrome-for-testing/",
            ),
            macos: Recipe::Command {
                program: "brew",
                args: &["install", "chromedriver"],
            },
            linux: Recipe::Print("sudo apt install chromium-chromedriver"),
        },
        pin: Pin::Unpinned,
        upstream: Upstream::Unread(DoctorUnread::MatchesBrowser),
        pack: None,
    },
    Requirement {
        name: MSEDGEDRIVER,
        tier: Tier::Use,
        need: Need::OneOf(super::Group::BrowserEngine),
        enables: "the browser tool against Edge, which every Windows machine already has",
        detect: Detection::Program {
            program: "msedgedriver",
            version_arg: "--version",
            places: NOWHERE,
        },
        homepage: Some("https://developer.microsoft.com/microsoft-edge/tools/webdriver/"),
        recipe: PerPlatform {
            windows: Recipe::Manual(
                "take the build matching your Edge's major version from \
                 https://developer.microsoft.com/microsoft-edge/tools/webdriver/",
            ),
            macos: Recipe::Command {
                program: "brew",
                args: &["install", "--cask", "microsoft-edge-driver"],
            },
            linux: Recipe::Manual(
                "take the build matching your Edge's major version from \
                 https://developer.microsoft.com/microsoft-edge/tools/webdriver/",
            ),
        },
        pin: Pin::Unpinned,
        upstream: Upstream::Unread(DoctorUnread::MatchesBrowser),
        pack: None,
    },
    Requirement {
        name: SANDBOX_ENGINE,
        tier: Tier::Use,
        need: Need::Optional,
        enables: "the exec tool's program and python arms, fuel-metered and with no socket",
        detect: Detection::Built {
            carried: super::host::ENGINE_CARRIED,
        },
        homepage: Some("https://wasmtime.dev/"),
        recipe: PerPlatform {
            windows: ENGINE_BY_BUILD,
            macos: ENGINE_BY_BUILD,
            linux: ENGINE_BY_BUILD,
        },
        pin: Pin::Unpinned,
        upstream: Upstream::Unread(DoctorUnread::ThisProject),
        pack: None,
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
        homepage: Some("https://github.com/python/cpython/blob/main/Tools/wasm/README.md"),
        recipe: PerPlatform {
            windows: PYTHON_WASI_BY_HAND,
            macos: PYTHON_WASI_BY_HAND,
            linux: PYTHON_WASI_BY_HAND,
        },
        pin: Pin::Unpinned,
        upstream: Upstream::Unread(DoctorUnread::NoSource),
        pack: None,
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
        homepage: None,
        recipe: PerPlatform {
            windows: Recipe::Manual("set COMSPEC to a command interpreter"),
            macos: Recipe::Manual("set SHELL to a shell, or restore /bin/sh"),
            linux: Recipe::Manual("set SHELL to a shell, or restore /bin/sh"),
        },
        pin: Pin::Unpinned,
        upstream: Upstream::Unread(DoctorUnread::NoSource),
        pack: None,
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
        homepage: Some("https://ffmpeg.org/"),
        recipe: PerPlatform {
            windows: Recipe::Command {
                program: "winget",
                args: &["install", "--id", "Gyan.FFmpeg", "-e", "--scope", "user"],
            },
            macos: Recipe::Command {
                program: "brew",
                args: &["install", "ffmpeg"],
            },
            linux: Recipe::Print("sudo apt install ffmpeg"),
        },
        pin: Pin::Unpinned,
        upstream: Upstream::Unread(DoctorUnread::NoSource),
        pack: None,
    },
    toolchain::GIT,
    toolchain::BASH,
    toolchain::RUSTUP,
    toolchain::RUST,
    toolchain::RUSTFMT,
    toolchain::CLIPPY,
    toolchain::JUST,
    toolchain::CARGO_NEXTEST,
    toolchain::BUN,
    toolchain::RENDER_BROWSER,
    toolchain::CARGO_DENY,
    toolchain::ELAN,
    toolchain::LEAN,
    toolchain::ZIG,
    toolchain::UV,
    toolchain::PYTHON,
    toolchain::CARGO_MUTANTS,
    toolchain::CARGO_FUZZ,
    toolchain::KANI,
];

/// No package manager ships the component and python.org publishes no
/// wasi binary, so the honest answer names the place this city looks
/// rather than a download nothing here can verify.
const PYTHON_WASI_BY_HAND: Recipe =
    Recipe::Manual("put a CPython wasi build at ~/.sprawling/components/python-wasi/python.wasm");

/// The engine is a feature of this binary, not a package on this
/// machine.
const ENGINE_BY_BUILD: Recipe =
    Recipe::Manual("install a build of sprawling with the `sandbox` feature");
