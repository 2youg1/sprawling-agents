// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::collections::{BTreeMap, BTreeSet};

use super::theorems::has_word;

const DECLARING: [&str; 10] = [
    "fn",
    "struct",
    "enum",
    "trait",
    "type",
    "const",
    "static",
    "mod",
    "union",
    "macro_rules",
];

pub(crate) struct Modules {
    files: BTreeMap<String, Module>,
    tops: BTreeSet<String>,
}

struct Module {
    text: String,
    declared: BTreeSet<String>,
    on_disk: bool,
}

impl Modules {
    pub(crate) fn new(files: BTreeMap<String, Option<String>>) -> Self {
        let tops = files
            .keys()
            .filter_map(|module| module.split("::").next())
            .map(str::to_owned)
            .collect();
        let files = files
            .into_iter()
            .map(|(path, text)| {
                let on_disk = text.is_some();
                let text = text.unwrap_or_default();
                let declared = declared(&text);
                (
                    path,
                    Module {
                        text,
                        declared,
                        on_disk,
                    },
                )
            })
            .collect();
        Self { files, tops }
    }

    pub(crate) fn paths_in<'l>(&self, line: &'l str) -> Vec<&'l str> {
        inline_code(line)
            .map(|span| span.trim_end_matches("()").trim_end_matches('!'))
            .filter(|span| {
                rust_path(span)
                    && span
                        .split_once("::")
                        .is_some_and(|(top, _)| self.tops.contains(top))
            })
            .collect()
    }

    pub(super) fn contains_module(&self, path: &str) -> bool {
        self.files.get(path).is_some_and(|module| module.on_disk)
    }

    pub(crate) fn resolves(&self, path: &str) -> bool {
        let segments: Vec<&str> = path.split("::").collect();
        let found = (1..=segments.len()).rev().find_map(|cut| {
            let module = segments.get(..cut)?.join("::");
            self.files
                .get(&module)
                .map(|found| (found, segments.get(cut..).unwrap_or_default()))
        });
        let Some((module, rest)) = found else {
            return false;
        };
        let Some((first, later)) = rest.split_first() else {
            return true;
        };
        let holds_rest = |file: &Module| later.iter().all(|segment| has_word(&file.text, segment));
        if module.declared.contains(*first) {
            return holds_rest(module);
        }
        if !reexports(&module.text, first) {
            return false;
        }
        let top = segments.first().copied().unwrap_or_default();
        let mut declaring = self
            .crate_files(top)
            .filter(|file| file.declared.contains(*first))
            .peekable();

        match declaring.peek() {
            Some(_) => declaring.any(holds_rest),
            None => later.is_empty(),
        }
    }

    fn crate_files<'s>(&'s self, top: &'s str) -> impl Iterator<Item = &'s Module> + 's {
        self.files
            .iter()
            .filter(move |(module, _)| {
                module.as_str() == top
                    || module
                        .strip_prefix(top)
                        .is_some_and(|rest| rest.starts_with("::"))
            })
            .map(|(_, module)| module)
    }
}

pub(super) fn inline_code(line: &str) -> impl Iterator<Item = &str> {
    line.split('`').skip(1).step_by(2)
}

pub(super) fn rust_path(path: &str) -> bool {
    path.contains("::")
        && path
            .split("::")
            .all(|part| !part.is_empty() && part.chars().all(|c| c.is_alphanumeric() || c == '_'))
}

fn declared(text: &str) -> BTreeSet<String> {
    let words: Vec<&str> = text
        .split(|c: char| !(c.is_alphanumeric() || c == '_'))
        .filter(|word| !word.is_empty())
        .collect();
    words
        .windows(2)
        .filter_map(|pair| match pair {
            [keyword, name] if DECLARING.contains(keyword) => Some((*name).to_owned()),
            [..] => None,
        })
        .collect()
}

fn reexports(text: &str, name: &str) -> bool {
    text.match_indices("use ").any(|(at, _)| {
        let lead = text
            .get(..at)
            .and_then(|head| head.rsplit('\n').next())
            .unwrap_or_default()
            .trim();
        let statement = text
            .get(at..)
            .and_then(|tail| tail.split(';').next())
            .unwrap_or_default();
        lead.starts_with("pub") && (has_word(statement, name) || statement.contains('*'))
    })
}
