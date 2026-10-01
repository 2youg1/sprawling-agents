// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! A stand-in on loopback reached under a preset host's own name
//! (gateway-SPEC.md section 8-32).
//!
//! The shape of a request depends on the host it is addressed to: the
//! conversation header and the chat face's spelling are looked up by the
//! host name, so a stand-in addressed as `127.0.0.1` never sees them. The
//! stand-in here speaks TLS under a certificate signed for that one host,
//! and [`StandIn::toward`] is the step after `client_for` that resolves
//! the host to the stand-in, trusts that certificate alone and leaves the
//! proxy out. The base URL is not touched, so the host, the path, the
//! name the handshake asks for and the `Host` header are the real
//! request's.
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
    clippy::as_conversions,
    clippy::cast_possible_truncation,
    reason = "test helpers"
)]

use std::collections::BTreeMap;
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener};
use std::sync::Arc;
use std::thread::JoinHandle;

use rustls::pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer};

/// What the stand-in heard: one request, taken apart.
#[derive(Debug)]
pub(crate) struct Heard {
    /// The path of the request line, without a query.
    pub(crate) path: String,
    /// Every header, its name lower-cased.
    pub(crate) headers: BTreeMap<String, String>,
    pub(crate) body: serde_json::Value,
}

/// A TLS listener on loopback that answers one request meant for one
/// host with a refusal, and hands back what it heard.
pub(crate) struct StandIn {
    host: String,
    at: SocketAddr,
    certificate: Vec<u8>,
    served: JoinHandle<Heard>,
}

impl StandIn {
    /// Signs a certificate for `host` and listens for one request.
    pub(crate) fn listening(host: &str) -> StandIn {
        let (certificate, key) = self_signed(host);
        let config = rustls::ServerConfig::builder_with_provider(Arc::new(
            rustls::crypto::aws_lc_rs::default_provider(),
        ))
        .with_safe_default_protocol_versions()
        .unwrap()
        .with_no_client_auth()
        .with_single_cert(
            vec![CertificateDer::from(certificate.clone())],
            PrivateKeyDer::Pkcs8(key),
        )
        .unwrap();
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let at = listener.local_addr().unwrap();
        let served = std::thread::spawn(move || {
            let (socket, _) = listener.accept().unwrap();
            let session = rustls::ServerConnection::new(Arc::new(config)).unwrap();
            let mut stream = rustls::StreamOwned::new(session, socket);
            let heard = read_request(&mut stream);
            refuse(&mut stream);
            stream.conn.send_close_notify();
            stream.flush().unwrap();
            heard
        });
        StandIn {
            host: host.to_owned(),
            at,
            certificate,
            served,
        }
    }

    /// The step after `client_for`: this host's name resolves to the
    /// stand-in, the stand-in's certificate is the only one trusted, and
    /// no proxy takes the request - a proxy would be handed the name and
    /// do its own resolving.
    pub(crate) fn toward(
        &self,
    ) -> impl Fn(reqwest::blocking::ClientBuilder) -> reqwest::blocking::ClientBuilder
    + Send
    + Sync
    + 'static {
        let (host, at, certificate) = (self.host.clone(), self.at, self.certificate.clone());
        move |builder| {
            builder
                .resolve(&host, at)
                .tls_certs_only([reqwest::Certificate::from_der(&certificate).unwrap()])
                .no_proxy()
        }
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

/// A certificate for `host` that signs itself, and its key.
///
/// The smallest leaf the verifier accepts: version 3, an Ed25519 key, a
/// validity from 2000 to 2049, and one `subjectAltName` naming the host,
/// because the name is what the verifier checks and the common name is
/// not. No extended key usage, which the verifier reads as any use. The
/// key is a fixed seed: the certificate protects nothing but one
/// loopback handshake inside a test.
fn self_signed(host: &str) -> (Vec<u8>, PrivatePkcs8KeyDer<'static>) {
    let mut pkcs8 = vec![
        0x30, 0x2e, 0x02, 0x01, 0x00, 0x30, 0x05, 0x06, 0x03, 0x2b, 0x65, 0x70, 0x04, 0x22, 0x04,
        0x20,
    ];
    pkcs8.extend_from_slice(&[7u8; 32]);
    let key = PrivatePkcs8KeyDer::from(pkcs8);
    let signing = rustls::crypto::aws_lc_rs::sign::any_eddsa_type(&key).unwrap();
    let ed25519 = der(0x30, &der(0x06, &[0x2b, 0x65, 0x70]));
    let name = der(
        0x30,
        &der(
            0x31,
            &der(
                0x30,
                &[der(0x06, &[0x55, 0x04, 0x03]), der(0x0c, b"stand-in")].concat(),
            ),
        ),
    );
    let validity = der(
        0x30,
        &[der(0x17, b"000101000000Z"), der(0x17, b"491231235959Z")].concat(),
    );
    let alt_names = der(0x30, &der(0x82, host.as_bytes()));
    let extensions = der(
        0xa3,
        &der(
            0x30,
            &der(
                0x30,
                &[der(0x06, &[0x55, 0x1d, 0x11]), der(0x04, &alt_names)].concat(),
            ),
        ),
    );
    let tbs = der(
        0x30,
        &[
            der(0xa0, &der(0x02, &[0x02])),
            der(0x02, &[0x01]),
            ed25519.clone(),
            name.clone(),
            validity,
            name,
            signing.public_key().unwrap().as_ref().to_vec(),
            extensions,
        ]
        .concat(),
    );
    let signature = signing
        .choose_scheme(&[rustls::SignatureScheme::ED25519])
        .unwrap()
        .sign(&tbs)
        .unwrap();
    let certificate = der(
        0x30,
        &[tbs, ed25519, der(0x03, &[&[0u8][..], &signature].concat())].concat(),
    );
    (certificate, key)
}

/// One DER element: the tag, the definite length, the content.
fn der(tag: u8, content: &[u8]) -> Vec<u8> {
    let length = content.len();
    let mut out = vec![tag];
    match length {
        0..=0x7f => out.push(length as u8),
        0x80..=0xff => out.extend([0x81, length as u8]),
        _ => out.extend([0x82, (length >> 8) as u8, length as u8]),
    }
    out.extend_from_slice(content);
    out
}

#[cfg(test)]
mod tests;
