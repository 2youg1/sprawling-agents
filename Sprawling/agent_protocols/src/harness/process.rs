// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! One consented ACP agent started as a child that speaks ACP on its pipes
//! (`crates/agent_protocols/Spec.lean` §8-19, D16, D17).
//!
//! The process is reclaimed when its handle is dropped, so whoever holds
//! the handle is the one list of what to kill. Its stderr is discarded
//! for the reason `mcp::stdio` gives: those are its diagnostics, not
//! answers.

use std::io::BufReader;
use std::path::Path;
use std::process::{Child, ChildStdin, Stdio};

use kernel::{AxCode, AxError};

use super::environment::passed;
use super::{AcpSession, Consented};
use crate::mcp::Lines;

/// A running agent. Dropping it kills the child and waits for it.
pub struct HarnessProcess {
    child: Child,
}

impl HarnessProcess {
    /// Starts the consented agent in `cwd` and opens a session with it
    /// there. The child sees only the whitelisted environment and the
    /// entry's own `env`.
    ///
    /// # Errors
    /// `E_CONFIG_INVALID` for an `env` value that is a vault reference,
    /// which this release does not redeem for an agent;
    /// `E_TOOL_UNAVAILABLE` when the program will not start, with the
    /// command line in the recovery; and every refusal of
    /// [`AcpSession::open`]. The child is reclaimed on each of them.
    pub fn start(
        agent: &Consented,
        cwd: &Path,
    ) -> Result<(HarnessProcess, AcpSession<ChildStdin>), AxError> {
        let entry = agent.entry();
        let launch = &entry.launch;
        let name = entry.id.as_str();
        if let Some((key, _)) = launch
            .env
            .iter()
            .find(|(_, value)| kernel::SecretRef::parse(value).is_ok())
        {
            return Err(AxError::failure(
                AxCode::ConfigInvalid,
                format!("start {name}"),
                format!("{name}: `{key}` is a vault reference"),
            )
            .with_recovery(
                "write the value the agent needs, or sign in inside the agent; a vault reference \
                 beside an agent is not redeemed yet",
            ));
        }
        let child = child::command(&launch.program)
            .args(&launch.args)
            .env_clear()
            .envs(passed(std::env::vars_os()))
            .envs(launch.env.iter().map(|(key, value)| (key, value)))
            .current_dir(cwd)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|err| unavailable(agent, &format!("{}: {err}", launch.program)))?;
        // From here on the handle owns the child, so every early return
        // below reclaims it.
        let mut process = HarnessProcess { child };
        let writer = process
            .child
            .stdin
            .take()
            .ok_or_else(|| unavailable(agent, "the child was started without pipes"))?;
        let output = process
            .child
            .stdout
            .take()
            .ok_or_else(|| unavailable(agent, "the child was started without pipes"))?;
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

fn unavailable(agent: &Consented, subject: &str) -> AxError {
    let entry = agent.entry();
    AxError::failure(
        AxCode::ToolUnavailable,
        format!("start {}", entry.id.as_str()),
        subject.to_owned(),
    )
    .with_recovery(format!(
        "install the agent so that `{}` runs in a terminal, then dispatch again",
        entry.launch.preview()
    ))
}
