// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

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

pub(super) enum Body {
    Command {
        child: Process,
        dir: PathBuf,
        claim: Claim,
        tail: Tail,

        ceiling: Mark,
    },
    Run(RunState),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Claim {

    Window(RunId),

    Run(RunId),

    Nobody,
}

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
