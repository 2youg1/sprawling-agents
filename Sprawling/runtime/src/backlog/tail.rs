// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::io::{Read, Seek, SeekFrom};
use std::sync::Arc;

use kernel::RunId;

use super::BacklogId;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stream {
    Out,
    Err,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Chunk {
    pub run: RunId,
    pub member: BacklogId,
    pub stream: Stream,
    pub bytes: Vec<u8>,
}

#[derive(Clone)]
pub struct Sink(Arc<dyn Fn(Chunk) + Send + Sync>);

impl Sink {
    #[must_use]
    pub fn new(deliver: impl Fn(Chunk) + Send + Sync + 'static) -> Sink {
        Sink(Arc::new(deliver))
    }

    pub(super) fn deliver(&self, chunks: Vec<Chunk>) {
        chunks.into_iter().for_each(|chunk| (self.0)(chunk));
    }
}

#[derive(Debug, Default)]
pub(super) struct Tail {
    out: u64,
    err: u64,
}

impl Tail {

    pub(super) fn take(
        &mut self,
        dir: &std::path::Path,
        from: (RunId, BacklogId),
        per_poll: usize,
    ) -> Vec<Chunk> {
        let half = per_poll.div_ceil(2);
        let mut chunks = Vec::new();
        for (stream, name, offset) in [
            (Stream::Out, "out", &mut self.out),
            (Stream::Err, "err", &mut self.err),
        ] {
            let bytes = grown(&dir.join(name), *offset, half);
            if bytes.is_empty() {
                continue;
            }
            *offset = offset.saturating_add(u64::try_from(bytes.len()).unwrap_or(u64::MAX));
            chunks.push(Chunk {
                run: from.0,
                member: from.1,
                stream,
                bytes,
            });
        }
        chunks
    }
}

fn grown(file: &std::path::Path, offset: u64, cap: usize) -> Vec<u8> {
    let mut bytes = Vec::new();
    let limit = u64::try_from(cap).unwrap_or(u64::MAX);
    let read = std::fs::File::open(file).and_then(|mut opened| {
        opened.seek(SeekFrom::Start(offset))?;
        opened.take(limit).read_to_end(&mut bytes)
    });
    match read {
        Ok(_) => bytes,
        Err(_) => Vec::new(),
    }
}
