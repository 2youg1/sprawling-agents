// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! What a running command has written since the last poll, handed to a
//! sink while the command still runs (runtime-SPEC 8-28-3).
//!
//! A child writes to two files, never to pipes, so reading its output
//! live needs nobody to change how it writes: the window's poll loop
//! reads what the files grew by since its last visit. What it reads is
//! a preview a page may drop. The tool result that lands in the ledger
//! is the one authority on this output, so a failed read is not a
//! failure of the command, only a piece the page sees one poll later.

use std::io::{Read, Seek, SeekFrom};
use std::sync::Arc;

use kernel::RunId;

use super::BacklogId;

/// Which of a command's two outputs a piece came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stream {
    Out,
    Err,
}

/// One piece of output, in the order it was written to its stream.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Chunk {
    pub run: RunId,
    pub member: BacklogId,
    pub stream: Stream,
    pub bytes: Vec<u8>,
}

/// Where the pieces go. A backlog without one reads nothing at all, so a
/// city nobody watches pays for none of this.
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

/// How far one member's two files have been read.
#[derive(Debug, Default)]
pub(super) struct Tail {
    out: u64,
    err: u64,
}

impl Tail {
    /// Takes what each file grew by since the last take, at most half
    /// of `per_poll` bytes from each, so a flood on stdout never starves
    /// stderr and a command that writes faster than a page reads leaves
    /// the page behind rather than the poll slower. The caller delivers
    /// them, which lets it release the table first.
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

/// Up to `cap` bytes of `file` past `offset`; nothing when the file
/// cannot be read, because the next poll asks again from the same place.
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
