// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::io::{BufRead, ErrorKind, Read};
use std::sync::mpsc::{Receiver, RecvTimeoutError};
use std::time::Duration;

use kernel::{AxCode, AxError};

pub const MESSAGE_CEILING: usize = 8_388_608;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Received {
    Message(String),

    EndOfInput,
}

pub struct Lines {
    heard: Receiver<Result<Received, AxError>>,
}

pub(crate) enum Heard {
    Message(String),

    Ended,

    Silent,
}

impl Lines {
    pub fn over<R: BufRead + Send + 'static>(mut reader: R, name: &str) -> Result<Lines, AxError> {
        let (sender, heard) = std::sync::mpsc::sync_channel(0);
        let named = name.to_owned();
        std::thread::Builder::new()
            .name(format!("read-{name}"))
            .spawn(move || {
                loop {
                    let read = read_one_message(&mut reader, &named);

                    let last = !matches!(read, Ok(Received::Message(_)));
                    if sender.send(read).is_err() || last {
                        break;
                    }
                }
            })
            .map_err(|err| {
                AxError::failure(
                    AxCode::ToolUnavailable,
                    "start a reader",
                    format!("{name}: {err}"),
                )
                .with_recovery(format!(
                    "the machine refused a thread to read {name}; close some programs and \
                     dispatch again"
                ))
            })?;
        Ok(Lines { heard })
    }

    pub(crate) fn next(&self, wait: Duration) -> Result<Heard, AxError> {
        match self.heard.recv_timeout(wait) {
            Ok(Ok(Received::Message(line))) => Ok(Heard::Message(line)),
            Ok(Ok(Received::EndOfInput)) | Err(RecvTimeoutError::Disconnected) => Ok(Heard::Ended),
            Ok(Err(refused)) => Err(refused),
            Err(RecvTimeoutError::Timeout) => Ok(Heard::Silent),
        }
    }
}

pub fn read_one_message(source: &mut dyn BufRead, server: &str) -> Result<Received, AxError> {
    let mut held: Vec<u8> = Vec::new();
    loop {
        let (consumed, framed) = next_chunk(source, server, &mut held)?;
        source.consume(consumed);
        if held.len() > MESSAGE_CEILING {
            return Err(over_ceiling(server, held.len()));
        }
        match framed {
            Framing::Complete => return decode(held, server).map(Received::Message),
            Framing::EndOfInput => return ended(held, server),
            Framing::More => {}
        }
    }
}

pub(crate) fn read_whole_message(source: &mut dyn Read, server: &str) -> Result<String, AxError> {
    let mut held: Vec<u8> = Vec::new();
    let mut chunk = [0_u8; WHOLE_CHUNK_BYTES];
    loop {
        let read = match source.read(&mut chunk) {
            Ok(read) => read,
            Err(err) if err.kind() == ErrorKind::Interrupted => continue,
            Err(err) => return Err(unreadable(server, &err)),
        };
        if read == 0 {
            return decode(held, server);
        }
        held.extend_from_slice(chunk.get(..read).ok_or_else(|| unframed(server))?);
        if held.len() > MESSAGE_CEILING {
            return Err(over_ceiling(server, held.len()));
        }
    }
}

const WHOLE_CHUNK_BYTES: usize = 8_192;

pub(super) fn answer_unread(refused: &AxError) -> AxError {
    AxError::failure(*refused.code(), refused.action(), refused.subject())
        .effect_unknown()
        .with_recovery(refused.recovery())
}

enum Framing {
    Complete,

    EndOfInput,

    More,
}

fn next_chunk(
    source: &mut dyn BufRead,
    server: &str,
    held: &mut Vec<u8>,
) -> Result<(usize, Framing), AxError> {
    let chunk = source.fill_buf().map_err(|err| unreadable(server, &err))?;
    if chunk.is_empty() {
        return Ok((0, Framing::EndOfInput));
    }
    match chunk.iter().position(|byte| *byte == b'\n') {
        Some(at) => {
            let upto = chunk.get(..at).ok_or_else(|| unframed(server))?;
            held.extend_from_slice(upto);
            let consumed = at.checked_add(1).ok_or_else(|| unframed(server))?;
            Ok((consumed, Framing::Complete))
        }
        None => {
            held.extend_from_slice(chunk);
            Ok((chunk.len(), Framing::More))
        }
    }
}

fn ended(held: Vec<u8>, server: &str) -> Result<Received, AxError> {
    if held.is_empty() {
        return Ok(Received::EndOfInput);
    }
    Err(AxError::failure(
        AxCode::WireMismatch,
        "read an mcp message",
        format!(
            "{server}: the output ended after {} bytes with no newline",
            held.len()
        ),
    )
    .with_recovery(
        "the server stopped mid-message; check its own logs, \
         then start the run again once it ends every message with a newline",
    ))
}

fn decode(mut held: Vec<u8>, server: &str) -> Result<String, AxError> {
    if held.last() == Some(&b'\r') {
        held.truncate(held.len().saturating_sub(1));
    }
    String::from_utf8(held).map_err(|err| {
        AxError::failure(
            AxCode::WireMismatch,
            "read an mcp message",
            format!("{server}: the message is not UTF-8 ({err})"),
        )
        .with_recovery("MCP messages are UTF-8 JSON; this server sent other bytes")
    })
}

fn over_ceiling(server: &str, held: usize) -> AxError {
    AxError::failure(
        AxCode::WireMismatch,
        "read an mcp message",
        format!("{server}: one message passed {MESSAGE_CEILING} bytes (read {held} so far)"),
    )
    .with_recovery(
        "the connection was dropped rather than a fragment parsed; \
         have this server answer in smaller messages, \
         or raise agent_protocols::mcp::MESSAGE_CEILING and its row in Tools/xtask/budgets.toml",
    )
}

fn unreadable(server: &str, err: &std::io::Error) -> AxError {
    AxError::failure(
        AxCode::ToolUnavailable,
        "read an mcp message",
        format!("{server}: cannot read the server's output: {err}"),
    )
    .with_recovery("the server is no longer reachable; this run continues without it")
}

fn unframed(server: &str) -> AxError {
    AxError::failure(
        AxCode::WireMismatch,
        "read an mcp message",
        format!("{server}: the transport reported a frame it did not hold"),
    )
    .with_recovery("restart this city; the server is started again with it")
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    reason = "test code"
)]
mod tests {
    use super::*;

    fn read(input: &[u8]) -> Result<Received, AxError> {
        let mut source = std::io::BufReader::new(input);
        read_one_message(&mut source, "apps")
    }

    #[test]
    fn a_message_at_the_ceiling_is_read() {
        let envelope = "{\"jsonrpc\":\"2.0\",\"m\":\"\"}";
        let filling = "x".repeat(MESSAGE_CEILING - envelope.len());
        let line = format!("{{\"jsonrpc\":\"2.0\",\"m\":\"{filling}\"}}");
        assert_eq!(line.len(), MESSAGE_CEILING);
        let mut input = line.clone().into_bytes();
        input.push(b'\n');

        assert_eq!(read(&input).unwrap(), Received::Message(line));
    }

    #[test]
    fn a_message_above_the_ceiling_is_refused_with_the_ceiling_named() {
        let mut input = vec![b'x'; MESSAGE_CEILING + 1];
        input.push(b'\n');

        let err = read(&input).expect_err("a message above the ceiling is not an answer");

        assert_eq!(err.code(), &AxCode::WireMismatch);
        assert!(err.subject().contains(&MESSAGE_CEILING.to_string()));
        assert!(err.subject().contains("apps"));
        assert!(err.recovery().contains("MESSAGE_CEILING"));
    }

    #[test]
    fn an_endless_message_is_refused_before_it_ends() {
        struct Endless;
        impl std::io::Read for Endless {
            fn read(&mut self, into: &mut [u8]) -> std::io::Result<usize> {
                into.fill(b'x');
                Ok(into.len())
            }
        }

        let mut source = std::io::BufReader::new(Endless);
        let err = read_one_message(&mut source, "apps").expect_err("a stream with no end");

        assert_eq!(err.code(), &AxCode::WireMismatch);
    }

    #[test]
    fn a_closed_output_between_messages_is_the_end_rather_than_a_refusal() {
        assert_eq!(read(b"").unwrap(), Received::EndOfInput);
    }

    #[test]
    fn output_that_ends_mid_message_is_refused_as_a_fragment() {
        let err = read(b"{\"jsonrpc\"").expect_err("a fragment is not a message");

        assert_eq!(err.code(), &AxCode::WireMismatch);
        assert!(err.recovery().contains("newline"));
    }

    #[test]
    fn two_messages_are_read_one_at_a_time() {
        let mut source = std::io::BufReader::new(&b"{\"id\":1}\n{\"id\":2}\n"[..]);

        assert_eq!(
            read_one_message(&mut source, "apps").unwrap(),
            Received::Message("{\"id\":1}".to_owned())
        );
        assert_eq!(
            read_one_message(&mut source, "apps").unwrap(),
            Received::Message("{\"id\":2}".to_owned())
        );
        assert_eq!(
            read_one_message(&mut source, "apps").unwrap(),
            Received::EndOfInput
        );
    }
}
