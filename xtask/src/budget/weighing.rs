// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! One reading, taken off what this checkout actually holds.
//!
//! The register states what a number may be; this module answers what
//! it is today. The two are apart because they fail apart: a register
//! row is wrong when the design changed and nobody said so, and a
//! reading is wrong when the thing it weighs was never built.
//!
//! **A metric this module does not know weighs nothing rather than
//! passing.** `measure` answers `None` for a name it cannot take a
//! reading for, and the gate stays silent on it, so a row added to the
//! register before its reader exists is an unmeasured row instead of a
//! budget that quietly holds.

use std::path::Path;

use crate::package::{ReleaseTarget, binary_path};
use crate::report::XtaskError;

/// How many packages the resolved lockfile names, workspace members
/// included.
///
/// The one home for that count. `docnum`'s `dependency_count` fact
/// quotes this reading into the documents that state it, and the
/// register row below prices it, so the number a reader meets in
/// `ARCHITECTURE.md`, the number `docs/third-party.md` states and the
/// number the ratchet refuses growth past are one reading taken once.
///
/// # Errors
/// Refuses a lockfile it cannot read, and one that resolves no package
/// at all, which is what a changed lockfile format looks like, and a
/// count that quietly fell to zero would rewrite every document that
/// quotes it.
pub(crate) fn lockfile_packages(root: &Path) -> Result<u64, XtaskError> {
    let text = crate::walk::read_text(&root.join("Cargo.lock"))?;
    let found = text.lines().filter(|line| *line == "[[package]]").count();
    if found == 0 {
        return Err(XtaskError::Doc {
            file: "Cargo.lock".to_owned(),
            msg: "no `[[package]]` entries; the lockfile format changed".to_owned(),
        });
    }
    u64::try_from(found).map_err(|_| XtaskError::Doc {
        file: "Cargo.lock".to_owned(),
        msg: format!("{found} packages is more than this counter can carry"),
    })
}

/// Weighs one metric, or says it is not built.
pub(crate) fn measure(root: &Path, name: &str) -> Result<Option<u64>, XtaskError> {
    match name {
        // Where the bundle lands is stated by the build script that
        // embeds it, so the scale and the binary weigh one directory.
        "frontend_artifact" => gzipped_total(&crate::bundle::dist(root)?),
        "release_binary" => Ok(binary_bytes(root)),
        // Read off the lockfile rather than off a build, so this row
        // answers in a checkout nobody has compiled.
        "dependency_count" => lockfile_packages(root).map(Some),
        // A gated row with no way to weigh it would silently pass; it is
        // an unmeasured row until this match learns it.
        _ => Ok(None),
    }
}

/// What the browser downloads: every file in the bundle, compressed.
/// Recursive, because the bundle nests (`snippets/<crate>/...`), and a
/// weight that skipped subdirectories would flatter the artifact.
fn gzipped_total(dist: &Path) -> Result<Option<u64>, XtaskError> {
    let Ok(entries) = std::fs::read_dir(dist) else {
        return Ok(None);
    };
    let mut total = 0u64;
    let mut found = false;
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            if let Some(nested) = gzipped_total(&path)? {
                found = true;
                total = total.saturating_add(nested);
            }
            continue;
        }
        let bytes = std::fs::read(&path).map_err(|source| XtaskError::Io {
            path: path.display().to_string(),
            source,
        })?;
        found = true;
        total = total.saturating_add(gzipped_len(&bytes));
    }
    Ok(found.then_some(total))
}

/// The compressed length, computed rather than shelled out for, so the
/// number does not depend on which gzip is on the path.
fn gzipped_len(bytes: &[u8]) -> u64 {
    let mut encoder = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::best());
    use std::io::Write as _;
    if encoder.write_all(bytes).is_err() {
        return u64::MAX;
    }
    match encoder.finish() {
        Ok(compressed) => u64::try_from(compressed.len()).unwrap_or(u64::MAX),
        Err(_) => u64::MAX,
    }
}

fn binary_bytes(root: &Path) -> Option<u64> {
    let path = binary_path(root, &ReleaseTarget::Host)?;
    std::fs::metadata(path).ok().map(|meta| meta.len())
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    reason = "test code"
)]
mod tests {
    use super::*;

    /// The lockfile is counted once, and both the document fact and
    /// the register row read that one count.
    #[test]
    fn the_lockfile_is_counted_once_for_the_document_and_the_ratchet() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
        let counted = lockfile_packages(root).unwrap();
        assert!(counted > 0, "this workspace resolves packages");
        assert_eq!(measure(root, "dependency_count").unwrap(), Some(counted));
    }

    #[test]
    fn a_metric_this_gate_cannot_weigh_reports_nothing_rather_than_passing() {
        let root = std::env::temp_dir();
        assert_eq!(measure(&root, "ledger_append").unwrap(), None);
        // A weighable row answers from this checkout, and answers
        // `None` rather than failing when nothing has been built.
        let here = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
        measure(here, "frontend_artifact").unwrap();
    }

    #[test]
    fn compression_is_ours_rather_than_whatever_gzip_is_on_the_path() {
        assert!(gzipped_len(b"") > 0);
        let repetitive = vec![b'a'; 10_000];
        assert!(gzipped_len(&repetitive) < 1_000);
    }

    #[test]
    fn the_bundle_weight_counts_nested_directories() {
        let dir = std::env::temp_dir().join(format!("budget-walk-{}", std::process::id()));
        std::fs::create_dir_all(dir.join("snippets").join("x")).unwrap();
        std::fs::write(dir.join("web.js"), vec![b'a'; 4_000]).unwrap();
        std::fs::write(
            dir.join("snippets").join("x").join("y.js"),
            vec![b'b'; 4_000],
        )
        .unwrap();
        let total = gzipped_total(&dir).unwrap().unwrap();
        let flat = gzipped_len(&vec![b'a'; 4_000]);
        assert!(total > flat, "the nested file must be weighed too");
        std::fs::remove_dir_all(&dir).ok();
    }
}
