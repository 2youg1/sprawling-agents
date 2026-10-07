// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The official harnesses this city drives, and how each is started as
//! an ACP agent (`crates/agent_protocols/Spec.lean` §8-19).
//!
//! **The roster is the person's ruling.** Five harnesses and no more: a
//! sixth is added by a ruling, not because the ACP registry grew a row.
//!
//! **How each starts is read from the ACP registry**
//! (`agentclientprotocol/registry`, one `agent.json` per agent, watched
//! in `docs/third-party.md` section 1), with every version pinned: an
//! unpinned `npx` fetches whatever npm holds on the day it runs.

/// One official harness.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Harness {
    /// OpenAI's Codex, through the registry's `codex-acp` adapter.
    Codex,
    /// Anthropic's Claude Code, through the registry's `claude-acp`
    /// adapter.
    ClaudeCode,
    /// xAI's Grok Build, which speaks ACP itself.
    GrokBuild,
    /// Moonshot's Kimi Code CLI, which speaks ACP itself.
    KimiCode,
    /// Pi, through the registry's `pi-acp` adapter.
    Pi,
}

/// The program an ACP agent is started with.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Program {
    /// Node's package runner, which fetches the pinned package.
    Npx,
    /// Kimi Code's own binary, installed by the person.
    Kimi,
}

impl Program {
    /// The name the search path is asked for. Node installs `npx` as a
    /// batch file on Windows, which a process cannot be started from
    /// under its bare name.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Program::Npx if cfg!(windows) => "npx.cmd",
            Program::Npx => "npx",
            Program::Kimi => "kimi",
        }
    }
}

/// One directory a harness writes once it is installed or signed in, as
/// its vendor documents it.
///
/// The vendors named here document one location for all three
/// platforms: a directory under the User's home (`%USERPROFILE%` on
/// Windows, `$HOME` on macOS and Linux), moved by one environment
/// variable when that is set.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SetUpDir {
    /// The variable the vendor documents as moving the directory.
    pub variable: Option<&'static str>,
    /// The directory under the home directory, one component per entry.
    pub under_home: &'static [&'static str],
    /// The vendor page that states the directory.
    pub source: &'static str,
}

impl SetUpDir {
    /// Where this directory is on a machine whose home directory and
    /// whose value of [`SetUpDir::variable`] are given.
    ///
    /// A variable set to something non-empty wins, as each vendor
    /// documents; with none, the directory is under the home, and with
    /// no home either it is nowhere this machine can name.
    #[must_use]
    pub fn on(
        &self,
        home: Option<&std::path::Path>,
        variable: Option<std::ffi::OsString>,
    ) -> Option<std::path::PathBuf> {
        match variable.filter(|value| !value.is_empty()) {
            Some(moved) => Some(std::path::PathBuf::from(moved)),
            None => home.map(|home| {
                self.under_home
                    .iter()
                    .fold(home.to_path_buf(), |path, part| path.join(part))
            }),
        }
    }
}

/// One command that starts a harness as an ACP agent on stdio.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Launch {
    pub program: Program,
    pub args: &'static [&'static str],
}

impl Harness {
    /// Every harness, in the order the page shows them.
    pub const ALL: [Harness; 5] = [
        Harness::ClaudeCode,
        Harness::Codex,
        Harness::GrokBuild,
        Harness::KimiCode,
        Harness::Pi,
    ];

    /// The word this harness travels under.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Harness::Codex => "codex",
            Harness::ClaudeCode => "claude_code",
            Harness::GrokBuild => "grok_build",
            Harness::KimiCode => "kimi_code",
            Harness::Pi => "pi",
        }
    }

    /// The command that starts this harness as an ACP agent.
    #[must_use]
    pub const fn launch(self) -> Launch {
        match self {
            Harness::ClaudeCode => Launch {
                program: Program::Npx,
                args: &["-y", "@agentclientprotocol/claude-agent-acp@0.86.0"],
            },
            Harness::Codex => Launch {
                program: Program::Npx,
                args: &["-y", "@agentclientprotocol/codex-acp@2.1.1"],
            },
            Harness::GrokBuild => Launch {
                program: Program::Npx,
                args: &["-y", "@xai-official/grok@1.0.49", "agent", "stdio"],
            },
            Harness::KimiCode => Launch {
                program: Program::Kimi,
                args: &["acp"],
            },
            Harness::Pi => Launch {
                program: Program::Npx,
                args: &["-y", "pi-acp@0.0.34"],
            },
        }
    }

    /// The directories whose presence says this harness is installed or
    /// signed in on this machine (`crates/wire/spec/Answer/Harnesses.lean`
    /// D23).
    ///
    /// Each row is read from the vendor page it cites. A harness whose
    /// vendor documents no such directory has an empty table, which the
    /// harness page reads as "not looked for" rather than "not there".
    #[must_use]
    pub const fn set_up(self) -> &'static [SetUpDir] {
        match self {
            Harness::ClaudeCode => &[SetUpDir {
                variable: Some("CLAUDE_CONFIG_DIR"),
                under_home: &[".claude"],
                source: "https://code.claude.com/docs/en/settings",
            }],
            Harness::Codex => &[SetUpDir {
                variable: Some("CODEX_HOME"),
                under_home: &[".codex"],
                source: "https://developers.openai.com/codex/auth",
            }],
            Harness::Pi => &[SetUpDir {
                variable: Some("PI_CODING_AGENT_DIR"),
                under_home: &[".pi", "agent"],
                source: "https://github.com/earendil-works/pi/blob/main/packages/coding-agent/docs/environment-variables.md",
            }],
            Harness::GrokBuild => &[SetUpDir {
                variable: Some("GROK_HOME"),
                under_home: &[".grok"],
                source: "https://docs.x.ai/build/settings/reference",
            }],
            Harness::KimiCode => &[SetUpDir {
                variable: Some("KIMI_CODE_HOME"),
                under_home: &[".kimi-code"],
                source: "https://moonshotai.github.io/kimi-code/en/configuration/data-locations",
            }],
        }
    }

    /// This harness's id in the ACP registry.
    #[must_use]
    pub const fn registry_id(self) -> &'static str {
        match self {
            Harness::ClaudeCode => "claude-acp",
            Harness::Codex => "codex-acp",
            Harness::GrokBuild => "grok-build",
            Harness::KimiCode => "kimi",
            Harness::Pi => "pi-acp",
        }
    }

    /// The harness a word names, the inverse of [`Harness::as_str`]. A
    /// word no harness travels under is `None`; the caller writes the
    /// refusal, because it knows where the word was written.
    #[must_use]
    pub fn parse(word: &str) -> Option<Harness> {
        Harness::ALL
            .into_iter()
            .find(|harness| harness.as_str() == word)
    }

    /// Where this harness's own vendor says how a person signs in. The
    /// person signs in inside the harness; the city never does.
    #[must_use]
    pub const fn docs(self) -> &'static str {
        match self {
            Harness::ClaudeCode => "https://code.claude.com/docs/en/authentication",
            Harness::Codex => "https://developers.openai.com/codex/auth",
            Harness::GrokBuild => "https://docs.x.ai/build/overview",
            Harness::KimiCode => "https://www.kimi.com/en/help/kimi-code/membership-guide",
            Harness::Pi => "https://github.com/svkozak/pi-acp",
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::indexing_slicing, reason = "test code")]
mod tests {
    use super::*;

    /// Every `npx` launch names a version: without one, npm hands over
    /// whatever it holds on the day the harness starts.
    #[test]
    fn every_package_a_harness_is_fetched_as_is_pinned_to_a_version() {
        for harness in Harness::ALL {
            let launch = harness.launch();
            if launch.program == Program::Npx {
                let package = launch
                    .args
                    .iter()
                    .find(|arg| !arg.starts_with('-'))
                    .unwrap();
                let (_, version) = package.rsplit_once('@').unwrap();
                assert!(
                    version.chars().next().unwrap().is_ascii_digit(),
                    "{} starts {package}",
                    harness.as_str()
                );
            }
        }
    }

    /// A variable the vendor documents moves the directory; an empty one
    /// is unset, and with neither a variable nor a home there is no path.
    #[test]
    fn a_set_up_directory_is_moved_by_its_variable_and_otherwise_under_home() {
        let row = Harness::Pi.set_up()[0];
        let home = std::path::Path::new("home");
        assert_eq!(
            [
                row.on(Some(home), None),
                row.on(Some(home), Some("".into())),
                row.on(Some(home), Some("moved".into())),
                row.on(None, None),
            ],
            [
                Some(home.join(".pi").join("agent")),
                Some(home.join(".pi").join("agent")),
                Some(std::path::PathBuf::from("moved")),
                None,
            ]
        );
    }

    #[test]
    fn every_word_names_its_harness_and_no_other_word_names_one() {
        for harness in Harness::ALL {
            assert_eq!(Harness::parse(harness.as_str()), Some(harness));
        }
        assert_eq!(Harness::parse("claude"), None);
    }

    #[test]
    fn every_harness_travels_under_its_own_word() {
        let mut words: Vec<&str> = Harness::ALL.iter().map(|h| h.as_str()).collect();
        words.sort_unstable();
        words.dedup();
        assert_eq!(words.len(), Harness::ALL.len());
    }
}
