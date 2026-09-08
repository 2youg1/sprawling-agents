// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Markdown links: what a document tells a reader to open, and where
//! that lands in the tree.

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
            ".." => {
                parts.pop();
            }
            other => parts.push(other),
        }
    }
    parts.join("/")
}
