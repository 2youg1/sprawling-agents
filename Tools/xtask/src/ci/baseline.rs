// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Fixed Git objects compared to the owned input manifest, never live diff.

use std::collections::{BTreeMap, BTreeSet};
use std::io::Write as _;
use std::path::Path;
use std::process::{Command, Output, Stdio};

use crate::report::XtaskError;

use super::view::Snapshot;

pub(crate) fn head(root: &Path) -> Result<String, XtaskError> {
    revision(root, "HEAD", "commit")
}

pub(crate) fn changed(
    root: &Path,
    base: &str,
    captured: &Snapshot,
) -> Result<BTreeSet<String>, XtaskError> {
    let tree = revision(root, base, "tree")?;
    let output = git(root, &["ls-tree", "-r", "-z", "--full-tree", &tree])?;
    let entries = entries(&output.stdout)?;
    let bytes = blobs(root, &entries)?;
    let mut remaining = bytes.as_slice();
    let mut previous = BTreeMap::new();
    for entry in entries {
        let digest = blob(&mut remaining, &entry.oid)?;
        if previous.insert(entry.path, (digest, entry.mode)).is_some() {
            return Err(invalid("duplicate Git tree path"));
        }
    }
    if !remaining.is_empty() {
        return Err(invalid("unexpected trailing Git blob data"));
    }
    let mut changed = BTreeSet::new();
    for (path, file) in captured.files() {
        match previous.remove(path) {
            Some((before, mode))
                if before == file.digest
                    && match mode.as_str() {
                        "100644" => file.executable != Some(true),
                        "100755" => file.executable != Some(false),
                        _ => false,
                    } => {}
            _ => {
                changed.insert(path.clone());
            }
        }
    }
    changed.extend(previous.into_keys());
    Ok(changed)
}

fn revision(root: &Path, value: &str, kind: &str) -> Result<String, XtaskError> {
    let output = git(
        root,
        &[
            "rev-parse",
            "--verify",
            "--end-of-options",
            &format!("{value}^{{{kind}}}"),
        ],
    )?;
    let oid = std::str::from_utf8(&output.stdout)
        .map_err(|error| invalid(&error.to_string()))?
        .trim();
    if ![40, 64].contains(&oid.len()) || !oid.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(invalid("Git did not return one complete object ID"));
    }
    Ok(oid.to_owned())
}

struct Entry {
    path: String,
    oid: String,
    mode: String,
}

fn entries(bytes: &[u8]) -> Result<Vec<Entry>, XtaskError> {
    let mut entries = Vec::new();
    for record in bytes
        .split(|byte| *byte == 0)
        .filter(|record| !record.is_empty())
    {
        let mut fields = record.splitn(2, |byte| *byte == b'\t');
        let header = fields
            .next()
            .ok_or_else(|| invalid("missing Git tree header"))?;
        let path = fields
            .next()
            .ok_or_else(|| invalid("missing Git tree path"))?;
        let path = std::str::from_utf8(path)
            .map_err(|error| invalid(&format!("non-UTF-8 Git path: {error}")))?;
        let header = std::str::from_utf8(header).map_err(|error| invalid(&error.to_string()))?;
        let mut parts = header.split(' ');
        let mode = parts
            .next()
            .ok_or_else(|| invalid("missing Git tree mode"))?;
        let kind = parts
            .next()
            .ok_or_else(|| invalid("missing Git object type"))?;
        let oid = parts
            .next()
            .ok_or_else(|| invalid("missing Git object ID"))?;
        if kind == "commit" || crate::walk::excluded_source_path(path) {
            continue;
        }
        if kind != "blob" || parts.next().is_some() {
            return Err(invalid("expected a blob in the recursive Git tree"));
        }
        entries.push(Entry {
            path: path.to_owned(),
            oid: oid.to_owned(),
            mode: mode.to_owned(),
        });
    }
    Ok(entries)
}

#[expect(
    clippy::disallowed_methods,
    reason = "developer tool Git object reader (child D4)"
)]
fn blobs(root: &Path, entries: &[Entry]) -> Result<Vec<u8>, XtaskError> {
    let mut child = Command::new("git")
        .current_dir(root)
        .args(["--no-replace-objects", "cat-file", "--batch"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|source| io("git cat-file --batch", source))?;
    let mut input = child
        .stdin
        .take()
        .ok_or_else(|| invalid("Git blob input pipe unavailable"))?;
    // Feeding and collecting concurrently also works when the object list
    // exceeds a pipe buffer; no filename is interpreted as a command.
    let output = std::thread::scope(|scope| {
        let writer = scope.spawn(move || {
            for entry in entries {
                writeln!(input, "{}", entry.oid)
                    .map_err(|source| io("feed Git object IDs", source))?;
            }
            Ok::<(), XtaskError>(())
        });
        let output = child
            .wait_with_output()
            .map_err(|source| io("read Git blobs", source));
        let fed = writer
            .join()
            .map_err(|_panic| invalid("Git object writer failed"))?;
        fed?;
        output
    })?;
    success("git cat-file --batch", output).map(|output| output.stdout)
}

fn blob(bytes: &mut &[u8], oid: &str) -> Result<blake3::Hash, XtaskError> {
    let newline = bytes
        .iter()
        .position(|byte| *byte == b'\n')
        .ok_or_else(|| invalid("missing Git blob header terminator"))?;
    let header = bytes
        .get(..newline)
        .ok_or_else(|| invalid("invalid Git blob header"))?;
    let header = std::str::from_utf8(header).map_err(|error| invalid(&error.to_string()))?;
    let prefix = format!("{oid} blob ");
    let size = header
        .strip_prefix(&prefix)
        .ok_or_else(|| invalid("Git returned a different object or object type"))?
        .parse::<usize>()
        .map_err(|error| invalid(&error.to_string()))?;
    let start = newline
        .checked_add(1)
        .ok_or_else(|| invalid("Git blob header length overflow"))?;
    let end = start
        .checked_add(size)
        .ok_or_else(|| invalid("Git blob size overflow"))?;
    let content = bytes
        .get(start..end)
        .ok_or_else(|| invalid("truncated Git blob"))?;
    let digest = blake3::hash(content);
    if bytes.get(end) != Some(&b'\n') {
        return Err(invalid("missing Git blob terminator"));
    }
    let next = end
        .checked_add(1)
        .ok_or_else(|| invalid("Git blob length overflow"))?;
    *bytes = bytes
        .get(next..)
        .ok_or_else(|| invalid("invalid Git blob boundary"))?;
    Ok(digest)
}

#[expect(
    clippy::disallowed_methods,
    reason = "developer tool Git object reader (child D4)"
)]
pub(super) fn git(root: &Path, args: &[&str]) -> Result<Output, XtaskError> {
    let command = format!("git {}", args.join(" "));
    let output = Command::new("git")
        .arg("--no-replace-objects")
        .current_dir(root)
        .args(args)
        .output()
        .map_err(|source| io(&command, source))?;
    success(&command, output)
}

fn success(command: &str, output: Output) -> Result<Output, XtaskError> {
    if output.status.success() {
        Ok(output)
    } else {
        Err(XtaskError::Cmd {
            cmd: command.to_owned(),
            msg: String::from_utf8_lossy(&output.stderr).into_owned(),
        })
    }
}

fn io(path: &str, source: std::io::Error) -> XtaskError {
    XtaskError::Io {
        path: path.to_owned(),
        source,
    }
}

fn invalid(msg: &str) -> XtaskError {
    XtaskError::Doc {
        file: "immutable Git baseline (baseline-data)".to_owned(),
        msg: format!(
            "{msg}; run git fsck, use a valid UTF-8 source tree and choose an existing base revision"
        ),
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, reason = "test code")]
mod tests {
    use super::*;

    fn command(root: &Path, args: &[&str]) {
        git(root, args).unwrap();
    }

    #[test]
    fn fixed_objects_are_compared_to_captured_unicode_paths_not_the_live_tree() {
        let root = tempfile::tempdir().unwrap();
        command(root.path(), &["init", "--quiet"]);
        std::fs::create_dir(root.path().join("src")).unwrap();
        let existing = "src/已有 文件.rs";
        let added = if cfg!(windows) {
            "src/新增 文件.rs"
        } else {
            "src/新增\n文件.rs"
        };
        std::fs::write(root.path().join(existing), "base").unwrap();
        std::fs::write(root.path().join("deleted.txt"), "base").unwrap();
        command(root.path(), &["add", "--all"]);
        command(
            root.path(),
            &[
                "-c",
                "user.name=Fixture",
                "-c",
                "user.email=fixture@example.invalid",
                "-c",
                "commit.gpgsign=false",
                "commit",
                "--quiet",
                "-m",
                "fixture",
            ],
        );
        let base = head(root.path()).unwrap();
        std::fs::write(root.path().join(existing), "captured").unwrap();
        std::fs::remove_file(root.path().join("deleted.txt")).unwrap();
        std::fs::write(root.path().join(added), "captured").unwrap();
        let captured = Snapshot::capture(root.path()).unwrap();
        // The live tree returns to A before comparison; the owned B remains
        // the actual execution input, including the untracked/deleted paths.
        command(root.path(), &["restore", "--worktree", "."]);
        std::fs::remove_file(root.path().join(added)).unwrap();
        assert_eq!(
            changed(root.path(), &base, &captured).unwrap(),
            BTreeSet::from([
                existing.to_owned(),
                added.to_owned(),
                "deleted.txt".to_owned()
            ])
        );
        captured.close().unwrap();
        root.close().unwrap();
    }

    #[test]
    fn replacement_refs_cannot_change_fixed_tree_or_blob_bytes() {
        let root = tempfile::tempdir().unwrap();
        command(root.path(), &["init", "--quiet"]);
        std::fs::write(root.path().join("original.txt"), "original\n").unwrap();
        std::fs::write(root.path().join("replacement.txt"), "replacement\n").unwrap();
        command(root.path(), &["add", "--all"]);
        let oid = |args: &[&str]| {
            String::from_utf8(git(root.path(), args).unwrap().stdout)
                .unwrap()
                .trim()
                .to_owned()
        };
        let original = oid(&["hash-object", "original.txt"]);
        let replacement = oid(&["hash-object", "replacement.txt"]);
        let base = oid(&["write-tree"]);
        let captured = Snapshot::capture(root.path()).unwrap();
        command(root.path(), &["replace", &original, &replacement]);
        assert!(
            changed(root.path(), &base, &captured).unwrap().is_empty(),
            "a blob replacement cannot rewrite the baseline bytes"
        );
        command(root.path(), &["replace", "-d", &original]);
        command(
            root.path(),
            &[
                "update-index",
                "--cacheinfo",
                &format!("100644,{replacement},original.txt"),
            ],
        );
        let replaced_tree = oid(&["write-tree"]);
        command(root.path(), &["replace", &base, &replaced_tree]);
        assert!(
            changed(root.path(), &base, &captured).unwrap().is_empty(),
            "a tree replacement cannot rewrite the baseline entries"
        );
        captured.close().unwrap();
        root.close().unwrap();
    }

    #[test]
    fn nul_records_preserve_tabs_newlines_and_backslashes() {
        let rows = entries(b"100644 blob aaaa\tname\twith\nnewline\\and-slash.rs\0").unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(
            rows.first().unwrap().path,
            "name\twith\nnewline\\and-slash.rs"
        );
        assert!(entries(b"100644 blob aaaa\tbad\xff\0").is_err());
    }

    #[cfg(unix)]
    #[test]
    fn owner_executable_mode_survives_capture_and_changes_the_base_comparison() {
        use std::os::unix::fs::PermissionsExt as _;

        let root = tempfile::tempdir().unwrap();
        command(root.path(), &["init", "--quiet"]);
        let path = root.path().join("fixture.sh");
        std::fs::write(&path, "exit 0\n").unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o644)).unwrap();
        command(root.path(), &["add", "fixture.sh"]);
        let tree = git(root.path(), &["write-tree"]).unwrap().stdout;
        let tree = std::str::from_utf8(&tree).unwrap().trim();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o744)).unwrap();
        let snapshot = Snapshot::capture(root.path()).unwrap();
        assert_eq!(
            changed(root.path(), tree, &snapshot).unwrap(),
            BTreeSet::from(["fixture.sh".to_owned()])
        );
        assert!(
            std::fs::metadata(snapshot.root().join("fixture.sh"))
                .unwrap()
                .permissions()
                .mode()
                & 0o100
                != 0
        );
        snapshot.close().unwrap();
        root.close().unwrap();
    }
}
