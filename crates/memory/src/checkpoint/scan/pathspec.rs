// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The git pathspecs a fence stages its scopes through.

/// Two git pathspecs per scope, the scope itself and everything
/// under it, because a scope is a prefix the run may write under or,
/// when the lane knows what a wave wrote, one file (runtime-SPEC 8-45).
///
/// Several, because a write domain is a set: a building's own
/// subtree plus whatever else its `RULES.toml` declares. Staging
/// one of them and judging against all of them is what left files a
/// run legitimately wrote outside every checkpoint.
///
/// No prefixes at all means the whole tree rather than nothing: the
/// caller that passes an empty set fences the whole city, and has no
/// resident to take a domain from.
///
/// A scope is a literal path: the address grammar admits `[`, `]`,
/// `*` and `?`, and a scope read as a glob would stage its matches
/// instead of the file the wave wrote.
pub(super) fn of(scopes: &[String]) -> Vec<String> {
    if scopes.is_empty() {
        return vec!["*".to_owned()];
    }
    scopes
        .iter()
        .flat_map(|scope| match scope.trim_end_matches('/') {
            "" | "." => vec!["*".to_owned()],
            prefix => {
                let literal = literal_glob(prefix);
                vec![format!("{literal}/*"), literal]
            }
        })
        .collect()
}

/// Spells `path` as a glob that matches only itself, each metacharacter
/// inside a one-character class. A class rather than a backslash escape,
/// because libgit2 compares a pattern with no unescaped wildcard byte for
/// byte, backslashes included.
fn literal_glob(path: &str) -> String {
    path.chars()
        .fold(String::with_capacity(path.len()), |mut glob, c| {
            match c {
                '[' | ']' | '*' | '?' | '\\' => {
                    glob.push('[');
                    if c == '\\' {
                        glob.push('\\');
                    }
                    glob.push(c);
                    glob.push(']');
                }
                other => glob.push(other),
            }
            glob
        })
}
