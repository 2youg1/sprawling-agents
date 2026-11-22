// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

mod toolchain;

use super::family::{CHROMIUM_ROW, GECKO_ROW, WEBKIT_ROW};
use super::{Detection, Need, PerPlatform, Pin, Platform, Recipe, Requirement, Tier, Upstream};
use kernel::{AxCode, AxError};
use wire::DoctorUnread;

pub(crate) const PYTHON_VARIABLE: &str = "SPRAWLING_PYTHON";

pub(crate) const CHROMEDRIVER: &str = "chromedriver";
pub(crate) const MSEDGEDRIVER: &str = "msedgedriver";
pub(crate) const PYTHON: &str = "host-python";
pub(crate) const SHELL: &str = "shell";
pub(crate) const PWSH: &str = "pwsh";
pub(crate) const FFMPEG: &str = "ffmpeg";

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

#[cfg(test)]
pub(crate) fn prereqs() -> String {
    let header = concat!(
        "# generated from Sprawling/app/src/doctor/table.rs by `table::prereqs`; ",
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
        | Detection::Family(_)
        | Detection::Font { .. } => "-".to_owned(),
    }
}

const NOWHERE: PerPlatform<&[&str]> = PerPlatform {
    windows: &[],
    macos: &[],
    linux: &[],
};

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
        name: PYTHON,
        tier: Tier::Use,
        need: Need::Optional,
        enables: "the exec tool's python arm, which runs this machine's interpreter under the building's confinement",
        detect: Detection::Interpreter {
            variable: PerPlatform {
                windows: PYTHON_VARIABLE,
                macos: PYTHON_VARIABLE,
                linux: PYTHON_VARIABLE,
            },
            fallback: PerPlatform {
                windows: "python",
                macos: "python3",
                linux: "python3",
            },
        },
        homepage: Some("https://www.python.org/downloads/"),
        recipe: PerPlatform {
            windows: Recipe::Manual(
                "install Python 3 from https://www.python.org/downloads/ or set SPRAWLING_PYTHON to an interpreter",
            ),
            macos: Recipe::Manual(
                "install Python 3 (for example `brew install python`) or set SPRAWLING_PYTHON to an interpreter",
            ),
            linux: Recipe::Manual(
                "install Python 3 from the distribution packages or set SPRAWLING_PYTHON to an interpreter",
            ),
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
        name: PWSH,
        tier: Tier::Use,
        need: Need::Optional,
        enables: "the exec tool's shell arm under PowerShell 7, where a building's CONFIG.toml \
        sets `[sandbox] interpreter = \"pwsh\"`",
        detect: Detection::Program {
            program: "pwsh",
            version_arg: "--version",
            places: NOWHERE,
        },
        homepage: Some("https://github.com/PowerShell/PowerShell/releases/latest"),
        recipe: PerPlatform {
            windows: Recipe::Command {
                program: "winget",
                args: &[
                    "install",
                    "--id",
                    "Microsoft.PowerShell",
                    "-e",
                    "--scope",
                    "user",
                ],
            },
            macos: Recipe::Command {
                program: "brew",
                args: &["install", "powershell"],
            },
            linux: Recipe::Manual("install PowerShell 7 from your distribution's packages"),
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
        homepage: Some("https://ffmpeg.org/download.html"),
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
    Requirement {
        name: "sarasa-gothic",
        tier: Tier::Use,
        need: Need::Optional,
        enables: "Chinese glyphs that pair with Paper Mono as a fallback face; install the Mono SC variant",
        detect: Detection::Font {
            files: &["SarasaMonoSC-Regular.ttf", "Sarasa-SuperTTC.ttc"],
        },
        homepage: Some("https://github.com/be5invis/sarasa-gothic/releases/latest"),
        recipe: PerPlatform {
            windows: Recipe::Manual(
                "download SarasaMonoSC-TTF-*.7z from https://github.com/be5invis/sarasa-gothic/releases/latest, \
                 extract it, and install the .ttf files in your user fonts folder",
            ),
            macos: Recipe::Command {
                program: "brew",
                args: &["install", "--cask", "font-sarasa-gothic"],
            },
            linux: Recipe::Manual(
                "download SarasaMonoSC-TTF-*.7z from https://github.com/be5invis/sarasa-gothic/releases/latest, \
                 extract it, and install the .ttf files in ~/.local/share/fonts",
            ),
        },
        pin: Pin::Unpinned,
        upstream: Upstream::GitHub("be5invis/sarasa-gothic"),
        pack: None,
    },
    Requirement {
        name: "maple-mono-nf-cn",
        tier: Tier::Use,
        need: Need::Optional,
        enables: "Chinese glyphs and Nerd Font icons as a single font stack; choose Maple Mono for sans and mono, and enter Maple Mono NF CN in the first fallback",
        detect: Detection::Font {
            files: &["MapleMono-NF-CN-Regular.ttf"],
        },
        homepage: Some("https://font.subf.dev/en/"),
        recipe: PerPlatform {
            windows: Recipe::Command {
                program: "powershell",
                args: &[
                    "-NoProfile",
                    "-Command",
                    "scoop bucket add nerd-fonts; scoop install Maple-Mono-NF-CN",
                ],
            },
            macos: Recipe::Command {
                program: "brew",
                args: &["install", "--cask", "font-maple-mono-nf-cn"],
            },
            linux: Recipe::Manual(
                "download Maple Mono NF CN from https://font.subf.dev/en/ and install the font files in ~/.local/share/fonts",
            ),
        },
        pin: Pin::Unpinned,
        upstream: Upstream::GitHub("subframe7536/maple-font"),
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
