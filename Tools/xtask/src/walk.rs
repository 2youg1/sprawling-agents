// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use crate::report::XtaskError;

const SKIP_DIRS: [&str; 6] = [
    "target",
    "node_modules",
    ".lake",
    ".svelte-check",
    ".zig-cache",
    "web-dist",
];

const GIT: &str = ".git";

pub(crate) fn files(root: &Path) -> Result<Vec<PathBuf>, XtaskError> {
    source_files(root, Aliases::Follow)
}

pub(crate) fn files_without_aliases(root: &Path) -> Result<Vec<PathBuf>, XtaskError> {
    source_files(root, Aliases::Refuse)
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Aliases {
    Follow,
    Refuse,
}

fn source_files(root: &Path, aliases: Aliases) -> Result<Vec<PathBuf>, XtaskError> {
    let mut out = Vec::new();
    collect(root, root, &mut out, aliases)?;
    out.sort_by_key(|p| rel(root, p));
    Ok(out)
}

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

pub(crate) fn rel(root: &Path, path: &Path) -> String {
    let relative = path.strip_prefix(root).unwrap_or(path).to_string_lossy();
    #[cfg(windows)]
    return relative.replace('\\', "/");
    #[cfg(not(windows))]
    relative.into_owned()
}

pub(crate) fn in_isolation_zone(rel: &str) -> bool {
    rel == "local" || rel.starts_with("local/")
}

pub(crate) fn excluded_source_path(path: &str) -> bool {
    if in_isolation_zone(path) {
        return true;
    }
    let mut parts = path.split('/').peekable();
    while let Some(part) = parts.next() {
        if part == GIT || (parts.peek().is_some() && SKIP_DIRS.contains(&part)) {
            return true;
        }
    }
    false
}

macro_rules! client_src {
    () => {
        "RefRain/src"
    };
}
pub(crate) use client_src;

pub(crate) const CLIENT_SRC: &str = client_src!();

pub(crate) const CLIENT_SWAP: &str = "RefRain/swap";

pub(crate) fn client_files(root: &Path, keep: &[&str]) -> Result<Vec<PathBuf>, XtaskError> {
    files_under(
        root,
        std::iter::once(CLIENT_SRC).chain(root.join(CLIENT_SWAP).is_dir().then_some(CLIENT_SWAP)),
        keep,
    )
}

fn collect(
    root: &Path,
    dir: &Path,
    out: &mut Vec<PathBuf>,
    aliases: Aliases,
) -> Result<(), XtaskError> {
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
        if name == GIT || in_isolation_zone(&rel(root, &path)) {
            continue;
        }
        if path.is_dir() && SKIP_DIRS.contains(&name.as_str()) {
            continue;
        }
        if aliases == Aliases::Refuse {
            let metadata = std::fs::symlink_metadata(&path).map_err(|source| XtaskError::Io {
                path: path.display().to_string(),
                source,
            })?;
            if is_alias(&metadata) || (!metadata.is_dir() && !metadata.is_file()) {
                return Err(XtaskError::Doc {
                    file: path.display().to_string(),
                    msg: "capture source input (source-alias): replace the link, reparse point or special item with ordinary source files before running verification".to_owned(),
                });
            }
        }
        if !path.is_dir() {
            out.push(path);
        } else if !is_checkout(&path)? {
            collect(root, &path, out, aliases)?;
        }
    }
    Ok(())
}

#[cfg(windows)]
pub(crate) fn is_alias(metadata: &std::fs::Metadata) -> bool {
    use std::os::windows::fs::MetadataExt as _;
    metadata.file_attributes() & 0x400 != 0
}

#[cfg(not(windows))]
pub(crate) fn is_alias(metadata: &std::fs::Metadata) -> bool {
    metadata.file_type().is_symlink()
}

fn is_checkout(dir: &Path) -> Result<bool, XtaskError> {
    let marker = dir.join(GIT);
    marker.try_exists().map_err(|source| XtaskError::Io {
        path: marker.to_string_lossy().into_owned(),
        source,
    })
}

pub(crate) fn read_text(path: &Path) -> Result<String, XtaskError> {
    let bytes = std::fs::read(path).map_err(|source| XtaskError::Io {
        path: path.to_string_lossy().into_owned(),
        source,
    })?;
    Ok(String::from_utf8_lossy(&bytes).into_owned())
}

const GENERATED: &str = "Generated by `";

pub(crate) fn generated(text: &str) -> bool {
    text.lines().take(10).any(|line| line.contains(GENERATED))
}

#[cfg(test)]
#[allow(clippy::unwrap_used, reason = "test code")]
mod tests {
    use super::*;

    #[test]
    fn rel_normalizes_separators() {
        let root = Path::new("/tmp/checkout");
        assert_eq!(rel(root, Path::new("/tmp/checkout/a/b.rs")), "a/b.rs");
    }

    #[test]
    fn nested_worktrees_and_git_pointer_files_are_not_walked() {
        let root = std::env::temp_dir().join(format!("xtask-walk-git-{}", std::process::id()));
        if root.exists() {
            std::fs::remove_dir_all(&root).unwrap();
        }
        let nested = root.join(".pi/worktrees/pkg");
        std::fs::create_dir_all(root.join(".pi")).unwrap();
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
        assert!(!in_isolation_zone("Sprawling/kernel/src/lib.rs"));
    }

    #[test]
    fn files_does_not_enter_the_isolation_zone() {
        let root = std::env::temp_dir().join(format!("xtask-walk-local-{}", std::process::id()));
        if root.exists() {
            std::fs::remove_dir_all(&root).unwrap();
        }
        std::fs::create_dir_all(root.join("local/private")).unwrap();
        std::fs::write(root.join("kept.toml"), "").unwrap();
        std::fs::write(root.join("local/notes.toml"), "secret").unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let private = root.join("local/private");
            std::fs::set_permissions(&private, std::fs::Permissions::from_mode(0o000)).unwrap();
            let walked: Vec<String> = files(&root)
                .unwrap()
                .iter()
                .map(|p| rel(&root, p))
                .collect();
            let _ = std::fs::set_permissions(&private, std::fs::Permissions::from_mode(0o755));
            std::fs::remove_dir_all(&root).unwrap();
            assert_eq!(walked, ["kept.toml"]);
        }
        #[cfg(not(unix))]
        {
            let walked: Vec<String> = files(&root)
                .unwrap()
                .iter()
                .map(|p| rel(&root, p))
                .collect();
            std::fs::remove_dir_all(&root).unwrap();
            assert_eq!(walked, ["kept.toml"]);
        }
    }
}
