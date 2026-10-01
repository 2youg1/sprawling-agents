// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Where the built client lands, read from the file that embeds it.
//!
//! The build script states the location relative to its own package,
//! because a package archived for crates.io carries its own directory
//! and nothing else (xtask-SPEC.md section 8-18); this joins it onto that
//! package's directory and hands every reader the whole repo-relative
//! path: a name with one home and a parent directory spelled once per
//! reader still let the bundler write where the build script did not look.
//!
//! The directory name had four homes: the build script that embeds the
//! bundle, the bundler that writes it, the gate that opens it, and the
//! gate that weighs it. None referred to another, so renaming the output
//! directory left a green build behind: the build script embedded a
//! placeholder with one warning, `render` and `budget` skipped, and the
//! defect reached a person as a blank page.
//!
//! **The path lives in `crates/sprawling/build.rs`, and this reads it
//! from there.** That file is the only reader that must work in the
//! published tree and in the crates.io archive, neither of which carries
//! `tools/xtask/`, and it is the first reader in any build; a copy kept
//! here would be the second home again. The
//! two files that restate the path in another language are held to it by
//! `restated`, which reports rather than rewrites — a gate does not edit
//! a bundler's configuration.

use std::path::{Path, PathBuf};

use crate::report::{Violation, XtaskError};
use crate::walk;

/// The package whose build script embeds the bundle, repo-relative.
const PACKAGE: &str = "crates/sprawling";

/// The file in it that owns the bundle's path.
const EMBEDDER: &str = "build.rs";

/// The constant in it that states the path.
const DECLARATION: &str = "BUNDLE_DIR";

/// The files in other languages that have to spell the same path:
/// the bundler that writes it, and the recipe that documents where a
/// contributor's build goes.
const RESTATEMENTS: [&str; 2] = ["client/vite.config.ts", "justfile"];

/// The workspace-relative path the product embeds, `/`-separated: the
/// package's directory, then the path its build script states.
///
/// # Errors
/// When the build script cannot be read or parsed, or when it no longer
/// declares the constant: a gate that guessed a path here would weigh an
/// empty directory and report a bundle of zero bytes.
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

/// Where the built client lands in this checkout.
///
/// # Errors
/// Propagates [`stated_path`]: the directory is the workspace root plus the
/// path the build script states, joined segment by segment so a path
/// on Windows carries one separator.
pub(crate) fn dist(root: &Path) -> Result<PathBuf, XtaskError> {
    Ok(stated_path(root)?
        .split('/')
        .fold(root.to_path_buf(), |at, segment| at.join(segment)))
}

/// The value of the declaration, or nothing when this file no longer
/// carries it.
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

/// Every file that restates the bundle's path still spells the one the
/// product embeds.
///
/// # Errors
/// Propagates the read of the build script and of each restating file.
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
