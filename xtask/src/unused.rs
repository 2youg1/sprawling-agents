// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Dead dependencies: a key a manifest declares that no source file of
//! its package names, and a workspace dependency no package inherits
//! (xtask-SPEC.md section 8-34). rustc's `dead_code` already refuses a
//! dead private item under `-D warnings`; a dead manifest entry is the
//! kind of dead code no compiler pass reports.

use std::path::Path;

use crate::report::{Violation, XtaskError};

pub(crate) fn check(_root: &Path) -> Result<Vec<Violation>, XtaskError> {
    Ok(Vec::new())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write(root: &Path, rel: &str, text: &str) {
        let path = root.join(rel);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, text).unwrap();
    }

    #[test]
    fn names_a_dependency_no_source_names_and_a_workspace_entry_no_package_inherits() {
        let root = std::env::temp_dir().join(format!("xtask-unused-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        write(
            &root,
            "Cargo.toml",
            "[workspace]\nmembers = [\"a\"]\n\n[workspace.dependencies]\nused = \"1\"\norphan = \"1\"\n",
        );
        write(
            &root,
            "a/Cargo.toml",
            "[package]\nname = \"a\"\n\n[dependencies]\nused = { workspace = true }\nsplit-name = \"1\"\nnamed_prefix = \"1\"\n\n[dev-dependencies]\nghost = \"1\"\n\n[target.'cfg(windows)'.dependencies]\nwinonly = \"1\"\n",
        );
        write(
            &root,
            "a/src/lib.rs",
            "use used::Thing;\nfn f() { split_name::go(); named_prefix_more::go(); }\n",
        );
        write(&root, "a/tests/t.rs", "#[test] fn t() { winonly::go(); }\n");

        let found: Vec<(String, String)> = check(&root)
            .unwrap()
            .into_iter()
            .map(|v| (v.location, v.violation))
            .collect();
        std::fs::remove_dir_all(&root).unwrap();

        assert_eq!(
            found,
            vec![
                (
                    "a/Cargo.toml".to_owned(),
                    "`[dependencies]` declares `named_prefix`, and no `.rs` file under `a` names `named_prefix`".to_owned()
                ),
                (
                    "a/Cargo.toml".to_owned(),
                    "`[dev-dependencies]` declares `ghost`, and no `.rs` file under `a` names `ghost`".to_owned()
                ),
                (
                    "Cargo.toml".to_owned(),
                    "`[workspace.dependencies]` declares `orphan`, and no package inherits it".to_owned()
                ),
            ]
        );
    }
}
