// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The copy a command runs in: one working tree, brought to what the
//! working directory holds before each command, and bounded.
//!
//! A copy is synced rather than made again, because creating files is
//! what a real-time scanner waits on, and a fresh copy per command would
//! create every file of the tree per command (runtime D10). The
//! sync leaves the copy file for file equal to the working directory, so
//! nothing an earlier command wrote reaches a later one.
//!
//! The bound is a refusal rather than a partial copy. Half a tree would
//! answer for files the copy never carried, and a command that then
//! reads a stale or missing file has no way to tell that apart from a
//! file somebody changed.

use std::collections::BTreeMap;
use std::ffi::OsString;
use std::fs::Metadata;
use std::io::Read;
use std::path::{Path, PathBuf};

use kernel::{AxCode, AxError};
use storage::FileWork;

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

/// How many bytes of a file and its copy are compared at a time.
const CHUNK: usize = 64 * 1024;

/// One sync of one copy: what the working directory held against the
/// bound, what the sync cost the filesystem, and the two buffers every
/// comparison of the walk reuses.
pub(super) struct Budget {
    files: u64,
    bytes: u64,
    work: FileWork,
    ours: Vec<u8>,
    theirs: Vec<u8>,
}

impl Budget {
    pub(super) fn new() -> Budget {
        Budget {
            files: 0,
            bytes: 0,
            work: FileWork::default(),
            ours: vec![0; CHUNK],
            theirs: vec![0; CHUNK],
        }
    }

    /// What bringing the copy to the working directory cost.
    pub(super) fn work(&self) -> FileWork {
        self.work
    }

    fn take(&mut self, bytes: u64) -> Result<(), AxError> {
        self.files = self.files.saturating_add(1);
        self.bytes = self.bytes.saturating_add(bytes);
        if self.files > MAX_FILES || self.bytes > MAX_BYTES {
            return Err(over_budget(self.files, self.bytes));
        }
        Ok(())
    }
}

/// Adds one to a count of the work.
fn one_more(count: &mut u64) {
    *count = count.saturating_add(1);
}

/// The three directories one sync walk is made of: where it reads,
/// where it writes, and the copy's own root, which it skips.
pub(super) struct Stage<'a> {
    pub(super) from: &'a Path,
    pub(super) into: &'a Path,
    pub(super) copy: &'a Path,
}

/// Brings the existing directory `stage.into` to what `stage.from` holds.
///
/// `stage.copy` is this copy's own directory, and it is skipped: a
/// working directory that contains the place copies go - this machine's
/// scratch root is one - would otherwise be copied into itself. A link
/// in the working directory is followed, so a link to a directory outside
/// it carries that directory's content rather than a link into the
/// person's tree; a link that points at its own parent is what
/// [`MAX_DEPTH`] ends. What the copy holds is read without following
/// links, because a command wrote it: an entry of another kind than the
/// working directory's, or a file whose bytes differ, is removed and
/// made again rather than written over, since a command can leave a
/// name that leads out of the copy.
pub(super) fn mirror(stage: &Stage<'_>, depth: u32, budget: &mut Budget) -> Result<(), AxError> {
    let Stage { from, into, copy } = *stage;
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
    let mut stale = held(into, budget)?;
    let entries =
        std::fs::read_dir(from).map_err(|err| copy_fault(&from.display().to_string(), &err))?;
    for entry in entries {
        let entry = entry.map_err(|err| copy_fault(&from.display().to_string(), &err))?;
        let source = entry.path();
        if source == copy {
            continue;
        }
        one_more(&mut budget.work.walked);
        let target = into.join(entry.file_name());
        let found = stale.remove(&entry.file_name());
        let metadata = std::fs::metadata(&source)
            .map_err(|err| copy_fault(&source.display().to_string(), &err))?;
        if metadata.is_dir() {
            match found {
                Some(kept) if kept.is_dir() => {}
                Some(kept) => {
                    discard(&target, &kept)?;
                    one_more(&mut budget.work.removed);
                    make_dir(&target)?;
                }
                None => make_dir(&target)?,
            }
            mirror(
                &Stage {
                    from: &source,
                    into: &target,
                    copy,
                },
                depth.saturating_add(1),
                budget,
            )?;
        } else if metadata.is_file() {
            budget.take(metadata.len())?;
            match found {
                Some(kept) if kept.is_file() && same_bytes(&source, &target, &kept, budget)? => {}
                Some(kept) => {
                    discard(&target, &kept)?;
                    copy_file(&source, &target)?;
                    one_more(&mut budget.work.rewritten);
                }
                None => {
                    copy_file(&source, &target)?;
                    one_more(&mut budget.work.created);
                }
            }
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
    for (name, kept) in stale {
        discard(&into.join(name), &kept)?;
        one_more(&mut budget.work.removed);
    }
    Ok(())
}

/// What the copy's directory `into` holds, by name, read without
/// following a link.
fn held(into: &Path, budget: &mut Budget) -> Result<BTreeMap<OsString, Metadata>, AxError> {
    let fault = |err: std::io::Error| copy_fault(&into.display().to_string(), &err);
    let mut found = BTreeMap::new();
    for entry in std::fs::read_dir(into).map_err(fault)? {
        let entry = entry.map_err(fault)?;
        one_more(&mut budget.work.walked);
        found.insert(entry.file_name(), entry.metadata().map_err(fault)?);
    }
    Ok(found)
}

/// Whether the copy's regular file `target` holds what `source` holds.
/// Lengths are compared first, so only files of one length are read.
fn same_bytes(
    source: &Path,
    target: &Path,
    kept: &Metadata,
    budget: &mut Budget,
) -> Result<bool, AxError> {
    let length = std::fs::metadata(source)
        .map_err(|err| copy_fault(&source.display().to_string(), &err))?
        .len();
    if length != kept.len() {
        return Ok(false);
    }
    let open = |path: &Path| {
        std::fs::File::open(path).map_err(|err| copy_fault(&path.display().to_string(), &err))
    };
    let (mut ours, mut theirs) = (open(source)?, open(target)?);
    let mut left = length;
    while left > 0 {
        let step = usize::try_from(left).map_or(CHUNK, |left| left.min(CHUNK));
        let (Some(a), Some(b)) = (budget.ours.get_mut(..step), budget.theirs.get_mut(..step))
        else {
            return Ok(false);
        };
        // A file that shrank while it was read differs from the one
        // the lengths were compared on.
        if ours.read_exact(a).is_err() || theirs.read_exact(b).is_err() || a != b {
            return Ok(false);
        }
        left = left.saturating_sub(u64::try_from(step).unwrap_or(u64::MAX));
    }
    Ok(true)
}

/// Removes one entry of the copy: a directory with what it holds, and
/// anything else - a file, or a link of either kind - as the name alone.
fn discard(target: &Path, kept: &Metadata) -> Result<(), AxError> {
    let gone = if kept.is_dir() {
        std::fs::remove_dir_all(target)
    } else {
        // A link to a directory is removed as a directory on Windows.
        std::fs::remove_file(target).or_else(|first| match kept.is_symlink() {
            true => std::fs::remove_dir(target).map_err(|_| first),
            false => Err(first),
        })
    };
    gone.map_err(|err| copy_fault(&target.display().to_string(), &err))
}

fn make_dir(target: &Path) -> Result<(), AxError> {
    std::fs::create_dir(target).map_err(|err| copy_fault(&target.display().to_string(), &err))
}

fn copy_file(source: &Path, target: &Path) -> Result<(), AxError> {
    std::fs::copy(source, target)
        .map(drop)
        .map_err(|err| copy_fault(&source.display().to_string(), &err))
}
