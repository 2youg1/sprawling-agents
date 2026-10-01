// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! What an index file is and what it may hold. `lib.rs` and a file named
//! for a directory of registered modules are exempt from the map, and
//! the exemption costs no coverage because their lines are checked to be
//! declarations and nothing else.

use std::collections::BTreeSet;
use std::path::Path;

use crate::report::{Violation, XtaskError};
use crate::walk;

/// Line prefixes allowed in `lib.rs` and pure index files: single-line
/// declarations only, which is what makes exempting them free.
const INDEX_PREFIXES: [&str; 8] = [
    "//",
    "#![",
    "#[",
    "pub mod ",
    "mod ",
    "pub use ",
    "pub(crate) use ",
    "use ",
];

pub(super) fn is_index_name(rel: &str, index_dirs: &BTreeSet<String>) -> bool {
    if rel.ends_with("/lib.rs") {
        return true;
    }
    rel.strip_suffix(".rs")
        .is_some_and(|stem| index_dirs.contains(stem))
}

pub(super) fn check_index_content(
    root: &Path,
    rel: &str,
    violations: &mut Vec<Violation>,
) -> Result<(), XtaskError> {
    let text = walk::read_text(&root.join(rel))?;
    for (index, raw) in text.lines().enumerate() {
        let line = raw.trim();
        let allowed =
            line.is_empty() || INDEX_PREFIXES.iter().any(|prefix| line.starts_with(prefix));
        if !allowed {
            violations.push(Violation {
                gate: "modmap",
                location: format!("{rel}:{}", index.saturating_add(1)),
                rule: "index files hold declarations only — comments, attributes, \
                       mod, use"
                    .to_owned(),
                violation: format!("logic line in an index file: {line:?}"),
                alternative: "move the logic into a registered module".to_owned(),
            });
            return Ok(());
        }
    }
    Ok(())
}
