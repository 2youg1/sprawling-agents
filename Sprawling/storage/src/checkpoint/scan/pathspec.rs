// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

pub(crate) fn of(scopes: &[String]) -> Vec<String> {
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
