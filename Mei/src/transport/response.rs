// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use serde_json::Value;

use crate::error::{Error, Result, protocol};

struct Head {
    status: u16,
    body: usize,
    end: Option<usize>,
}

#[derive(Default)]
pub(super) struct Response {
    bytes: Vec<u8>,
    head: Option<Head>,
}

impl Response {
    pub(super) fn extend(&mut self, bytes: &[u8]) -> Result<bool> {
        let search = self.bytes.len().saturating_sub(3);
        self.bytes.extend_from_slice(bytes);
        if self.head.is_none() {
            let boundary = self
                .bytes
                .get(search..)
                .and_then(|tail| tail.windows(4).position(|w| w == b"\r\n\r\n"));
            if let Some(boundary) = boundary {
                let end = search.saturating_add(boundary).saturating_add(4);
                if let Some(header) = self.bytes.get(..end) {
                    self.head = Some(parse_head(header)?);
                }
            }
        }
        let Some(end) = self.head.as_ref().and_then(|head| head.end) else {
            return Ok(false);
        };
        if self.bytes.len() > end {
            return Err(invalid("response extends past Content-Length"));
        }
        Ok(self.bytes.len() == end)
    }

    pub(super) fn finish(self) -> Result<(Value, bool)> {
        let head = self.head.ok_or_else(|| {
            protocol(
                "parse the HTTP header/body boundary",
                format!(
                    "response has no CRLF header/body boundary; first bytes: {}",
                    String::from_utf8_lossy(self.bytes.get(..200).unwrap_or(&self.bytes))
                ),
                "return a complete HTTP response ending headers with CRLF CRLF",
            )
        })?;
        if head.end.is_some_and(|end| self.bytes.len() != end) {
            return Err(invalid(
                "connection closed before Content-Length bytes arrived",
            ));
        }
        let body = self.bytes.get(head.body..).unwrap_or_default();
        if head.status != 200 {
            return Err(Error::Body(String::from_utf8_lossy(body).into_owned()));
        }
        let value = serde_json::from_slice(body).map_err(|err| {
            protocol(
                "parse the Messages JSON body",
                format!(
                    "{err}; first bytes: {}",
                    String::from_utf8_lossy(body.get(..200).unwrap_or(body))
                ),
                "return one complete JSON object as the HTTP body",
            )
        })?;
        Ok((value, head.end.is_some()))
    }
}

fn invalid(subject: &str) -> Error {
    protocol("parse the HTTP response framing", subject, "return a single decimal Content-Length for a keep-alive response, or close the connection at the end of the body").into()
}

fn parse_head(bytes: &[u8]) -> Result<Head> {
    let mut local = [httparse::EMPTY_HEADER; 24];
    let mut parsed = httparse::Response::new(&mut local);
    let mut large;
    match parsed.parse(bytes) {
        Ok(httparse::Status::Complete(_)) => {}
        Err(httparse::Error::TooManyHeaders) => {
            large =
                vec![httparse::EMPTY_HEADER; bytes.iter().filter(|byte| **byte == b'\n').count()];
            parsed = httparse::Response::new(&mut large);
            if !parsed.parse(bytes).is_ok_and(|status| status.is_complete()) {
                return Err(invalid("invalid HTTP response headers"));
            }
        }
        _ => return Err(invalid("invalid HTTP response headers")),
    }
    framing(
        parsed.code.ok_or_else(|| invalid("missing HTTP status"))?,
        parsed.headers,
        bytes.len(),
    )
}

fn framing(status: u16, headers: &[httparse::Header<'_>], body: usize) -> Result<Head> {
    let mut keep = false;
    let mut close = false;
    for header in headers {
        if header.name.eq_ignore_ascii_case("transfer-encoding") {
            return Err(invalid("Transfer-Encoding is not supported"));
        }
        if header.name.eq_ignore_ascii_case("connection") {
            for token in header
                .value
                .split(|byte| *byte == b',')
                .map(<[u8]>::trim_ascii)
            {
                keep |= token.eq_ignore_ascii_case(b"keep-alive");
                close |= token.eq_ignore_ascii_case(b"close");
            }
        }
    }
    if matches!(status, 204 | 304) {
        return Err(Error::Body(String::new()));
    }
    if (100..200).contains(&status) {
        return Err(protocol(
            "read the final HTTP response",
            "the gateway returned an informational status or a protocol switch",
            "return one final non-streaming Messages response",
        )
        .into());
    }
    let mut lengths = headers
        .iter()
        .filter(|header| header.name.eq_ignore_ascii_case("content-length"));
    let end = if keep
        && !close
        && let Some(header) = lengths.next()
    {
        let value = header.value;
        if lengths.next().is_some() || value.is_empty() || !value.iter().all(u8::is_ascii_digit) {
            return Err(invalid("ambiguous or invalid Content-Length"));
        }
        let digits =
            std::str::from_utf8(value).map_err(|_| invalid("invalid decimal Content-Length"))?;
        let length = digits
            .parse::<usize>()
            .map_err(|_| invalid("Content-Length exceeds the addressable response size"))?;
        let size = body
            .checked_add(length)
            .ok_or_else(|| invalid("Content-Length exceeds the addressable response size"))?;
        Some(size)
    } else {
        None
    };
    Ok(Head { status, body, end })
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::indexing_slicing, reason = "test code")]
mod tests {
    use super::*;

    #[test]
    fn every_split_of_a_persistent_response_has_the_same_boundary() {
        let bytes = b"HTTP/1.1 200 OK\r\nConnection: Keep-Alive\r\nContent-Length: 2\r\n\r\n42";
        for offset in 0..=bytes.len() {
            let mut response = Response::default();
            assert_eq!(
                response.extend(&bytes[..offset]).unwrap(),
                offset == bytes.len()
            );
            assert!(response.extend(&bytes[offset..]).unwrap());
            assert_eq!(response.finish().unwrap(), (Value::from(42), true));
        }
    }

    #[test]
    fn only_one_complete_unambiguous_body_can_return_a_socket() {
        for (headers, body) in [
            ("Content-Length: 2", "1"),
            ("Content-Length: 2", "123"),
            ("Content-Length: 1\r\nContent-Length: 1", "1"),
            ("Content-Length: +1", "1"),
            ("Content-Length: 18446744073709551616", "1"),
            ("Content-Length: 1\r\nTransfer-Encoding: chunked", "1"),
        ] {
            let wire =
                format!("HTTP/1.0 200 OK\r\nConnection: keep-alive\r\n{headers}\r\n\r\n{body}");
            let mut response = Response::default();
            assert!(
                response
                    .extend(wire.as_bytes())
                    .and_then(|_| response.finish())
                    .is_err(),
                "{headers} {body}"
            );
        }
        let mut response = Response::default();
        assert!(!response.extend(b"HTTP/1.0 200 OK\r\nConnection: keep-alive, close\r\nContent-Length: 1\r\n\r\n1").unwrap());
        assert_eq!(response.finish().unwrap(), (Value::from(1), false));
    }
}
