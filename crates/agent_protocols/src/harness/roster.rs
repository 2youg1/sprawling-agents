// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Which agent a word names: the rows the person added, and the five
//! official harnesses as built-in entries (`crates/agent_protocols/Spec.lean`
//! §8-19, D12).
//!
//! A built-in entry records what the registry does not: the word it has
//! always travelled under, the directories its vendor documents and the
//! vendor's sign-in page. How it starts is the catalog snapshot's row for
//! its registry id, so a version is pinned in one place.

use kernel::{AxError, B3Hash};

use super::catalog::Catalog;
use super::entry::{AgentEntry, AgentId, Consented};

/// One official harness: a built-in catalog entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Official {
    /// The word `[resident] harness` names it by.
    pub word: &'static str,
    /// Its id in the ACP registry, whose row says how it starts.
    pub registry_id: &'static str,
    /// The directories whose presence says it is installed or signed in
    /// (`crates/agent_protocols/Spec.lean` D19). Each row cites its
    /// vendor page; an empty table reads as "not looked for".
    pub set_up: &'static [SetUpDir],
    /// Where its vendor says how a person signs in.
    pub docs: &'static str,
}

/// The five official harnesses, in the order detection lists those it finds.
pub const OFFICIAL: [Official; 5] = [
    Official {
        word: "claude_code",
        registry_id: "claude-acp",
        set_up: &[SetUpDir {
            variable: Some("CLAUDE_CONFIG_DIR"),
            under_home: &[".claude"],
            source: "https://code.claude.com/docs/en/settings",
        }],
        docs: "https://code.claude.com/docs/en/authentication",
    },
    Official {
        word: "codex",
        registry_id: "codex-acp",
        set_up: &[SetUpDir {
            variable: Some("CODEX_HOME"),
            under_home: &[".codex"],
            source: "https://developers.openai.com/codex/auth",
        }],
        docs: "https://developers.openai.com/codex/auth",
    },
    Official {
        word: "grok_build",
        registry_id: "grok-build",
        set_up: &[SetUpDir {
            variable: Some("GROK_HOME"),
            under_home: &[".grok"],
            source: "https://docs.x.ai/build/settings/reference",
        }],
        docs: "https://docs.x.ai/build/overview",
    },
    Official {
        word: "kimi_code",
        registry_id: "kimi",
        set_up: &[SetUpDir {
            variable: Some("KIMI_CODE_HOME"),
            under_home: &[".kimi-code"],
            source: "https://moonshotai.github.io/kimi-code/en/configuration/data-locations",
        }],
        docs: "https://www.kimi.com/en/help/kimi-code/membership-guide",
    },
    Official {
        word: "pi",
        registry_id: "pi-acp",
        set_up: &[SetUpDir {
            variable: Some("PI_CODING_AGENT_DIR"),
            under_home: &[".pi", "agent"],
            source: "https://github.com/earendil-works/pi/blob/main/packages/coding-agent/docs/environment-variables.md",
        }],
        docs: "https://github.com/svkozak/pi-acp",
    },
];

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

/// Every agent a word can name in this city: the city's `[[agent]]` rows,
/// each with the digest its consent recorded, and the catalog the
/// built-in entries start from.
#[derive(Debug, Clone)]
pub struct Roster {
    rows: Vec<(AgentEntry, Option<B3Hash>)>,
    catalog: Catalog,
}

/// Why a word seats nobody.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Unseated {
    /// No row and no built-in entry travels under the word; the caller
    /// writes the refusal, because it knows where the word was written.
    Unknown,
    /// A row whose launch changed after its consent.
    Refused(AxError),
}

impl Roster {
    #[must_use]
    pub fn new(rows: Vec<(AgentEntry, Option<B3Hash>)>, catalog: Catalog) -> Roster {
        Roster { rows, catalog }
    }

    /// A built-in entry as the catalog starts it, under its own word.
    /// `None` when the snapshot has no row this platform can start.
    #[must_use]
    pub fn builtin(&self, official: &Official) -> Option<AgentEntry> {
        let row = self.catalog.find(official.registry_id)?;
        Some(AgentEntry {
            id: AgentId::official(official.word),
            ..row.clone()
        })
    }

    /// The consented agent `word` names: a row the person added first,
    /// then a built-in entry.
    ///
    /// # Errors
    /// [`Unseated::Unknown`] for a word nothing travels under, and
    /// [`Unseated::Refused`] for a row whose recorded digest is not its
    /// launch's.
    pub fn seat(&self, word: &str) -> Result<Consented, Unseated> {
        if let Some((entry, digest)) = self
            .rows
            .iter()
            .find(|(entry, _)| entry.id.as_str() == word)
        {
            return match digest {
                Some(digest) => Consented::given(entry.clone(), digest).map_err(Unseated::Refused),
                None => Ok(Consented::written_by_hand(entry.clone())),
            };
        }
        OFFICIAL
            .iter()
            .find(|official| official.word == word)
            .and_then(|official| self.builtin(official))
            .map(Consented::written_by_hand)
            .ok_or(Unseated::Unknown)
    }

    /// Every word that seats an agent, rows first, for a refusal's
    /// `nearby`.
    #[must_use]
    pub fn words(&self) -> Vec<String> {
        self.rows
            .iter()
            .map(|(entry, _)| entry.id.as_str().to_owned())
            .chain(OFFICIAL.iter().map(|official| official.word.to_owned()))
            .collect()
    }

    /// The rows the person added.
    pub fn rows(&self) -> impl Iterator<Item = &AgentEntry> {
        self.rows.iter().map(|(entry, _)| entry)
    }

    #[must_use]
    pub fn catalog(&self) -> &Catalog {
        &self.catalog
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::indexing_slicing, reason = "test code")]
mod tests {
    use super::*;

    /// A variable the vendor documents moves the directory; an empty one
    /// is unset, and with neither a variable nor a home there is no path.
    #[test]
    fn a_set_up_directory_is_moved_by_its_variable_and_otherwise_under_home() {
        let row = OFFICIAL[4].set_up[0];
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
}
