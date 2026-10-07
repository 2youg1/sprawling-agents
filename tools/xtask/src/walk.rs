// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Deterministic file walker: sorted output, fixed skip set, forward-slash
//! relative paths. Determinism makes gate reports diffable across runs and
//! platforms (tools/xtask/Spec.lean §10-1).

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use crate::report::XtaskError;

/// Directories a gate never walks into: the build output of the
/// toolchains this tree can contain.
///
/// **A gate testifies about committed objects**, and a build directory
/// holds none - `.gitignore` names every one of these. `.lake` is on the
/// list because the adversarial checker compiles beside its own source:
/// ninety-two generated files sit there after one build, and every
/// finding a scan reads out of them is a true statement about something
/// no reader will ever receive. A gate that reports generated files
/// reports nothing, because nobody reads past the first screen of them.
///
/// `web-dist` is the client bundle `just build-web` writes into the
/// `sprawling` package, where its build script and its crates.io archive
/// find it (tools/xtask/Spec.lean §8-18): Vite's output, minified.
///
/// `.zig-cache` is where `mlugg/setup-zig` points Zig's global cache,
/// `$GITHUB_WORKSPACE/.zig-cache`, and the leaf's build fills it: a
/// `zig build-lib` overrides only the local cache, so the system
/// libraries it resolves land here. It is walked where CI's `gates` job
/// is green and the release's `verify` job is red for it, because
/// `verify` runs `just check`, whose first step builds the workspace and
/// whose third then reads the binary contents of the cache: twenty-odd
/// high-entropy findings about a `gdi32.lib` nobody will ever receive.
///
/// This list is a second authority for "what is in the tree", and git is
/// the first. It stays a list rather than a `.gitignore` reader because
/// six names cost six tokens and a parser costs a parser - but that is
/// the parameter: **the day this list needs an entry that is not a build
/// directory, read the ignore file instead of adding a row.**
const SKIP_DIRS: [&str; 6] = [
    "target",
    "node_modules",
    ".lake",
    ".svelte-check",
    ".zig-cache",
    "web-dist",
];

/// Version control's own entry. It is never walked, whether a directory or
/// the pointer file a worktree or submodule carries (which names an
/// absolute path on one machine), and a directory below the root that
/// holds one is another checkout - `git worktree add` into `.pi/worktrees/`
/// for example - so none of it is this tree's committed objects. The rule
/// is structural rather than a row in [`SKIP_DIRS`], so it holds wherever a
/// tool puts its checkouts (tools/xtask/Spec.lean §8, "the scan surface").
const GIT: &str = ".git";

/// All regular files under `root`, sorted by their relative forward-slash path.
pub(crate) fn files(root: &Path) -> Result<Vec<PathBuf>, XtaskError> {
    let mut out = Vec::new();
    collect(root, &mut out)?;
    out.sort_by_key(|p| rel(root, p));
    Ok(out)
}

/// Like [`files`], keeping only the given extensions (lowercase, no dot).
pub(crate) fn files_with_ext(root: &Path, keep: &[&str]) -> Result<Vec<PathBuf>, XtaskError> {
    let all = files(root)?;
    Ok(all
        .into_iter()
        .filter(|p| {
            p.extension()
                .and_then(|e| e.to_str())
                .is_some_and(|e| keep.contains(&e))
        })
        .collect())
}

/// Like [`files_with_ext`], over several directories of `root` at once:
/// each file once however many of the directories hold it, which they
/// do when one package sits inside another's directory, sorted by its
/// relative path as [`files`] sorts.
pub(crate) fn files_under<'d>(
    root: &Path,
    dirs: impl IntoIterator<Item = &'d str>,
    keep: &[&str],
) -> Result<Vec<PathBuf>, XtaskError> {
    let mut found = BTreeMap::new();
    for dir in dirs {
        for file in files_with_ext(&root.join(dir), keep)? {
            found.entry(rel(root, &file)).or_insert(file);
        }
    }
    Ok(found.into_values().collect())
}

/// Repo-relative path with forward slashes; used for all matching and reports
/// because the module table is written with forward slashes.
pub(crate) fn rel(root: &Path, path: &Path) -> String {
    let full = path.to_string_lossy().replace('\\', "/");
    let base = root.to_string_lossy().replace('\\', "/");
    match full.strip_prefix(&base) {
        Some(tail) => tail.trim_start_matches('/').to_owned(),
        None => full,
    }
}

/// The isolation zone: root-level `local/` holds handoffs and machine-local
/// notes, is gitignored, and never enters the tree. Gates testify about
/// committed objects only, so repo-root scans exclude it (tools/xtask/Spec.lean §10-1).
pub(crate) fn in_isolation_zone(rel: &str) -> bool {
    rel == "local" || rel.starts_with("local/")
}

/// Where the web client's sources are, spelled once for every gate that
/// walks them.
///
/// Five gates asked this question and five gates answered it: `length`
/// held `CLIENT_DIR`, `wiring` and `wording` each held a `CLIENT`, and
/// `color` and `wire-ts` each spelled the directory inside a longer
/// path. Nothing compared the five, so moving the client would have
/// left four gates walking a directory that no longer existed and
/// reporting green about a tree they never opened.
///
/// It is a macro as well as a constant because the two longer paths are
/// `const` items, and `concat!` takes literals: the macro is how a
/// compile-time path is built from this one without a dependency and
/// without a second spelling.
macro_rules! client_src {
    () => {
        "client/src"
    };
}
pub(crate) use client_src;

/// The directory the macro above names, for the readers that want a value.
pub(crate) const CLIENT_SRC: &str = client_src!();

fn collect(dir: &Path, out: &mut Vec<PathBuf>) -> Result<(), XtaskError> {
    let entries = std::fs::read_dir(dir).map_err(|source| XtaskError::Io {
        path: dir.to_string_lossy().into_owned(),
        source,
    })?;
    for entry in entries {
        let entry = entry.map_err(|source| XtaskError::Io {
            path: dir.to_string_lossy().into_owned(),
            source,
        })?;
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().into_owned();
        if name == GIT {
            continue;
        }
        if !path.is_dir() {
            out.push(path);
        } else if !SKIP_DIRS.contains(&name.as_str()) && !is_checkout(&path)? {
            collect(&path, out)?;
        }
    }
    Ok(())
}

fn is_checkout(dir: &Path) -> Result<bool, XtaskError> {
    let marker = dir.join(GIT);
    marker.try_exists().map_err(|source| XtaskError::Io {
        path: marker.to_string_lossy().into_owned(),
        source,
    })
}

/// Read a file as (lossy) UTF-8; gates judge text, they never panic on bytes.
pub(crate) fn read_text(path: &Path) -> Result<String, XtaskError> {
    let bytes = std::fs::read(path).map_err(|source| XtaskError::Io {
        path: path.to_string_lossy().into_owned(),
        source,
    })?;
    Ok(String::from_utf8_lossy(&bytes).into_owned())
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]
mod tests {
    use super::*;

    #[test]
    fn rel_normalizes_separators() {
        let root = Path::new("C:\\repo");
        let file = Path::new("C:\\repo\\crates\\kernel\\src\\lib.rs");
        assert_eq!(rel(root, file), "crates/kernel/src/lib.rs");
    }

    /// A checkout made by `git worktree add` inside the tree - its `.git` is
    /// a pointer file naming an absolute path - is another tree, and the
    /// root's own `.git` pointer is not a committed object either.
    #[test]
    fn nested_worktrees_and_git_pointer_files_are_not_walked() {
        let root = std::env::temp_dir().join(format!("walk-worktree-{}", std::process::id()));
        let nested = root.join(".pi/worktrees/pkg");
        std::fs::create_dir_all(nested.join("src")).unwrap();
        std::fs::write(root.join(".git"), "gitdir: C:/elsewhere/.git/worktrees/x\n").unwrap();
        std::fs::write(root.join("kept.rs"), "").unwrap();
        std::fs::write(root.join(".pi/settings.json"), "{}").unwrap();
        std::fs::write(
            nested.join(".git"),
            "gitdir: C:/elsewhere/.git/worktrees/pkg\n",
        )
        .unwrap();
        std::fs::write(nested.join("src/lib.rs"), "").unwrap();

        let walked: Vec<String> = files(&root)
            .unwrap()
            .iter()
            .map(|p| rel(&root, p))
            .collect();
        std::fs::remove_dir_all(&root).unwrap();

        assert_eq!(walked, [".pi/settings.json", "kept.rs"]);
    }

    #[test]
    fn isolation_zone_is_the_local_prefix_only() {
        assert!(in_isolation_zone("local/Handoff.md"));
        assert!(in_isolation_zone("local"));
        assert!(!in_isolation_zone("localx/notes.md"));
        assert!(!in_isolation_zone("crates/kernel/src/lib.rs"));
    }
}
