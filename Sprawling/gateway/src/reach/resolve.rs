// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

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
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;

use rustls::pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer};

#[derive(Debug)]
pub(crate) struct Heard {

    pub(crate) path: String,

    pub(crate) headers: BTreeMap<String, String>,
    pub(crate) body: serde_json::Value,
}

pub(crate) struct StandIn {
    host: String,
    at: SocketAddr,
    certificate: Vec<u8>,
    served: JoinHandle<Heard>,
}

impl StandIn {

    pub(crate) fn listening(host: &str) -> StandIn {
        let (certificate, config) = server_for(host);
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let at = listener.local_addr().unwrap();
        let served = std::thread::spawn(move || {
            let (socket, _) = listener.accept().unwrap();
            let session = rustls::ServerConnection::new(config).unwrap();
            let mut stream = rustls::StreamOwned::new(session, socket);
            let heard = read_request(&mut stream).unwrap();
            refuse(&mut stream, Closing::Close);
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

    pub(crate) fn toward(
        &self,
    ) -> impl Fn(reqwest::blocking::ClientBuilder) -> reqwest::blocking::ClientBuilder
    + Send
    + Sync
    + 'static {
        toward(&self.host, self.at, &self.certificate)
    }

    pub(crate) fn heard(self) -> Heard {
        self.served.join().unwrap()
    }
}

pub(crate) struct KeptOpen {
    host: String,
    at: SocketAddr,
    certificate: Vec<u8>,

    served_on: Arc<Mutex<Vec<usize>>>,
}

impl KeptOpen {

    pub(crate) fn listening(host: &str) -> KeptOpen {
        let (certificate, config) = server_for(host);
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let at = listener.local_addr().unwrap();
        let served_on = Arc::new(Mutex::new(Vec::new()));
        let noted = Arc::clone(&served_on);
        std::thread::spawn(move || {
            for (connection, socket) in listener.incoming().enumerate() {
                let session = rustls::ServerConnection::new(Arc::clone(&config)).unwrap();
                let mut stream = rustls::StreamOwned::new(session, socket.unwrap());
                let noted = Arc::clone(&noted);
                std::thread::spawn(move || {
                    while read_request(&mut stream).is_some() {
                        noted.lock().unwrap().push(connection);
                        refuse(&mut stream, Closing::KeepOpen);
                    }
                });
            }
        });
        KeptOpen {
            host: host.to_owned(),
            at,
            certificate,
            served_on,
        }
    }

    pub(crate) fn toward(
        &self,
    ) -> impl Fn(reqwest::blocking::ClientBuilder) -> reqwest::blocking::ClientBuilder
    + Send
    + Sync
    + 'static {
        toward(&self.host, self.at, &self.certificate)
    }

    pub(crate) fn served_on(&self) -> Vec<usize> {
        self.served_on.lock().unwrap().clone()
    }
}

fn server_for(host: &str) -> (Vec<u8>, Arc<rustls::ServerConfig>) {
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
    (certificate, Arc::new(config))
}

fn toward(
    host: &str,
    at: SocketAddr,
    certificate: &[u8],
) -> impl Fn(reqwest::blocking::ClientBuilder) -> reqwest::blocking::ClientBuilder + Send + Sync + 'static
{
    let (host, certificate) = (host.to_owned(), certificate.to_vec());
    move |builder| {
        builder
            .resolve(&host, at)
            .tls_certs_only([reqwest::Certificate::from_der(&certificate).unwrap()])
            .no_proxy()
    }
}

fn read_request(stream: &mut impl Read) -> Option<Heard> {
    let mut bytes = Vec::new();
    let mut buf = [0u8; 16_384];
    let head_end = loop {
        let n = stream.read(&mut buf).unwrap_or(0);
        if n == 0 && bytes.is_empty() {
            return None;
        }
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
    Some(Heard {
        path,
        headers,
        body: serde_json::from_slice(&body).unwrap_or(serde_json::Value::Null),
    })
}

#[derive(Clone, Copy)]
enum Closing {
    Close,
    KeepOpen,
}

fn refuse(stream: &mut impl Write, closing: Closing) {
    let body = "{}";
    let connection = match closing {
        Closing::Close => "connection: close\r\n",
        Closing::KeepOpen => "",
    };
    stream
        .write_all(
            format!(
                "HTTP/1.1 500 X\r\ncontent-type: application/json\r\ncontent-length: {}\r\n{connection}\r\n{body}",
                body.len()
            )
            .as_bytes(),
        )
        .unwrap();
    stream.flush().unwrap();
}

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
