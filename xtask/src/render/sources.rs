// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The client's own sources, walked once so every finding can name the
//! line that drew it.
//!
//! The reading itself belongs to `browser::survey` - it is the same
//! reading whether it is taken by this gate or by a resident. What is
//! only true here is that there is a repository to walk: the product
//! binary ships without one, so the walk stays on this side of the
//! seam and hands the index across.

use std::collections::BTreeMap;
use std::path::Path;

use browser::survey::{DRAWN_IN, ENOUGH, Sources, literals};

use crate::report::XtaskError;
use crate::walk::{self, CLIENT_SRC};

/// Read the client once. A survey opens the same page five times, and
/// five walks of the source tree would be four too many.
///
/// # Errors
/// Refuses a source tree it cannot read, because a location that is
/// silently absent reads as a box nobody wrote.
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
