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
                args: &["-y", "@agentclientprotocol/claude-agent-acp@0.85.1"],
            },
            Harness::Codex => Launch {
                program: Program::Npx,
                args: &["-y", "@agentclientprotocol/codex-acp@2.1.1"],
            },
            Harness::GrokBuild => Launch {
                program: Program::Npx,
                args: &["-y", "@xai-official/grok@1.0.48", "agent", "stdio"],
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
