// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! What one member of the backlog is made of, and where a child's
//! output goes while nobody is reading it.

use super::process::Process;
use std::path::PathBuf;

use kernel::{Address, AxCode, AxError, RunId};

use super::BacklogKind;
use super::ceiling::Mark;
use super::tail::Tail;

pub(super) struct Member {
    pub(super) scope: Address,
    pub(super) what: String,
    pub(super) body: Body,
}

/// What a member is made of: a child process this table can kill, or
/// a run this table can only ask to stop.
pub(super) enum Body {
    Command {
        child: Process,
        dir: PathBuf,
        claim: Claim,
        tail: Tail,
        /// How the run's memory ceiling held as the command entered
        /// (`crates/runtime/spec/Tools/Exec.lean` D95).
        ceiling: Mark,
    },
    Run(RunState),
}

/// Who a command's ending is handed to.
///
/// The table is one per city, so the run that started a command is the
/// only fact that keeps its output out of every other run's tool result.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Claim {
    /// Inside its short window: the call polling it takes the ending,
    /// and [`super::Backlog::harvest`] leaves it alone.
    Window(RunId),
    /// Past its window: only this run's harvest takes the ending.
    Run(RunId),
    /// Its run has ended: the ending is read so the process handle and
    /// the output files are let go, and it is handed to nobody.
    Nobody,
}

/// Whether a halt has reached a run member yet.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum RunState {
    Going,
    Stopping,
}

impl Body {
    pub(super) fn kind(&self) -> BacklogKind {
        match self {
            Body::Command { .. } => BacklogKind::Command,
            Body::Run(_) => BacklogKind::Run,
        }
    }
}

/// Reads what a child wrote and takes the place it wrote to away.
///
/// A stream that cannot be read is answered with a sentence in its place
/// rather than with an error: the command's exit code is the fact the
/// caller is owed, and a temporary file that vanished must not turn a run
/// that finished into a run that failed, nor read as a command that
/// printed nothing (`crates/runtime/spec/Tools/Exec.lean` D95).
/// A directory that will not go costs disk, never the result.
pub(super) fn collect(dir: &std::path::Path) -> (String, String) {
    let read = |name: &str, stream: &str| match std::fs::read(dir.join(name)) {
        Ok(bytes) => String::from_utf8_lossy(&bytes).into_owned(),
        Err(err) => format!("[this command's {stream} could not be read back: {err}]"),
    };
    let out = read("out", "stdout");
    let err = read("err", "stderr");
    drop(std::fs::remove_dir_all(dir));
    (out, err)
}

pub(super) fn storage(dir: &std::path::Path, err: &std::io::Error) -> AxError {
    AxError::failure(
        AxCode::StorageFatal,
        "keep a command's output",
        format!("{}: {err}", dir.display()),
    )
    .with_recovery("make the system temporary directory writable")
}

#[cfg(test)]
#[allow(clippy::unwrap_used, reason = "test code")]
mod tests {
    /// A stream whose file is gone says so instead of reading as empty.
    #[test]
    fn a_stream_that_cannot_be_read_says_so() {
        let dir = tempfile::tempdir().unwrap();
        let kept = dir.path().join("member");
        std::fs::create_dir_all(&kept).unwrap();
        std::fs::write(kept.join("out"), "built").unwrap();
        let (out, err) = super::collect(&kept);
        assert_eq!(out, "built");
        assert!(
            err.starts_with("[this command's stderr could not be read back: "),
            "{err}"
        );
        assert!(!kept.exists());
    }
}
