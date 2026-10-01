// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! A stand-in on loopback that hears one request meant for a preset
//! host (gateway-SPEC.md section 8-32).
//!
//! The shape of a request depends on the host it is addressed to: the
//! conversation header and the chat face's spelling are looked up by the
//! host name. The stand-in records what it heard - the path of the
//! request line, every header, the body - so a test can compare that
//! with what the host's vendor documents.
//!
//! Every item here is `#[cfg(test)]`: nothing in this module reaches a
//! shipped binary.

#![cfg(test)]
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::string_slice,
    clippy::arithmetic_side_effects,
    reason = "test helpers"
)]

use std::collections::BTreeMap;
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener};
use std::thread::JoinHandle;

/// What the stand-in heard: one request, taken apart.
#[derive(Debug)]
pub(crate) struct Heard {
    /// The path of the request line, without a query.
    pub(crate) path: String,
    /// Every header, its name lower-cased.
    pub(crate) headers: BTreeMap<String, String>,
    pub(crate) body: serde_json::Value,
}

/// A listener on loopback that answers one request with a refusal and
/// hands back what it heard.
pub(crate) struct StandIn {
    at: SocketAddr,
    served: JoinHandle<Heard>,
}

impl StandIn {
    /// Listens for one request meant for `host`.
    pub(crate) fn listening(host: &str) -> StandIn {
        let _named = host;
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let at = listener.local_addr().unwrap();
        let served = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let heard = read_request(&mut stream);
            refuse(&mut stream);
            heard
        });
        StandIn { at, served }
    }

    /// The address a client that cannot resolve a name to this stand-in
    /// has to be given instead of the host's own base URL.
    pub(crate) fn loopback(&self, base_url: &str) -> String {
        let (_, rest) = base_url.split_once("://").unwrap();
        let path = rest.find('/').map_or("", |at| &rest[at..]);
        format!("http://{}{path}", self.at)
    }

    /// What the stand-in heard, once its one request has come and gone.
    pub(crate) fn heard(self) -> Heard {
        self.served.join().unwrap()
    }
}

/// Reads one HTTP/1.1 request: the head, then as many body bytes as
/// `content-length` says.
fn read_request(stream: &mut impl Read) -> Heard {
    let mut bytes = Vec::new();
    let mut buf = [0u8; 16_384];
    let head_end = loop {
        let n = stream.read(&mut buf).unwrap();
        assert!(n > 0, "the client closed before its request was whole");
        bytes.extend_from_slice(&buf[..n]);
        if let Some(at) = bytes.windows(4).position(|w| w == b"\r\n\r\n") {
            break at;
        }
    };
    let head = String::from_utf8(bytes[..head_end].to_vec()).unwrap();
    let mut lines = head.split("\r\n");
    let request_line = lines.next().unwrap();
    let target = request_line.split(' ').nth(1).unwrap();
    let path = target.split('?').next().unwrap().to_owned();
    let headers: BTreeMap<String, String> = lines
        .map(|line| {
            let (name, value) = line.split_once(':').unwrap();
            (name.trim().to_ascii_lowercase(), value.trim().to_owned())
        })
        .collect();
    let length: usize = headers
        .get("content-length")
        .map_or(0, |value| value.parse().unwrap());
    let mut body = bytes[head_end + 4..].to_vec();
    while body.len() < length {
        let n = stream.read(&mut buf).unwrap();
        assert!(n > 0, "the client closed before its body was whole");
        body.extend_from_slice(&buf[..n]);
    }
    Heard {
        path,
        headers,
        body: serde_json::from_slice(&body).unwrap(),
    }
}

/// Answers with a refusal: what the client does with an answer is not
/// what this stand-in is asked about.
fn refuse(stream: &mut impl Write) {
    let body = "{}";
    stream
        .write_all(
            format!(
                "HTTP/1.1 500 X\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
                body.len()
            )
            .as_bytes(),
        )
        .unwrap();
    stream.flush().unwrap();
}

#[cfg(test)]
mod tests;
