// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::io::{self, ErrorKind};

use bytes::{Buf, Bytes, BytesMut};
use h3_webtransport::stream::BidiStream;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

use super::relay::HANDSHAKE_PATIENCE;

type Stream = BidiStream<h3_quinn::BidiStream<Bytes>, Bytes>;

const HEADER_BYTES: usize = 5;
const OPENING_FRAME_BYTES: usize = 16 * 1024;

pub(super) struct Device {
    stream: Stream,
    received: BytesMut,
    limit: usize,
}

impl Device {
    pub(super) fn new(stream: Stream) -> Self {
        Self {
            stream,
            received: BytesMut::with_capacity(4096),
            limit: OPENING_FRAME_BYTES,
        }
    }

    pub(super) fn authenticated(&mut self) {
        self.limit = wire::DROP_BYTES_MAX;
    }

    pub(super) async fn recv(&mut self) -> io::Result<Option<Bytes>> {
        loop {
            if let [kind, a, b, c, d, ..] = self.received.as_ref() {
                let bytes = usize::try_from(u32::from_be_bytes([*a, *b, *c, *d]))
                    .map_err(|source| io::Error::new(ErrorKind::InvalidData, source))?;
                if *kind != 0 || bytes > self.limit {
                    return Err(io::Error::new(
                        ErrorKind::InvalidData,
                        "invalid device frame",
                    ));
                }
                if self.received.len().saturating_sub(HEADER_BYTES) >= bytes {
                    self.received.advance(HEADER_BYTES);
                    return Ok(Some(self.received.split_to(bytes).freeze()));
                }
            }
            if self.stream.read_buf(&mut self.received).await? == 0 {
                if self.received.is_empty() {
                    return Ok(None);
                }
                return Err(io::Error::new(
                    ErrorKind::UnexpectedEof,
                    "partial device frame",
                ));
            }
        }
    }

    pub(super) async fn send(&mut self, bytes: &[u8]) -> io::Result<()> {
        self.packet(0, bytes).await
    }

    pub(super) async fn close(&mut self, code: &str) -> io::Result<()> {
        self.packet(1, code.as_bytes()).await?;
        self.stream.shutdown().await
    }

    async fn packet(&mut self, kind: u8, bytes: &[u8]) -> io::Result<()> {
        let length = u32::try_from(bytes.len())
            .map_err(|source| io::Error::new(ErrorKind::InvalidData, source))?;
        let [a, b, c, d] = length.to_be_bytes();
        tokio::time::timeout(HANDSHAKE_PATIENCE, async {
            self.stream.write_all(&[kind, a, b, c, d]).await?;
            self.stream.write_all(bytes).await
        })
        .await?
    }
}
