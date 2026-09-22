// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Markdown links: what a document tells a reader to open, where that
//! lands in the tree, and how the tree spells it.

use std::collections::{BTreeMap, BTreeSet};

/// Every markdown link target a file mentions, as written.
pub(super) fn link_targets(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let bytes: Vec<char> = text.chars().collect();
    let mut index = 0usize;
    while index < bytes.len() {
        if bytes.get(index) == Some(&'(')
            && index > 0
            && bytes.get(index.saturating_sub(1)) == Some(&']')
        {
            let mut end = index.saturating_add(1);
            let mut target = String::new();
            while let Some(ch) = bytes.get(end) {
                if *ch == ')' {
                    break;
                }
                target.push(*ch);
                end = end.saturating_add(1);
            }
            if !target.is_empty() {
                out.push(target);
            }
            index = end;
        }
        index = index.saturating_add(1);
    }
    out
}

/// A link target as a repo-relative path, resolved against the file that
/// carries it.
pub(super) fn resolve(from: &str, target: &str) -> String {
    if target.starts_with('/') {
        return target.trim_start_matches('/').to_owned();
    }
    let mut parts: Vec<&str> = from.split('/').collect();
    parts.pop();
    for segment in target.split('/') {
        match segment {
            "." | "" => {}
            // A `..` with nothing left to consume points outside the
            // repository. Dropping it silently resolved such a link to a
            // path that happens to exist — `ARCHITECTURE.md` plus
            // `../docs/glossary.md` became `docs/glossary.md` — so the
            // gate passed seven links that are 404 for every reader. The
            // segment is kept instead, and no published path spells one.
            ".." => {
                if parts.pop().is_none() {
                    parts.push("..");
                }
            }
            other => parts.push(other),
        }
    }
    parts.join("/")
}

/// The published tree indexed by the lower-cased spelling of every path.
///
/// A link is judged against this rather than against an `open` call,
/// because the two disagree exactly where it matters: Windows and macOS
/// hand out `docs/Glossary.md` when the file is named `docs/glossary.md`,
/// Linux and the hosting site return 404, and a gate built on `open`
/// stays green on the machine that wrote the dead link.
pub(super) struct Spellings<'a> {
    published: &'a BTreeSet<String>,
    by_lowercase: BTreeMap<String, &'a str>,
}

impl<'a> Spellings<'a> {
    pub(super) fn of(published: &'a BTreeSet<String>) -> Self {
        let mut by_lowercase = BTreeMap::new();
        for rel in published {
            by_lowercase.insert(rel.to_lowercase(), rel.as_str());
        }
        Self {
            published,
            by_lowercase,
        }
    }

    /// The name on disk, when a link reaches a published file whose
    /// spelling differs from the link in case alone.
    ///
    /// `None` covers two different situations on purpose: the link is
    /// right, or it points at something this tree does not publish as a
    /// file at all - a directory, or a path already reported by the
    /// scaffolding assertion. Only the case question is answered here.
    pub(super) fn miscased(&self, pointed: &str) -> Option<&'a str> {
        if self.published.contains(pointed) {
            return None;
        }
        let on_disk = self.by_lowercase.get(&pointed.to_lowercase())?;
        Some(on_disk)
    }
}
