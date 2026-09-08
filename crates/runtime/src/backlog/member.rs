// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! What one member of the backlog is made of, and where a child's
//! output goes while nobody is reading it.

use std::path::PathBuf;
use std::process::Child;

use kernel::{Address, AxCode, AxError};

use super::BacklogKind;

pub(super) struct Member {
    pub(super) scope: Address,
    pub(super) what: String,
    pub(super) body: Body,
}

/// What a member is made of: a child process this table can kill, or
/// a run this table can only ask to stop.
pub(super) enum Body {
    Command {
        child: Child,
        dir: PathBuf,
        /// Set once the short window has passed. Only a backgrounded
        /// member is collected by [`super::Backlog::harvest`]; one still inside
        /// its window belongs to the caller that is polling it.
        backgrounded: bool,
    },
    Run(RunState),
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
/// Failing to read is answered with what was read so far rather than
/// with an error: the command's exit code is the fact the caller is
/// owed, and a temporary file that vanished must not turn a run that
/// finished into a run that failed.
pub(super) fn collect(dir: &std::path::Path) -> (String, String) {
    let read = |name: &str| {
        std::fs::read(dir.join(name))
            .map(|bytes| String::from_utf8_lossy(&bytes).into_owned())
            .unwrap_or_default()
    };
    let out = read("out");
    let err = read("err");
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
