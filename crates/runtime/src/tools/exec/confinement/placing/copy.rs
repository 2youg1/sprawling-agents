// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The copy a command runs in: one working tree, made fresh for one
//! command, bounded, and removed when the command that ran in it ends.
//!
//! The bound is a refusal rather than a partial copy. Half a tree would
//! answer for files the copy never carried, and a command that then
//! reads a stale or missing file has no way to tell that apart from a
//! file somebody changed.

use std::path::{Path, PathBuf};

use kernel::{AxCode, AxError};

/// How many files one command's working tree may hold before the copy
/// refuses to make itself. A room holding a build cache is the case
/// this bounds: the point of the copy is a throwaway environment for
/// one command, and a tree too large to copy in seconds is one no
/// default path should spend on every call.
pub(super) const MAX_FILES: u64 = 100_000;

/// How many bytes those files may hold. The pair is a budget rather
/// than a promise about the machine: past either figure the refusal
/// names the figure, and `where: host` is the caller's own decision.
pub(super) const MAX_BYTES: u64 = 256 * 1024 * 1024;

/// How deep a working tree may nest before the copy refuses. A link
/// that points at its own parent is a loop no walk can end, and a
/// refusal that names the depth is better than a walk that grows until
/// the process dies.
const MAX_DEPTH: u32 = 64;

/// Refusal for a copy that cannot be made.
fn copy_fault(subject: &str, err: &std::io::Error) -> AxError {
    AxError::failure(
        AxCode::SandboxDenied,
        "copy the working tree",
        format!("{subject}: {err}"),
    )
    .with_recovery(
        "the sandbox copies the working directory before a command runs; a directory \
         that cannot be read cannot be copied",
    )
}

/// Refusal for a tree past the budget, naming the figure that was passed.
fn over_budget(files: u64, bytes: u64) -> AxError {
    AxError::failure(
        AxCode::SandboxDenied,
        "copy the working tree",
        format!(
            "it holds {files} files and {bytes} bytes, past the {MAX_FILES}-file, \
             {MAX_BYTES}-byte budget one sandboxed command is given"
        ),
    )
    .with_recovery(
        "run the command with `where: host`, which is the placement a person decides \
         on, or point the room at a smaller directory",
    )
}

/// Removal as its own step, so the failure of one is not the failure of
/// the copy that was made.
pub(super) fn remove(copy: &Path) {
    drop(std::fs::remove_dir_all(copy));
}

/// A directory of this machine's scratch root that no other copy holds.
pub(super) fn fresh(root: &Path) -> Result<PathBuf, AxError> {
    let pid = std::process::id();
    for attempt in 0..64u32 {
        let candidate = root.join(format!("sprawling-sandbox-{pid}-{attempt}"));
        match std::fs::create_dir(&candidate) {
            Ok(()) => return Ok(candidate),
            Err(err) if err.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(err) => return Err(copy_fault(&root.display().to_string(), &err)),
        }
    }
    Err(AxError::failure(
        AxCode::StorageFatal,
        "make a scratch directory",
        format!("{pid} has 64 copies already outstanding"),
    )
    .with_recovery(
        "remove the `sprawling-sandbox-*` directories in the scratch root and run the \
         command again",
    ))
}

/// What one command's copy may hold, counted as it is made.
pub(super) struct Budget {
    files: u64,
    bytes: u64,
}

impl Budget {
    pub(super) fn new() -> Budget {
        Budget { files: 0, bytes: 0 }
    }

    pub(super) fn take(&mut self, bytes: u64) -> Result<(), AxError> {
        self.files = self.files.saturating_add(1);
        self.bytes = self.bytes.saturating_add(bytes);
        if self.files > MAX_FILES || self.bytes > MAX_BYTES {
            return Err(over_budget(self.files, self.bytes));
        }
        Ok(())
    }
}

/// Copies `from` into the existing directory `into`.
///
/// `copy` is this copy's own directory, and it is skipped: a working
/// directory that contains the place copies go - this machine's scratch
/// root is one - would otherwise be copied into itself, once per copy. A
/// link is followed, so a working tree that points at a directory outside
/// it carries that directory's content rather than a link into the
/// person's tree; a link that points at its own parent is what
/// [`MAX_DEPTH`] ends.
pub(super) fn copy_into(
    from: &Path,
    into: &Path,
    copy: &Path,
    depth: u32,
    budget: &mut Budget,
) -> Result<(), AxError> {
    if depth > MAX_DEPTH {
        return Err(AxError::failure(
            AxCode::SandboxDenied,
            "copy the working tree",
            format!("{} nests deeper than {MAX_DEPTH}", from.display()),
        )
        .with_recovery(
            "a directory that links to one of its own ancestors has no bottom; \
             run the command with `where: host` if it is meant",
        ));
    }
    let entries =
        std::fs::read_dir(from).map_err(|err| copy_fault(&from.display().to_string(), &err))?;
    for entry in entries {
        let entry = entry.map_err(|err| copy_fault(&from.display().to_string(), &err))?;
        let source = entry.path();
        if source == copy {
            continue;
        }
        let target = into.join(entry.file_name());
        let metadata = std::fs::metadata(&source)
            .map_err(|err| copy_fault(&source.display().to_string(), &err))?;
        if metadata.is_dir() {
            std::fs::create_dir(&target)
                .map_err(|err| copy_fault(&target.display().to_string(), &err))?;
            copy_into(&source, &target, copy, depth.saturating_add(1), budget)?;
        } else if metadata.is_file() {
            budget.take(metadata.len())?;
            std::fs::copy(&source, &target)
                .map_err(|err| copy_fault(&source.display().to_string(), &err))?;
        } else {
            return Err(AxError::failure(
                AxCode::SandboxDenied,
                "copy the working tree",
                format!("{} is not a file or a directory", source.display()),
            )
            .with_recovery(
                "the sandbox carries the working directory; a device, a socket or a \
                 broken link in it has no copy to carry",
            ));
        }
    }
    Ok(())
}
