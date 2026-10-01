// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Every tool a person needs to develop this code (sprawling-SPEC.md
//! section 8-58).
//!
//! Data, with no branch in it. `table::REQUIREMENTS` lists these rows in
//! install order, because a later row's recipe starts a program an
//! earlier row installs: `cargo install` needs rustup, `elan toolchain
//! install` needs elan, `uv python install` needs uv. `just prereqs`
//! reads the same rows through `prereqs.tsv`, which `table::prereqs`
//! renders.
//!
//! `Required` means `just check` cannot run without it; `Optional` means
//! `just check` skips it when it is absent, or only a recipe a person
//! runs on purpose calls it, and the row says which recipe that is.

use wire::DoctorUnread;

use crate::doctor::family::{CHROMIUM_RECIPE, Family};
use crate::doctor::pin::LEAN_TOOLCHAIN as LEAN_PIN;
use crate::doctor::{Detection, Need, Pack, PerPlatform, Pin, Recipe, Requirement, Tier, Upstream};

use super::NOWHERE;

/// One command, spelled once and answered on all three platforms.
/// Every tool below that installs the same way everywhere uses it, so
/// the three columns are never one line copied three times.
const fn same_command(program: &'static str, args: &'static [&'static str]) -> PerPlatform<Recipe> {
    PerPlatform {
        windows: Recipe::Command { program, args },
        macos: Recipe::Command { program, args },
        linux: Recipe::Command { program, args },
    }
}

/// A program on the search path, asked its version with `--version`.
const fn program(name: &'static str) -> Detection {
    Detection::Program {
        program: name,
        version_arg: "--version",
        places: NOWHERE,
    }
}

/// A row of this tier; everything but the facts that differ per tool.
const fn row(
    name: &'static str,
    need: Need,
    enables: &'static str,
    detect: Detection,
    homepage: &'static str,
    recipe: PerPlatform<Recipe>,
) -> Requirement {
    Requirement {
        name,
        tier: Tier::Develop,
        need,
        enables,
        detect,
        homepage: Some(homepage),
        recipe,
        pin: Pin::Unpinned,
        upstream: Upstream::Unread(DoctorUnread::NoSource),
        pack: None,
    }
}

/// The three facts a row states beyond its detection and its recipe,
/// chained onto `row` so that each row says only the ones it has.
impl Requirement {
    const fn from(mut self, upstream: Upstream) -> Requirement {
        self.upstream = upstream;
        self
    }

    const fn pinned(mut self, pin: Pin) -> Requirement {
        self.pin = pin;
        self
    }

    /// A cargo tool this repository calls, installed from its crate.
    const fn packed(mut self, krate: &'static str) -> Requirement {
        self.pack = Some(Pack::RustTools);
        self.upstream = Upstream::Crate(krate);
        self
    }
}

pub(super) const GIT: Requirement = row(
    "git",
    Need::Required,
    "version control for this repository, and restoring a discarded file from its checkpoint",
    Detection::Program {
        program: "git",
        version_arg: "--version",
        // Where winget puts it, so the row answers before this process
        // has seen the search path the installer changed.
        places: PerPlatform {
            windows: &[r"C:\Program Files\Git\cmd\git.exe"],
            macos: &[],
            linux: &[],
        },
    },
    "https://git-scm.com/",
    PerPlatform {
        windows: Recipe::Command {
            program: "winget",
            args: &["install", "--id", "Git.Git", "-e", "--scope", "user"],
        },
        macos: Recipe::Command {
            program: "brew",
            args: &["install", "git"],
        },
        linux: Recipe::Print("sudo apt install git"),
    },
)
.from(Upstream::GitHub("git-for-windows/git"));

/// The shell every recipe of this repository runs in: the `justfile`
/// opens with `set shell := ["bash", "-uc"]`. The listing asks for the
/// line only a real bash prints, so the WSL launcher a Windows search
/// path can resolve first is reported as absent, which is what `just`
/// would meet (sprawling-SPEC.md 8-130).
pub(super) const BASH: Requirement = row(
    "bash",
    Need::Required,
    "`just`, which runs every recipe of this repository in bash",
    Detection::Listed {
        program: "bash",
        args: &["--version"],
        line: "GNU bash",
    },
    "https://www.gnu.org/software/bash/",
    PerPlatform {
        windows: Recipe::Manual(concat!(
            "run `just` from Git Bash, which the git row installs: its own bash comes first ",
            r"there, while elsewhere C:\Windows\System32\bash.exe can come first and starts ",
            "WSL rather than a shell",
        )),
        macos: Recipe::Command {
            program: "brew",
            args: &["install", "bash"],
        },
        linux: Recipe::Print("sudo apt install bash"),
    },
);

pub(super) const RUSTUP: Requirement = row(
    "rustup",
    Need::Required,
    "cargo and the Rust toolchain rust-toolchain.toml pins, fetched on first use",
    program("rustup"),
    "https://rustup.rs/",
    PerPlatform {
        windows: Recipe::Command {
            program: "winget",
            args: &["install", "--id", "Rustlang.Rustup", "-e"],
        },
        macos: Recipe::Print("curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"),
        linux: Recipe::Print("curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"),
    },
)
.from(Upstream::RustupChannel);

/// The compiler itself. `rustup default stable` puts one on the search
/// path; inside this repository rustup then fetches the version
/// `rust-toolchain.toml` pins on first use.
pub(super) const RUST: Requirement = row(
    "rust",
    Need::Required,
    "the compiler, at the version rust-toolchain.toml pins, which rustup fetches on first use",
    program("rustc"),
    "https://www.rust-lang.org/",
    same_command("rustup", &["default", "stable"]),
)
.from(Upstream::RustChannel)
.pinned(Pin::RustToolchain);

pub(super) const RUSTFMT: Requirement = row(
    "rustfmt",
    Need::Required,
    "`cargo fmt --check`, the first step of `just check`",
    program("rustfmt"),
    "https://github.com/rust-lang/rustfmt",
    same_command("rustup", &["component", "add", "rustfmt"]),
)
.from(Upstream::Unread(DoctorUnread::WithToolchain));

pub(super) const CLIPPY: Requirement = row(
    "clippy",
    Need::Required,
    "`cargo clippy -D warnings`, which is where this workspace's lints bite",
    program("cargo-clippy"),
    "https://doc.rust-lang.org/clippy/",
    same_command("rustup", &["component", "add", "clippy"]),
)
.from(Upstream::Unread(DoctorUnread::WithToolchain));

pub(super) const JUST: Requirement = row(
    "just",
    Need::Required,
    "`just check`, the closing condition of every change here",
    program("just"),
    "https://just.systems/",
    PerPlatform {
        windows: Recipe::Command {
            program: "winget",
            args: &["install", "--id", "Casey.Just", "-e", "--scope", "user"],
        },
        macos: Recipe::Command {
            program: "brew",
            args: &["install", "just"],
        },
        linux: Recipe::Command {
            program: "cargo",
            args: &["install", "just", "--locked"],
        },
    },
)
.from(Upstream::Crate("just"));

pub(super) const CARGO_NEXTEST: Requirement = row(
    "cargo-nextest",
    Need::Required,
    "`just test`, the test runner every check here calls",
    program("cargo-nextest"),
    "https://nexte.st/",
    same_command("cargo", &["install", "cargo-nextest", "--locked"]),
)
.packed("cargo-nextest");

pub(super) const CARGO_DENY: Requirement = row(
    "cargo-deny",
    Need::Optional,
    "`just deny`, the supply-chain read `just gates` ends with; CI runs it on every push",
    program("cargo-deny"),
    "https://embarkstudios.github.io/cargo-deny/",
    same_command("cargo", &["install", "cargo-deny", "--locked"]),
)
.packed("cargo-deny");

pub(super) const BUN: Requirement = row(
    "bun",
    Need::Required,
    "`just build-web` and the client's checks, which the artifact gates judge",
    program("bun"),
    "https://bun.sh/",
    PerPlatform {
        windows: Recipe::Command {
            program: "winget",
            args: &["install", "--id", "Oven-sh.Bun", "-e", "--scope", "user"],
        },
        macos: Recipe::Command {
            program: "brew",
            args: &["install", "oven-sh/bun/bun"],
        },
        linux: Recipe::Print("curl -fsSL https://bun.sh/install | bash"),
    },
)
.from(Upstream::GitHub("oven-sh/bun"));

/// The browser the render gate measures the built client in. The same
/// family and recipe as the browser tool's Chromium row, under the tier
/// that needs it for another reason.
pub(super) const RENDER_BROWSER: Requirement = row(
    "render-browser",
    Need::Required,
    "`cargo xtask render`, which measures the built client in a headless Chromium browser",
    Detection::Family(Family::Chromium),
    "https://www.chromium.org/",
    CHROMIUM_RECIPE,
)
.from(Upstream::Unread(DoctorUnread::ManyBrands));

/// elan has no winget package, and its official installers are
/// scripts. On Windows this city runs `elan-init.ps1` without a prompt
/// once the person presses install, because a fresh machine has to
/// finish in one pass; the Linux line stays printed (sprawling-SPEC.md
/// section 8-58). `-DefaultToolchain none` leaves the Lean version to
/// the `lean` row, which installs the one `lean-toolchain` pins.
pub(super) const ELAN: Requirement = row(
    "elan",
    Need::Required,
    "the Lean toolchain manager `just models` and `just adversary` run through",
    program("elan"),
    "https://github.com/leanprover/elan",
    PerPlatform {
        windows: Recipe::Command {
            program: "powershell",
            args: &[
                "-NoProfile",
                "-ExecutionPolicy",
                "Bypass",
                "-Command",
                "& ([scriptblock]::Create((irm \
                 https://raw.githubusercontent.com/leanprover/elan/master/elan-init.ps1))) \
                 -NoPrompt 1 -DefaultToolchain none",
            ],
        },
        macos: Recipe::Command {
            program: "brew",
            args: &["install", "elan-init"],
        },
        linux: Recipe::Print(
            "curl https://raw.githubusercontent.com/leanprover/elan/master/elan-init.sh -sSf | sh",
        ),
    },
)
.from(Upstream::GitHub("leanprover/elan"));

pub(super) const LEAN: Requirement = row(
    "lean",
    Need::Required,
    "`just models`, which proves every crate's Lean specification, at the version lean-toolchain pins",
    Detection::Listed {
        program: "elan",
        args: &["toolchain", "list"],
        line: LEAN_PIN,
    },
    "https://lean-lang.org/",
    same_command("elan", &["toolchain", "install", LEAN_PIN]),
)
.from(Upstream::GitHub("leanprover/lean4"))
.pinned(Pin::LeanToolchain);

/// The Zig version `desktop/ffi/zig-version` pins, the one file the
/// leaf's build script and CI's install step read too (sprawling-SPEC.md
/// 8-146).
const ZIG_PIN: &str = include_str!("../../../../../desktop/ffi/zig-version").trim_ascii_end();

/// The compiler of the desktop server's Zig leaf. Required because a
/// Windows build of this binary compiles the leaf, and `Need` does not
/// vary by platform; elsewhere the leaf is not compiled and the tool is
/// simply unused (sprawling-SPEC.md 8-146).
pub(super) const ZIG: Requirement = row(
    "zig",
    Need::Required,
    "the Zig leaf desktop/ffi builds on Windows, at the version desktop/ffi/zig-version pins",
    Detection::Listed {
        program: "zig",
        args: &["version"],
        line: ZIG_PIN,
    },
    "https://ziglang.org/",
    PerPlatform {
        windows: Recipe::Command {
            program: "winget",
            args: &[
                "install",
                "--id",
                "zig.zig",
                "-e",
                "--version",
                ZIG_PIN,
                "--scope",
                "user",
            ],
        },
        macos: Recipe::Command {
            program: "brew",
            args: &["install", "zig"],
        },
        linux: Recipe::Manual(
            "install the Zig desktop/ffi/zig-version names from https://ziglang.org/download/",
        ),
    },
);

pub(super) const UV: Requirement = row(
    "uv",
    Need::Optional,
    "the Python this repository's probes and maintenance scripts run on",
    program("uv"),
    "https://docs.astral.sh/uv/",
    PerPlatform {
        windows: Recipe::Command {
            program: "winget",
            args: &["install", "--id", "astral-sh.uv", "-e", "--scope", "user"],
        },
        macos: Recipe::Command {
            program: "brew",
            args: &["install", "uv"],
        },
        linux: Recipe::Print("curl -LsSf https://astral.sh/uv/install.sh | sh"),
    },
)
.from(Upstream::GitHub("astral-sh/uv"));

/// Present when uv can name an interpreter, whichever installed it.
pub(super) const PYTHON: Requirement = row(
    "python",
    Need::Optional,
    "Python for the probes and data transforms, installed and managed by uv",
    Detection::Listed {
        program: "uv",
        args: &["python", "find"],
        line: "",
    },
    "https://www.python.org/",
    same_command("uv", &["python", "install"]),
)
.from(Upstream::PythonOrg);

pub(super) const CARGO_MUTANTS: Requirement = row(
    "cargo-mutants",
    Need::Optional,
    "`just mutants`, which asks whether the tests notice a changed kernel",
    program("cargo-mutants"),
    "https://mutants.rs/",
    same_command("cargo", &["install", "cargo-mutants", "--locked"]),
)
.packed("cargo-mutants");

pub(super) const CARGO_FUZZ: Requirement = row(
    "cargo-fuzz",
    Need::Optional,
    "`just fuzz <target>`, which needs a nightly toolchain as well",
    program("cargo-fuzz"),
    "https://rust-fuzz.github.io/book/cargo-fuzz.html",
    same_command("cargo", &["install", "cargo-fuzz", "--locked"]),
)
.packed("cargo-fuzz");

pub(super) const CARGO_PUBLIC_API: Requirement = row(
    "cargo-public-api",
    Need::Optional,
    "`cargo xtask apisync`, which the nightly job runs; it reads a nightly rustdoc as well",
    program("cargo-public-api"),
    "https://github.com/cargo-public-api/cargo-public-api",
    same_command("cargo", &["install", "cargo-public-api", "--locked"]),
)
.packed("cargo-public-api");

pub(super) const KANI: Requirement = row(
    "kani",
    Need::Optional,
    "`just proof`, proving a property against real MIR, on Linux and nowhere else today",
    program("cargo-kani"),
    "https://model-checking.github.io/kani/",
    PerPlatform {
        windows: Recipe::Manual("kani runs on Linux; a WSL installation is where it goes here"),
        macos: Recipe::Manual("kani runs on Linux; this platform has no build today"),
        linux: Recipe::Print("cargo install --locked kani-verifier && cargo kani setup"),
    },
)
.packed("kani-verifier");
