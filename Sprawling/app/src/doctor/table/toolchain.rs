// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use wire::DoctorUnread;

use crate::doctor::family::{CHROMIUM_RECIPE, Family};
use crate::doctor::pin::LEAN_TOOLCHAIN as LEAN_PIN;
use crate::doctor::{Detection, Need, Pack, PerPlatform, Pin, Recipe, Requirement, Tier, Upstream};

use super::NOWHERE;

const fn same_command(program: &'static str, args: &'static [&'static str]) -> PerPlatform<Recipe> {
    PerPlatform {
        windows: Recipe::Command { program, args },
        macos: Recipe::Command { program, args },
        linux: Recipe::Command { program, args },
    }
}

const fn program(name: &'static str) -> Detection {
    Detection::Program {
        program: name,
        version_arg: "--version",
        places: NOWHERE,
    }
}

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

impl Requirement {
    const fn from(mut self, upstream: Upstream) -> Requirement {
        self.upstream = upstream;
        self
    }

    const fn pinned(mut self, pin: Pin) -> Requirement {
        self.pin = pin;
        self
    }

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
        places: PerPlatform {
            windows: &[r"C:\Program Files\Git\cmd\git.exe"],
            macos: &[],
            linux: &[],
        },
    },
    "https://git-scm.com/downloads",
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

pub(super) const BASH: Requirement = row(
    "bash",
    Need::Required,
    "`just`, which runs every recipe of this repository in bash",
    Detection::Listed {
        program: "bash",
        args: &["--version"],
        line: "GNU bash",
    },
    "https://www.gnu.org/software/bash/manual/html_node/Installing-Bash.html",
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

pub(super) const RUST: Requirement = row(
    "rust",
    Need::Required,
    "the compiler, at the version rust-toolchain.toml pins, which rustup fetches on first use",
    program("rustc"),
    "https://www.rust-lang.org/tools/install",
    same_command("rustup", &["default", "stable"]),
)
.from(Upstream::RustChannel)
.pinned(Pin::RustToolchain);

pub(super) const RUSTFMT: Requirement = row(
    "rustfmt",
    Need::Required,
    "`cargo fmt --check`, the first step of `just check`",
    program("rustfmt"),
    "https://github.com/rust-lang/rustfmt/releases/latest",
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
    "https://nexte.st/book/installation.html",
    same_command("cargo", &["install", "cargo-nextest", "--locked"]),
)
.packed("cargo-nextest");

pub(super) const CARGO_DENY: Requirement = row(
    "cargo-deny",
    Need::Optional,
    "`just deny`, the supply-chain read `just gates` ends with; CI runs it on every push",
    program("cargo-deny"),
    "https://embarkstudios.github.io/cargo-deny/#quickstart",
    same_command("cargo", &["install", "cargo-deny", "--locked"]),
)
.packed("cargo-deny");

pub(super) const BUN: Requirement = row(
    "bun",
    Need::Required,
    "`just build-web` and the client's checks, which the artifact gates judge",
    program("bun"),
    "https://bun.sh/docs/installation",
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

pub(super) const RENDER_BROWSER: Requirement = row(
    "render-browser",
    Need::Required,
    "`cargo xtask render`, which measures the built client in a headless Chromium browser",
    Detection::Family(Family::Chromium),
    "https://www.chromium.org/",
    CHROMIUM_RECIPE,
)
.from(Upstream::Unread(DoctorUnread::ManyBrands));

pub(super) const ELAN: Requirement = row(
    "elan",
    Need::Required,
    "the Lean toolchain manager `just models` and `just adversary` run through",
    program("elan"),
    "https://github.com/leanprover/elan/releases/latest",
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

const fn lean_install(pin: &'static str) -> &'static str {
    if pin.is_empty() { "stable" } else { pin }
}

const LEAN_INSTALL: &str = lean_install(LEAN_PIN);

pub(super) const LEAN: Requirement = row(
    "lean",
    Need::Required,
    "`just models`, which proves every crate's Lean specification, at the version lean-toolchain pins",
    Detection::Listed {
        program: "elan",
        args: &["toolchain", "list"],
        line: LEAN_PIN,
    },
    "https://lean-lang.org/download/",
    same_command("elan", &["toolchain", "install", LEAN_INSTALL]),
)
.from(Upstream::GitHub("leanprover/lean4"))
.pinned(Pin::LeanToolchain);

const ZIG_PIN: &str = crate::doctor::pin::ZIG_VERSION;

macro_rules! winget_zig {
    ($pin:expr) => {
        if $pin.is_empty() {
            &["install", "--id", "zig.zig", "-e", "--scope", "user"]
        } else {
            &[
                "install",
                "--id",
                "zig.zig",
                "-e",
                "--version",
                $pin,
                "--scope",
                "user",
            ]
        }
    };
}

pub(super) const ZIG: Requirement = row(
    "zig",
    Need::Required,
    "the Zig leaves: Calyx/ffi on every platform and Sprawling/desktop/ffi on Windows, at the version Sprawling/desktop/ffi/zig-version pins",
    Detection::Listed {
        program: "zig",
        args: &["version"],
        line: ZIG_PIN,
    },
    "https://ziglang.org/download/",
    PerPlatform {
        windows: Recipe::Command {
            program: "winget",
            args: winget_zig!(ZIG_PIN),
        },
        macos: Recipe::Command {
            program: "brew",
            args: &["install", "zig"],
        },
        linux: Recipe::Manual(
            "install the Zig Sprawling/desktop/ffi/zig-version names from https://ziglang.org/download/",
        ),
    },
);

pub(super) const UV: Requirement = row(
    "uv",
    Need::Required,
    "the Python required by source gates, probes and maintenance scripts",
    program("uv"),
    "https://docs.astral.sh/uv/getting-started/installation/",
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

pub(super) const PYTHON: Requirement = row(
    "python",
    Need::Required,
    "Python for the comments gate, probes and data transforms, installed and managed by uv",
    Detection::Listed {
        program: "uv",
        args: &["python", "find"],
        line: "",
    },
    "https://www.python.org/downloads/",
    same_command("uv", &["python", "install"]),
)
.from(Upstream::PythonOrg);

pub(super) const CARGO_MUTANTS: Requirement = row(
    "cargo-mutants",
    Need::Optional,
    "`just mutants`, which asks whether the tests notice a change in a module held to a mutation score",
    program("cargo-mutants"),
    "https://mutants.rs/installation.html",
    same_command("cargo", &["install", "cargo-mutants", "--locked"]),
)
.packed("cargo-mutants");

pub(super) const CARGO_FUZZ: Requirement = row(
    "cargo-fuzz",
    Need::Optional,
    "`just fuzz <target>`, which needs a nightly toolchain as well",
    program("cargo-fuzz"),
    "https://rust-fuzz.github.io/book/cargo-fuzz/setup.html",
    same_command("cargo", &["install", "cargo-fuzz", "--locked"]),
)
.packed("cargo-fuzz");

pub(super) const KANI: Requirement = row(
    "kani",
    Need::Optional,
    "`just proof`, proving a property against real MIR, on Linux and nowhere else today",
    program("cargo-kani"),
    "https://model-checking.github.io/kani/install-guide.html",
    PerPlatform {
        windows: Recipe::Manual("kani runs on Linux; a WSL installation is where it goes here"),
        macos: Recipe::Manual("kani runs on Linux; this platform has no build today"),
        linux: Recipe::Print("cargo install --locked kani-verifier && cargo kani setup"),
    },
)
.packed("kani-verifier");

#[cfg(test)]
mod tests {
    use super::lean_install;

    #[test]
    fn a_build_without_pins_installs_lean_stable_and_any_zig() {
        const UNPINNED: &[&str] = winget_zig!("");
        const PINNED: &[&str] = winget_zig!("0.16.0");
        let any: &[&str] = &["install", "--id", "zig.zig", "-e", "--scope", "user"];
        let one: &[&str] = &[
            "install",
            "--id",
            "zig.zig",
            "-e",
            "--version",
            "0.16.0",
            "--scope",
            "user",
        ];
        assert_eq!(
            (
                lean_install(""),
                lean_install("leanprover/lean4:v4.33.1"),
                UNPINNED,
                PINNED
            ),
            ("stable", "leanprover/lean4:v4.33.1", any, one)
        );
    }
}
