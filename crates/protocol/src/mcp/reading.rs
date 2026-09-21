// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! How much of one message this city will read before it refuses.
//!
//! An MCP server is an outside party a configuration file pointed at,
//! and everything it writes is untrusted input. The stdio transport
//! frames on newlines, so a server that never writes one - broken,
//! wedged, or hostile - asks this process to grow a buffer without
//! bound. The process that reads it is also the city's only writer, so
//! exhausting its memory ends the history rather than one tool call.
//!
//! [`MESSAGE_CEILING`] is where that stops, and it is not a parameter:
//! a ceiling a caller may choose is a ceiling each transport chooses
//! differently, and the refusal below has to name the same number to
//! every server. A message above the ceiling is refused whole, with the
//! server, the ceiling and the way to change it in the refusal, because
//! parsing the first megabytes of a message whose end nobody has seen
//! is reading a fragment as if it were the answer.
//!
//! A refusal ends the connection rather than skipping one message. The
//! unread remainder of an oversized line is indistinguishable from the
//! next message, so a caller that read again would take the tail of a
//! refused message for a fresh one; the assembly layer reclaims the
//! child instead.

use std::io::BufRead;

use kernel::{AxCode, AxError};

/// The largest message this city reads from an MCP server, in bytes.
///
/// Derived from the largest payload the city already accepts rather
/// than picked: one picture is at most `kernel::consts_policy`'s
/// `IMAGE_MAX_BYTES` (2 MiB), which is 2,796,203 bytes once base64
/// encoding has grown it by four thirds, so 8 MiB leaves a tool result
/// room for such a picture, its text, and the JSON-RPC envelope around
/// both. What it refuses is a server streaming without end.
pub const MESSAGE_CEILING: usize = 8_388_608;

/// What one read of the transport found.
///
/// Two outcomes rather than an optional message, because "the server
/// closed its output" is a state on which the caller stops reading,
/// and an absent value would leave that decision to be re-derived at
/// every call site.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Received {
    /// One complete message, with its framing newline removed.
    Message(String),
    /// The server closed its output between messages, which is how a
    /// child process that has finished looks from this side.
    EndOfInput,
}

/// Reads one newline-framed message from `source`, refusing anything
/// above [`MESSAGE_CEILING`].
///
/// `server` is the name the refusal reports, so a city talking to four
/// servers says which one misbehaved.
///
/// # Errors
/// Refuses a message above the ceiling, a stream that ends in the
/// middle of a message, bytes that are not UTF-8, and any failure the
/// underlying reader reports. After any of these the caller must stop
/// reading this source: the bytes left in it belong to a message this
/// function has already refused.
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

/// Where the current chunk left the message.
enum Framing {
    /// The framing newline was in this chunk.
    Complete,
    /// The reader has no more bytes.
    EndOfInput,
    /// The chunk held message bytes and no newline.
    More,
}

/// Appends the next chunk's message bytes to `held` and reports how many
/// bytes of the reader they occupied, so the caller can consume exactly
/// those and release the borrow first.
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

/// A stream that ends between messages is a server that finished; one
/// that ends inside a message is a server that was cut off, and the
/// bytes it left are a fragment rather than an answer.
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

/// A message written with CRLF framing keeps no carriage return: the
/// newline frames the message and the byte before it is framing too,
/// so a Windows-hosted server and a Unix one produce the same message.
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
         or raise protocol::mcp::MESSAGE_CEILING and its row in xtask/budgets.toml",
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

/// The buffered reader handed back a chunk that does not contain the
/// position it just reported, which no implementation does; the refusal
/// exists so this function reaches its answer without indexing.
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

    /// One message of exactly the ceiling is an ordinary message: the
    /// limit is the largest message that passes, not the smallest that
    /// fails.
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

    /// The refusal arrives while the server is still writing, so the
    /// city never holds the whole of an unbounded message.
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
