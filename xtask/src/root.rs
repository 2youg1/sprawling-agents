// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Which tree this run judges (xtask-SPEC.md section 12, "判的是哪棵树").

use std::path::{Path, PathBuf};

use crate::report::XtaskError;

/// The checkout this run judges, given the xtask manifest directory the
/// binary was built from and the directory it was started in.
pub(crate) fn judged(built: &Path, _cwd: &Path) -> Result<PathBuf, XtaskError> {
    match built.parent() {
        Some(parent) => Ok(parent.to_path_buf()),
        None => Err(XtaskError::Doc {
            file: "CARGO_MANIFEST_DIR".to_owned(),
            msg: "xtask manifest directory has no parent".to_owned(),
        }),
    }
}

#[cfg(test)]
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
}
