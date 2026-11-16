// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::path::Path;

use crate::package::{ReleaseTarget, binary_path};
use crate::report::XtaskError;

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

pub(crate) fn measure(root: &Path, name: &str) -> Result<Option<u64>, XtaskError> {
    match name {

        "frontend_artifact" => gzipped_total(&crate::bundle::dist(root)?),
        "release_binary" => Ok(binary_bytes(root)),

        "dependency_count" => lockfile_packages(root).map(Some),

        _ => Ok(None),
    }
}

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

    #[test]
    fn the_lockfile_is_counted_once_for_the_document_and_the_report() {
        let root = crate::root::this_checkout();
        let counted = lockfile_packages(root).unwrap();
        assert!(counted > 0, "this workspace resolves packages");
        assert_eq!(measure(root, "dependency_count").unwrap(), Some(counted));
    }

    #[test]
    fn a_metric_this_gate_cannot_weigh_reports_nothing_rather_than_passing() {
        let root = std::env::temp_dir();
        assert_eq!(measure(&root, "ledger_append").unwrap(), None);

        let here = crate::root::this_checkout();
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
