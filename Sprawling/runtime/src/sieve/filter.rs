// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The filter table: what `FILTERS.toml` may say, the three reducers
//! compiled in, and which entry a command key hits.
//!
//! Predicates are prefix, contains and suffix only. A user table and
//! a built-in entry are the same type, so a building's table can
//! replace `cargo`'s verdict without touching code. The ladder is
//! whole-value: the lowest layer that speaks replaces the table above
//! it entirely, so an under-specified layer can only narrow.

use kernel::{AxCode, AxError};
use serde::Deserialize;

use crate::sieve::CommandKey;

/// One entry. Everything but `id` and `command` has a default.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Filter {
    pub id: String,
    pub command: String,
    #[serde(default)]
    pub subcommands: Vec<String>,
    #[serde(default = "yes")]
    pub strip_ansi: bool,
    #[serde(default)]
    pub drop_prefix: Vec<String>,
    #[serde(default)]
    pub drop_contains: Vec<String>,
    #[serde(default)]
    pub drop_suffix: Vec<String>,
    #[serde(default)]
    pub keep_prefix: Vec<String>,
    #[serde(default)]
    pub keep_contains: Vec<String>,
    #[serde(default)]
    pub keep_suffix: Vec<String>,
    /// A kept line pulls the lines after it, up to the next blank one,
    /// into the same candidate. What a rustc diagnostic needs.
    #[serde(default)]
    pub group_until_blank: bool,
    pub head: Option<u64>,
    pub tail: Option<u64>,
    /// Said when nothing survives. `{sub}` and `{code}` are filled in.
    pub on_empty: Option<String>,
}

fn yes() -> bool {
    true
}

const GENERIC_ON_EMPTY: &str = "(no output kept), exit {code}";

impl Filter {
    fn empty(id: &str, command: &str) -> Filter {
        Filter {
            id: id.to_owned(),
            command: command.to_owned(),
            subcommands: Vec::new(),
            strip_ansi: true,
            drop_prefix: Vec::new(),
            drop_contains: Vec::new(),
            drop_suffix: Vec::new(),
            keep_prefix: Vec::new(),
            keep_contains: Vec::new(),
            keep_suffix: Vec::new(),
            group_until_blank: false,
            head: None,
            tail: None,
            on_empty: None,
        }
    }

    fn generic() -> Filter {
        Filter::empty("generic", "")
    }

    fn cargo() -> Filter {
        let mut filter = Filter::empty("cargo", "cargo");
        filter.subcommands = own(&["build", "check", "test", "clippy", "nextest", "run", "doc"]);
        filter.drop_prefix = own(&[
            "   Compiling ",
            "    Checking ",
            "    Finished ",
            "   Updating ",
            "    Blocking ",
            "  Downloaded ",
            " Downloading ",
            "     Running ",
            "   Doc-tests ",
        ]);
        filter.keep_contains = own(&["error", "warning:", "panicked at", "test result:", "-->"]);
        filter.group_until_blank = true;
        filter.head = Some(20);
        filter.tail = Some(40);
        filter.on_empty = Some("cargo {sub}: clean, exit {code}".to_owned());
        filter
    }

    fn git() -> Filter {
        let mut filter = Filter::empty("git", "git");
        filter.drop_prefix = own(&[
            "remote: Counting objects",
            "remote: Compressing objects",
            "remote: Enumerating objects",
            "remote: Resolving deltas",
            "remote: Total ",
            "Receiving objects",
            "Resolving deltas",
            "Compressing objects",
            "Counting objects",
            "Enumerating objects",
            "Writing objects",
            "Unpacking objects",
        ]);
        filter.keep_contains = own(&[
            "error",
            "fatal",
            "CONFLICT",
            "conflict",
            "rejected",
            "hint:",
            "Your branch",
        ]);
        filter.on_empty = Some("git {sub}: no output, exit {code}".to_owned());
        filter
    }

    /// Whether this entry answers for `key`: the command, and the
    /// subcommand when the entry names any.
    fn hits(&self, key: &CommandKey) -> bool {
        self.command == key.command()
            && (self.subcommands.is_empty()
                || key
                    .subcommand()
                    .is_some_and(|sub| self.subcommands.iter().any(|s| s == sub)))
    }

    /// A line the table says to keep. Checked before `drops`, so a keep
    /// always wins over a drop.
    pub(crate) fn keeps(&self, line: &str) -> bool {
        self.keep_prefix
            .iter()
            .any(|p| line.starts_with(p.as_str()))
            || self.keep_contains.iter().any(|p| line.contains(p.as_str()))
            || self.keep_suffix.iter().any(|p| line.ends_with(p.as_str()))
    }

    pub(crate) fn drops(&self, line: &str) -> bool {
        self.drop_prefix
            .iter()
            .any(|p| line.starts_with(p.as_str()))
            || self.drop_contains.iter().any(|p| line.contains(p.as_str()))
            || self.drop_suffix.iter().any(|p| line.ends_with(p.as_str()))
    }

    /// The line said in place of nothing.
    pub(crate) fn when_empty(&self, key: &CommandKey, exit_code: Option<i64>) -> String {
        let template = self.on_empty.as_deref().unwrap_or(GENERIC_ON_EMPTY);
        let code = exit_code.map_or_else(|| "none".to_owned(), |c| c.to_string());
        template
            .replace("{sub}", key.subcommand().unwrap_or(""))
            .replace("{code}", &code)
    }
}

fn own(items: &[&str]) -> Vec<String> {
    items.iter().map(|s| (*s).to_owned()).collect()
}

#[derive(Deserialize)]
struct TableFile {
    #[serde(default)]
    filter: Vec<Filter>,
}

/// The entries in the order they answer, and the generic entry behind
/// them all.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FilterTable {
    filters: Vec<Filter>,
    generic: Filter,
}

impl FilterTable {
    /// Exactly three reducers ship: `cargo`, `git`, and `generic`.
    #[must_use]
    pub fn builtin() -> FilterTable {
        FilterTable {
            filters: vec![Filter::cargo(), Filter::git()],
            generic: Filter::generic(),
        }
    }

    /// Reads one `FILTERS.toml`. A table that does not parse is refused
    /// rather than partially honoured.
    pub fn parse(toml_text: &str) -> Result<FilterTable, AxError> {
        let file: TableFile = toml::from_str(toml_text).map_err(|err| {
            AxError::failure(AxCode::InvalidArgs, "parse filter table", err.to_string())
                .with_recovery("fix FILTERS.toml; predicates are prefix, contains, suffix")
        })?;
        Ok(FilterTable {
            filters: file.filter,
            generic: Filter::generic(),
        })
    }

    /// Whole-value override across the ladder: the building's table when
    /// it has one, else the city's, else the built-in three.
    pub fn resolve(city: Option<&str>, building: Option<&str>) -> Result<FilterTable, AxError> {
        match building.or(city) {
            Some(text) => FilterTable::parse(text),
            None => Ok(FilterTable::builtin()),
        }
    }

    /// The first entry that answers for the key, or generic.
    #[must_use]
    pub fn lookup(&self, key: &CommandKey) -> &Filter {
        self.filters
            .iter()
            .find(|filter| filter.hits(key))
            .unwrap_or(&self.generic)
    }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    reason = "test code"
)]
mod tests {
    use super::*;
    use kernel::ExecArm;

    fn key(path: &str, args: &[&str]) -> CommandKey {
        CommandKey::of(&ExecArm::Program {
            path: path.to_owned(),
            args: own(args),
        })
    }

    #[test]
    fn a_command_hits_by_file_name_and_subcommand() {
        let table = FilterTable::builtin();
        assert_eq!(table.lookup(&key("/usr/bin/cargo", &["build"])).id, "cargo");
        assert_eq!(
            table.lookup(&key("CARGO.EXE", &["--quiet", "test"])).id,
            "cargo"
        );
        assert_eq!(table.lookup(&key("cargo", &["publish"])).id, "generic");
        assert_eq!(table.lookup(&key("git", &["status"])).id, "git");
        assert_eq!(table.lookup(&key("git", &[])).id, "git");
        assert_eq!(table.lookup(&key("ls", &[])).id, "generic");
    }

    #[test]
    fn a_keep_wins_over_a_drop_and_the_empty_line_is_filled_in() {
        let cargo = Filter::cargo();
        assert!(cargo.drops("   Compiling error v1"));
        assert!(cargo.keeps("   Compiling error v1"));
        assert_eq!(
            cargo.when_empty(&key("cargo", &["build"]), Some(0)),
            "cargo build: clean, exit 0"
        );
        assert_eq!(
            Filter::generic().when_empty(&key("ls", &[]), None),
            "(no output kept), exit none"
        );
    }

    #[test]
    fn unknown_fields_and_missing_ids_are_refused() {
        assert!(
            FilterTable::parse("[[filter]]\nid = \"x\"\ncommand = \"x\"\nregex = \".*\"\n")
                .is_err()
        );
        assert!(FilterTable::parse("[[filter]]\ncommand = \"x\"\n").is_err());
        let table =
            FilterTable::parse("[[filter]]\nid = \"x\"\ncommand = \"x\"\nhead = 3\n").unwrap();
        assert_eq!(table.lookup(&key("x", &[])).head, Some(3));
        assert!(table.lookup(&key("x", &[])).strip_ansi);
    }
}
