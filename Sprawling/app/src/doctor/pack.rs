// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Pack {

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

    const WAYS_IN: &[&str] = &["binstall", "miri"];

    fn repository() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
    }

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

    fn called_in(lines: &[String], called: &mut BTreeSet<String>) {
        let subcommand = |word: &str| {
            word.chars().next().is_some_and(|c| c.is_ascii_lowercase())
                && word
                    .chars()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
                && !BUILT_IN.contains(&word)
                && !WAYS_IN.contains(&word)
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

    #[test]
    fn the_rust_tools_pack_is_every_cargo_tool_this_repository_calls() {
        let root = repository();
        let mut files = vec![root.join("justfile")];
        files_under(&root.join(".github"), &[".yml"], &mut files);
        files_under(&root.join("Tools/xtask/src"), &[".rs"], &mut files);
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
                | Detection::Family(_)
                | Detection::Font { .. } => row.name.to_owned(),
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
