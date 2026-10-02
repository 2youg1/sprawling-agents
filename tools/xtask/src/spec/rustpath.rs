// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Backticked Rust paths and whether each names an item in the code
//! (tools/xtask/Spec.lean §8-42, assertion 7, and D24): the module map
//! says which file a module is, and a lexical scan of that file says
//! whether the next segment is declared or re-exported there.

use std::collections::{BTreeMap, BTreeSet};

use super::theorems::has_word;

/// The keywords a declared item's name follows.
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

/// The registered modules by path, each with the text of its file (empty
/// when the file is not on disk yet) and the names it declares.
pub(crate) struct Modules {
    files: BTreeMap<String, Module>,
    tops: BTreeSet<String>,
}

struct Module {
    text: String,
    declared: BTreeSet<String>,
}

impl Modules {
    pub(crate) fn new(files: BTreeMap<String, String>) -> Self {
        let tops = files
            .keys()
            .filter_map(|module| module.split("::").next())
            .map(str::to_owned)
            .collect();
        let files = files
            .into_iter()
            .map(|(path, text)| {
                let declared = declared(&text);
                (path, Module { text, declared })
            })
            .collect();
        Self { files, tops }
    }

    /// The backticked Rust paths in `line`: a code span that is exactly
    /// `top::segment(::segment)*` with `top` a module map crate.
    pub(crate) fn paths_in<'l>(&self, line: &'l str) -> Vec<&'l str> {
        line.split('`')
            .skip(1)
            .step_by(2)
            .map(|span| span.trim_end_matches("()").trim_end_matches('!'))
            .filter(|span| {
                let segments: Vec<&str> = span.split("::").collect();
                segments.len() >= 2
                    && segments.iter().all(|s| {
                        !s.is_empty() && s.chars().all(|c| c.is_alphanumeric() || c == '_')
                    })
                    && segments.first().is_some_and(|top| self.tops.contains(*top))
            })
            .collect()
    }

    /// Whether `path` names a module, or an item declared or re-exported
    /// in the longest registered module it starts with. The segments after
    /// the item (a variant, a method, a field) are words of the file that
    /// declares the item: the module itself, or for a re-export, any module
    /// of the same crate that declares it. A re-export from outside the
    /// crate resolves by its name alone.
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
        // A re-export from another crate has no declaring file in this one,
        // and only its name can be judged.
        match declaring.peek() {
            Some(_) => declaring.any(holds_rest),
            None => later.is_empty(),
        }
    }

    /// Every registered module of crate `top`, its root too.
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

/// The names `text` declares an item under, at any visibility.
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

/// Whether some `pub` or `pub(…)` `use` statement in `text` names `name`,
/// or re-exports a module's every item with `*`; a private `use` brings a
/// name into scope without making a path to it.
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
