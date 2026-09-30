// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The reading end of a harness session (agent_protocols-SPEC.md 8-19).
//!
//! A read on a pipe has no deadline, and a harness running a long command
//! can go minutes without a line, while a halt must not wait for it to
//! speak. So one thread per session reads the harness's output and hands
//! each message over a channel the session waits on for a bounded time.
//! The channel holds no queue: the reader waits until the session takes
//! the line, so the child's own pipe is the only buffer. The thread ends
//! when the harness closes its output, which killing the child does, or
//! when the session drops the channel.

use std::io::BufRead;
use std::sync::mpsc::{Receiver, RecvTimeoutError};
use std::time::Duration;

use kernel::{AxCode, AxError};

use crate::mcp::{Received, read_one_message};

/// What the harness said, as the session waits for it.
pub struct Lines {
    heard: Receiver<Result<Received, AxError>>,
}

/// What one bounded wait found.
pub(crate) enum Heard {
    Message(String),
    /// The harness closed its output, or its reader is gone.
    Ended,
    /// Nothing within the wait.
    Silent,
}

impl Lines {
    /// Starts the reader over `reader`, which a harness named `name`
    /// writes into.
    ///
    /// # Errors
    /// `E_TOOL_UNAVAILABLE` when the machine refuses the thread.
    pub fn over<R: BufRead + Send + 'static>(mut reader: R, name: &str) -> Result<Lines, AxError> {
        let (sender, heard) = std::sync::mpsc::sync_channel(0);
        let named = name.to_owned();
        std::thread::Builder::new()
            .name(format!("acp-{name}"))
            .spawn(move || {
                loop {
                    let read = read_one_message(&mut reader, &named);
                    // A refusal or the end of input is the last thing
                    // this reader has to say: the bytes after an
                    // oversized message cannot be told apart from the
                    // next one (agent_protocols-SPEC.md 8-15).
                    let last = !matches!(read, Ok(Received::Message(_)));
                    if sender.send(read).is_err() || last {
                        break;
                    }
                }
            })
            .map_err(|err| {
                AxError::failure(
                    AxCode::ToolUnavailable,
                    "read a harness",
                    format!("{name}: {err}"),
                )
                .with_recovery(
                    "the machine refused a thread to read this harness; close some programs and \
                     dispatch again",
                )
            })?;
        Ok(Lines { heard })
    }

    /// Waits at most `wait` for the next message.
    ///
    /// # Errors
    /// Propagates the reader's refusal of an oversized or unreadable
    /// message.
    pub(crate) fn next(&self, wait: Duration) -> Result<Heard, AxError> {
        match self.heard.recv_timeout(wait) {
            Ok(Ok(Received::Message(line))) => Ok(Heard::Message(line)),
            Ok(Ok(Received::EndOfInput)) | Err(RecvTimeoutError::Disconnected) => Ok(Heard::Ended),
            Ok(Err(refused)) => Err(refused),
            Err(RecvTimeoutError::Timeout) => Ok(Heard::Silent),
        }
    }
}
