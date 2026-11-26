// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::io::{BufRead, BufReader, Write};
use std::process::{Child, ChildStdin, ChildStdout, Command};
use std::time::Duration;

use super::{failure, frame};
use crate::error::Result;

pub(super) struct Process {
    pub(super) child: Child,
    input: Option<ChildStdin>,
    output: BufReader<ChildStdout>,
}

pub(super) enum Interpreter {
    Ready(Command),
    Running(Process),
    Closed,
}

impl Interpreter {
    pub(super) fn close_within(&mut self, grace: Duration) -> Result<()> {
        if let Interpreter::Running(process) = self {
            drop(process.input.take());
            let status = crate::child_exit::wait(&mut process.child, grace).map_err(|err| {
                failure(
                    "close and wait for Python",
                    err.to_string(),
                    "if the interpreter does not exit, kill it and wait again",
                )
            })?;
            if status.is_none() {
                self.terminate()?;
            }
        }
        *self = Self::Closed;
        Ok(())
    }

    pub(super) fn terminate(&mut self) -> Result<()> {
        use crate::child_exit::TerminateError;

        let Interpreter::Running(process) = self else {
            *self = Self::Closed;
            return Ok(());
        };
        crate::child_exit::terminate(&mut process.child).map_err(|error| {
            let (action, err) = match error {
                TerminateError::Kill(err) => ("kill Python", err),
                TerminateError::Wait(err) => ("wait for killed Python", err),
            };
            failure(
                action,
                err.to_string(),
                "reap the process before starting another Mei run",
            )
        })?;
        *self = Self::Closed;
        Ok(())
    }

    pub(super) fn start(&mut self) -> Result<&mut Process> {
        if let Self::Ready(command) = self {
            *self = Self::Running(Process::spawn(command)?);
        }
        match self {
            Self::Running(process) => Ok(process),
            Self::Ready(_) | Self::Closed => Err(failure(
                "start the Python tool",
                "the interpreter has been closed",
                "start a new Mei session before running another Python call",
            )),
        }
    }
}

impl Process {
    fn spawn(command: &mut Command) -> Result<Self> {
        let mut child = command.spawn().map_err(|err| {
            failure(
                "start Python for Mei",
                err.to_string(),
                "install Python 3 on PATH as python3 or python",
            )
        })?;
        let input = child.stdin.take().ok_or_else(|| {
            failure(
                "open the Python control input",
                "stdin was not piped",
                "keep the Python control channel piped",
            )
        })?;
        let output = child.stdout.take().ok_or_else(|| {
            failure(
                "open the Python control output",
                "stdout was not piped",
                "keep the Python control channel piped",
            )
        })?;
        Ok(Self {
            child,
            input: Some(input),
            output: BufReader::new(output),
        })
    }

    pub(super) fn write_frame(&mut self, frame: &[u8]) -> Result<()> {
        let input = self.input.as_mut().ok_or_else(|| {
            failure(
                "write a Python control frame",
                "the control input is closed",
                "start a new Mei process instead of reusing a closed interpreter",
            )
        })?;
        input
            .write_all(frame)
            .and_then(|()| input.flush())
            .map_err(|err| {
                failure(
                    "write a Python control frame",
                    err.to_string(),
                    "the interpreter or private input channel died; start a new Mei process",
                )
            })
    }

    pub(super) fn read_frame(&mut self) -> Result<frame::Frame> {
        let available = self.output.fill_buf().map_err(|err| {
            failure(
                "read a Python response frame",
                err.to_string(),
                "keep the private output channel connected to the embedded interpreter",
            )
        })?;
        if available.is_empty() {
            let status = self.child.try_wait().map_err(|err| {
                failure(
                    "reap Python after a closed channel",
                    err.to_string(),
                    "inspect the interpreter failure before starting a new process",
                )
            })?;
            return Err(failure(
                "read a Python response frame",
                format!("the private channel closed; interpreter status: {status:?}"),
                "the interpreter or private channel died; start a new Mei process",
            ));
        }
        frame::read(&mut self.output)
    }
}
