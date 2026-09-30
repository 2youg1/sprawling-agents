// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The rows a page draws as one (sprawling-SPEC.md section 8-58).
//!
//! Each member of a pack stays a row of the table: it is detected,
//! judged and installed on its own, and `just prereqs` reads it on its
//! own. What a pack adds is only that a page shows a person one row and
//! one install control for the lot, instead of one per cargo subcommand.

/// A set of table rows a page draws as one.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Pack {
    /// Every cargo subcommand this repository's recipes, workflows and
    /// documents call. Which ones those are is decided by where the
    /// repository calls them, and the test below holds the table to it.
    RustTools,
}

impl Pack {
    pub(crate) fn wire(self) -> wire::DoctorPack {
        match self {
            Pack::RustTools => wire::DoctorPack::RustTools,
        }
    }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::arithmetic_side_effects,
    reason = "test code"
)]
mod tests {
    use std::collections::BTreeSet;
    use std::path::{Path, PathBuf};

    use super::Pack;
    use crate::doctor::{Detection, REQUIREMENTS};

    /// The subcommands cargo carries itself, which no row installs.
    const BUILT_IN: &[&str] = &[
        "add",
        "bench",
        "build",
        "check",
        "clean",
        "clippy",
        "config",
        "doc",
        "fetch",
        "fix",
        "fmt",
        "generate-lockfile",
        "help",
        "info",
        "init",
        "install",
        "locate-project",
        "login",
        "logout",
        "metadata",
        "new",
        "owner",
        "package",
        "pkgid",
        "publish",
        "remove",
        "report",
        "run",
        "rustc",
        "rustdoc",
        "search",
        "test",
        "tree",
        "uninstall",
        "update",
        "vendor",
        "verify-project",
        "version",
        "yank",
        "xtask",
    ];

    fn repository() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
    }

    /// Every file under `dir` whose name ends in one of `endings`.
    fn files_under(dir: &Path, endings: &[&str], found: &mut Vec<PathBuf>) {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                files_under(&path, endings, found);
            } else if endings
                .iter()
                .any(|ending| path.to_string_lossy().ends_with(ending))
            {
                found.push(path);
            }
        }
    }

    /// The text of one file that can call a program: a recipe or a
    /// workflow step rather than its comments, in prose only what is
    /// set as code, and in Rust only a string that starts with `cargo `,
    /// which is how a command line is spelled there.
    fn commands_of(path: &Path, text: &str) -> Vec<String> {
        let name = path.to_string_lossy();
        if name.ends_with(".md") {
            return text
                .lines()
                .flat_map(|line| line.split('`').skip(1).step_by(2))
                .map(str::to_owned)
                .collect();
        }
        if name.ends_with(".rs") {
            return text
                .split("\"cargo ")
                .skip(1)
                .map(|rest| format!("cargo {rest}"))
                .collect();
        }
        text.lines()
            .filter(|line| !line.trim_start().starts_with('#'))
            .map(str::to_owned)
            .collect()
    }

    /// The cargo tools some command text calls: each `cargo <sub>`
    /// (past a `+toolchain`) whose `<sub>` cargo does not carry, and
    /// each `cargo-*` name on an install action's `tool:` line, spelled
    /// as the program each one installs.
    fn called_in(lines: &[String], called: &mut BTreeSet<String>) {
        let subcommand = |word: &str| {
            word.chars().next().is_some_and(|c| c.is_ascii_lowercase())
                && word
                    .chars()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
                && !BUILT_IN.contains(&word)
        };
        for line in lines {
            let words: Vec<&str> = line.split_whitespace().collect();
            for (at, _) in words
                .iter()
                .enumerate()
                .filter(|(_, word)| **word == "cargo")
            {
                if let Some(sub) = words
                    .iter()
                    .skip(at + 1)
                    .find(|word| !word.starts_with('+'))
                    && subcommand(sub)
                {
                    called.insert(format!("cargo-{sub}"));
                }
            }
            if let Some(tools) = line.trim().strip_prefix("tool:") {
                called.extend(
                    tools
                        .split(',')
                        .map(str::trim)
                        .filter(|tool| tool.starts_with("cargo-"))
                        .map(str::to_owned),
                );
            }
        }
    }

    /// The pack is exactly the cargo tools this repository calls, so a
    /// tool a recipe starts calling joins it and a tool nothing calls
    /// any more leaves it; the failure names the difference both ways.
    #[test]
    fn the_rust_tools_pack_is_every_cargo_tool_this_repository_calls() {
        let root = repository();
        let mut files = vec![root.join("justfile")];
        files_under(&root.join(".github"), &[".yml"], &mut files);
        files_under(&root.join("xtask/src"), &[".rs"], &mut files);
        files_under(&root.join("docs"), &[".md"], &mut files);
        let mut called = BTreeSet::new();
        for file in &files {
            let text = std::fs::read_to_string(file).unwrap();
            called_in(&commands_of(file, &text), &mut called);
        }
        let packed: BTreeSet<String> = REQUIREMENTS
            .iter()
            .filter(|row| row.pack == Some(Pack::RustTools))
            .map(|row| match &row.detect {
                Detection::Program { program, .. } => (*program).to_owned(),
                Detection::Listed { .. }
                | Detection::Component { .. }
                | Detection::Interpreter { .. }
                | Detection::Built { .. }
                | Detection::Family(_) => row.name.to_owned(),
            })
            .collect();
        assert_eq!(
            (
                called.difference(&packed).collect::<Vec<_>>(),
                packed.difference(&called).collect::<Vec<_>>()
            ),
            (Vec::new(), Vec::new()),
            "(called by the repository and not in the pack, in the pack and called by nothing)"
        );
    }
}
