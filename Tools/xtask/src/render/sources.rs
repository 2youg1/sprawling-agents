// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::collections::BTreeMap;
use std::path::Path;

use browser::survey::{DRAWN_IN, ENOUGH, Sources, literals};

use crate::report::XtaskError;
use crate::walk::{self, CLIENT_SRC};

pub(crate) fn index(root: &Path) -> Result<Sources, XtaskError> {
    let mut found: BTreeMap<String, String> = BTreeMap::new();
    for path in walk::files_with_ext(&root.join(CLIENT_SRC), &DRAWN_IN)? {
        let rel = walk::rel(root, &path);
        let text = walk::read_text(&path)?;
        for (index, line) in text.lines().enumerate() {
            for literal in literals(line) {
                if literal.chars().count() < ENOUGH || !literal.contains(' ') {
                    continue;
                }
                let at = index.saturating_add(1);
                found
                    .entry(literal)
                    .or_insert_with(|| format!("{rel}:{at}"));
            }
        }
    }
    Ok(Sources::of(found.into_iter().collect()))
}
