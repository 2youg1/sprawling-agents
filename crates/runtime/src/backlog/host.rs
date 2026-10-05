// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Host command ownership at the Backlog spawn boundary.

use super::{Backlog, Body, Claim, Member, Started, Tail, collect, process, storage};
use kernel::{Address, AxCode, AxError, RunId};
use std::process::{Command, Stdio};

impl Backlog {
    /// Starts a command, waits out the short window, and hands back
    /// either its result or a handle onto it.
    ///
    /// The command's program, arguments, working directory and
    /// environment are the caller's; this module owns only where the
    /// output goes and how the waiting is done. A command that outlives
    /// its window is owed to `owner` and to no other run.
    ///
    /// # Errors
    /// `E_TOOL_UNAVAILABLE` when the program cannot be started, and
    /// `E_STORAGE_FATAL` when the place its output goes cannot be
    /// written.
    pub fn run(
        &self,
        owner: RunId,
        scope: &Address,
        what: String,
        mut command: Command,
    ) -> Result<Started, AxError> {
        let id = self.mint()?;
        let dir = self.scratch.dir(id);
        std::fs::create_dir_all(&dir).map_err(|err| storage(&dir, &err))?;
        let out = std::fs::File::create(dir.join("out")).map_err(|err| storage(&dir, &err))?;
        let errs = std::fs::File::create(dir.join("err")).map_err(|err| storage(&dir, &err))?;
        command
            .stdin(Stdio::null())
            .stdout(Stdio::from(out))
            .stderr(Stdio::from(errs));
        let child = command.spawn().map_err(|err| {
            AxError::failure(
                AxCode::ToolUnavailable,
                "start a command",
                format!("{what}: {err}"),
            )
            .with_recovery("check the program name, or use the shell arm")
        })?;
        self.enrol(
            id,
            Member {
                scope: scope.clone(),
                what: what.clone(),
                body: Body::Command {
                    child: process::Process::Host(child),
                    dir: dir.clone(),
                    claim: Claim::Window(owner),
                    tail: Tail::default(),
                },
            },
        )?;
        let mut tail = Tail::default();
        for _ in 0..self.window.polls() {
            if let Some(exit) = self.settle(id)? {
                let (stdout, stderr) = collect(&dir);
                return Ok(Started::Settled {
                    exit,
                    stdout,
                    stderr,
                });
            }
            if let Some(sink) = &self.sink {
                sink.deliver(tail.take(&dir, (owner, id), self.window.read_per_poll()));
            }
            std::thread::sleep(self.window.interval());
        }
        self.hand_over(id, tail)?;
        Ok(Started::Backgrounded { id, what })
    }
}
