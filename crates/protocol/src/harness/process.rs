// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! One official harness started as a child that speaks ACP on its pipes
//! (agent_protocols-SPEC.md 8-19).
//!
//! The process is reclaimed when its handle is dropped, so whoever holds
//! the handle is the one list of what to kill. Its stderr is discarded
//! for the reason `mcp::stdio` gives: those are its diagnostics, not
//! answers.

use std::io::BufReader;
use std::path::Path;
use std::process::{Child, ChildStdin, Command, Stdio};

use kernel::{AxCode, AxError};

use super::{AcpSession, Harness, Lines};

/// A running harness. Dropping it kills the child and waits for it.
pub struct HarnessProcess {
    child: Child,
}

impl HarnessProcess {
    /// Starts `harness` in `cwd` and opens a session with it there.
    ///
    /// # Errors
    /// `E_TOOL_UNAVAILABLE` when the program will not start, with the
    /// harness's own sign-in and install page in the recovery; and every
    /// refusal of [`AcpSession::open`]. The child is reclaimed on each of
    /// them.
    pub fn start(
        harness: Harness,
        cwd: &Path,
    ) -> Result<(HarnessProcess, AcpSession<ChildStdin>), AxError> {
        let launch = harness.launch();
        let name = harness.as_str();
        let child = Command::new(launch.program.name())
            .args(launch.args)
            .current_dir(cwd)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|err| unavailable(harness, &format!("{}: {err}", launch.program.name())))?;
        // From here on the handle owns the child, so every early return
        // below reclaims it.
        let mut process = HarnessProcess { child };
        let writer = process
            .child
            .stdin
            .take()
            .ok_or_else(|| unavailable(harness, "the child was started without pipes"))?;
        let output = process
            .child
            .stdout
            .take()
            .ok_or_else(|| unavailable(harness, "the child was started without pipes"))?;
        let lines = Lines::over(BufReader::new(output), name)?;
        let session = AcpSession::open(lines, writer, name, cwd)?;
        Ok((process, session))
    }
}

impl Drop for HarnessProcess {
    fn drop(&mut self) {
        if self.child.kill().is_ok() {
            match self.child.wait() {
                Ok(_status) => {}
                // A child that cannot be waited for was reaped by the
                // platform; either way it is not running.
                Err(_unwaitable) => {}
            }
        }
    }
}

fn unavailable(harness: Harness, subject: &str) -> AxError {
    AxError::failure(
        AxCode::ToolUnavailable,
        format!("start {}", harness.as_str()),
        subject.to_owned(),
    )
    .with_recovery(format!(
        "install the harness and sign in inside it as {} says, then dispatch again",
        harness.docs()
    ))
}
