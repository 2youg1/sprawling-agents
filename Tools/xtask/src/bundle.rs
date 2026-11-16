// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::path::{Path, PathBuf};

use crate::report::{Violation, XtaskError};
use crate::walk;

const PACKAGE: &str = "Sprawling/app";

const EMBEDDER: &str = "build.rs";

const DECLARATION: &str = "BUNDLE_DIR";

const RESTATEMENTS: [&str; 2] = ["client/vite.config.ts", "justfile"];

pub(crate) fn stated_path(root: &Path) -> Result<String, XtaskError> {
    let embedder = format!("{PACKAGE}/{EMBEDDER}");
    let text = walk::read_text(&root.join(&embedder))?;
    let parsed = syn::parse_file(&text).map_err(|err| XtaskError::Doc {
        file: embedder.clone(),
        msg: format!("this file does not parse as Rust: {err}"),
    })?;
    let declared = declared(&parsed.items).ok_or_else(|| XtaskError::Doc {
        file: embedder,
        msg: format!("no `const {DECLARATION}: &str` states where the client bundle lands"),
    })?;
    Ok(format!("{PACKAGE}/{declared}"))
}

pub(crate) fn dist(root: &Path) -> Result<PathBuf, XtaskError> {
    Ok(stated_path(root)?
        .split('/')
        .fold(root.to_path_buf(), |at, segment| at.join(segment)))
}

fn declared(items: &[syn::Item]) -> Option<String> {
    for item in items {
        if let syn::Item::Const(declared) = item
            && declared.ident == DECLARATION
            && let syn::Expr::Lit(literal) = declared.expr.as_ref()
            && let syn::Lit::Str(text) = &literal.lit
        {
            return Some(text.value());
        }
    }
    None
}

pub(crate) fn restated(root: &Path) -> Result<Vec<Violation>, XtaskError> {
    let stated = stated_path(root)?;
    let mut violations = Vec::new();
    for rel in RESTATEMENTS {
        let path = root.join(rel);
        if !path.is_file() {
            continue;
        }
        if walk::read_text(&path)?.contains(&stated) {
            continue;
        }
        violations.push(Violation {
            gate: "artifact",
            location: rel.to_owned(),
            rule: format!(
                "the client bundle lands at one path, stated by `{DECLARATION}` in \
                 {PACKAGE}/{EMBEDDER}"
            ),
            violation: format!(
                "this file spells no `{stated}`, so it writes or documents \
                 a directory the binary does not embed"
            ),
            alternative: format!(
                "spell `{stated}` here, or change `{DECLARATION}` and bring every restatement \
                 with it in the same change-set"
            ),
        });
    }
    Ok(violations)
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    reason = "test code"
)]
mod tests;
