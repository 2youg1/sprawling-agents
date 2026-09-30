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

/// The checkout this run judges: the first directory from `cwd` upward
/// that holds `xtask/Cargo.toml`, the rule cargo itself follows to find
/// the workspace. `built` is the xtask manifest directory compiled into
/// the binary; when its checkout is not the one found here, the binary
/// is another checkout's build (a copied target directory that cargo
/// took for fresh), so its gates are not this tree's gates and the run
/// refuses with `StaleBuild` rather than judge either tree silently.
/// The path returned is the one found from `cwd`, not its canonical
/// form, because a verbatim `\\?\` path breaks the tools gates spawn.
pub(crate) fn judged(built: &Path, cwd: &Path) -> Result<PathBuf, XtaskError> {
    let here = cwd
        .ancestors()
        .find(|dir| dir.join("xtask").join("Cargo.toml").is_file())
        .ok_or_else(|| XtaskError::NoCheckout {
            cwd: cwd.display().to_string(),
        })?;
    let stale = || XtaskError::StaleBuild {
        built: built.display().to_string(),
        here: here.display().to_string(),
    };
    let built_root = built.parent().ok_or_else(stale)?;
    match (
        std::fs::canonicalize(built_root),
        std::fs::canonicalize(here),
    ) {
        (Ok(a), Ok(b)) if a == b => Ok(here.to_path_buf()),
        (Ok(_) | Err(_), Ok(_) | Err(_)) => Err(stale()),
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, reason = "test code")]
mod tests {
    use super::*;

    fn checkout(parent: &Path, name: &str) -> PathBuf {
        let root = parent.join(name);
        std::fs::create_dir_all(root.join("xtask")).unwrap();
        std::fs::create_dir_all(root.join("crates")).unwrap();
        std::fs::write(root.join("xtask").join("Cargo.toml"), "").unwrap();
        root
    }

    #[test]
    fn a_build_from_another_checkout_refuses_to_judge_this_one() {
        let scratch = std::env::temp_dir().join(format!("xtask-root-{}", std::process::id()));
        let built = checkout(&scratch, "built");
        let here = checkout(&scratch, "here");
        let verdict = judged(&built.join("xtask"), &here.join("crates"));
        let same = judged(&here.join("xtask"), &here.join("crates"));
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
