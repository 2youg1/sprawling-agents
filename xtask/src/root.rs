// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Which tree this run judges (xtask-SPEC.md section 12, "判的是哪棵树").

use std::path::{Path, PathBuf};

use crate::report::XtaskError;

#[cfg(test)]
#[allow(clippy::unwrap_used, reason = "test code")]
pub(crate) mod fixture;

/// The name of the file cargo writes at a workspace's root, and so the
/// mark of a checkout wherever its tools sit inside it.
const LOCKFILE: &str = "Cargo.lock";

/// The checkout this run judges: the checkout `cwd` is in. `built` is
/// the xtask manifest directory compiled into the binary; when its
/// checkout is not the one found here, the binary is another checkout's
/// build (a copied target directory that cargo took for fresh), so its
/// gates are not this tree's gates and the run refuses with `StaleBuild`
/// rather than judge either tree silently. The path returned is the one
/// found from `cwd`, not its canonical form, because a verbatim `\\?\`
/// path breaks the tools gates spawn.
pub(crate) fn judged(built: &Path, cwd: &Path) -> Result<PathBuf, XtaskError> {
    let here = checkout_of(cwd)?;
    let stale = || XtaskError::StaleBuild {
        built: built.display().to_string(),
        here: here.display().to_string(),
    };
    let built_root = checkout_of(built).map_err(|_| stale())?;
    match (
        std::fs::canonicalize(built_root),
        std::fs::canonicalize(here),
    ) {
        (Ok(a), Ok(b)) if a == b => Ok(here.to_path_buf()),
        (Ok(_) | Err(_), Ok(_) | Err(_)) => Err(stale()),
    }
}

/// The checkout `dir` lies in: the first directory from `dir` upward that
/// holds `Cargo.lock`, which is where cargo puts the workspace root. How
/// many levels below it xtask sits is a fact about the layout, so nothing
/// counts parent directories to find the root (xtask-SPEC.md section 12-3).
///
/// # Errors
/// `no-checkout` when no directory from `dir` upward holds the lockfile.
pub(crate) fn checkout_of(dir: &Path) -> Result<&Path, XtaskError> {
    dir.ancestors()
        .find(|candidate| candidate.join(LOCKFILE).is_file())
        .ok_or_else(|| XtaskError::NoCheckout {
            from: dir.display().to_string(),
        })
}

/// The checkout this test binary was built from, found by the same rule.
#[cfg(test)]
#[expect(clippy::unwrap_used, reason = "test code")]
pub(crate) fn this_checkout() -> &'static Path {
    checkout_of(Path::new(env!("CARGO_MANIFEST_DIR"))).unwrap()
}

#[cfg(test)]
#[allow(clippy::unwrap_used, reason = "test code")]
mod tests {
    use super::*;

    fn checkout(parent: &Path, name: &str) -> PathBuf {
        let root = parent.join(name);
        std::fs::create_dir_all(root.join("crates")).unwrap();
        std::fs::write(root.join(LOCKFILE), "").unwrap();
        root
    }

    #[test]
    fn a_build_from_another_checkout_refuses_to_judge_this_one() {
        let scratch = std::env::temp_dir().join(format!("xtask-root-{}", std::process::id()));
        let built = checkout(&scratch, "built");
        let here = checkout(&scratch, "here");
        let verdict = judged(&built.join("tools/xtask"), &here.join("crates"));
        let same = judged(&here.join("tools/xtask"), &here.join("crates"));
        std::fs::remove_dir_all(&scratch).unwrap();
        assert!(
            matches!(verdict, Err(XtaskError::StaleBuild { .. })),
            "{verdict:?}"
        );
        assert_eq!(same.unwrap(), here);
    }

    /// xtask two levels down still finds the checkout, because the lockfile
    /// marks it, not xtask's own directory.
    #[test]
    fn a_relocated_xtask_still_finds_its_checkout() {
        let root = super::fixture::relocated("root");
        super::fixture::write(&root, "Cargo.lock", "");
        super::fixture::write(&root, "tools/xtask/Cargo.toml", "");
        let verdict = judged(&root.join("tools/xtask"), &root.join("crates"));
        std::fs::remove_dir_all(&root).unwrap();
        assert!(
            matches!(&verdict, Ok(found) if *found == root),
            "{verdict:?}"
        );
    }
}
