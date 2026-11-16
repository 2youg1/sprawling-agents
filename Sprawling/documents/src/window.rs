// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use kernel::{AxCode, AxError};
use serde::{Deserialize, Serialize};

use crate::encoding::Encoding;
use crate::format::Format;
use crate::layout;
use crate::span::{Span, offset};

pub const WINDOW_BYTES_MAX: u64 = 64 * 1024;

const REACH: u64 = 3;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "schema", schemars(title = "Window2"))]
pub struct Window {
    pub span: Span,
    pub text: String,
}

#[derive(Debug, Clone, Copy)]
pub struct Lifted<'a> {
    pub at: u64,
    pub bytes: &'a [u8],

    pub size: u64,
}

pub fn head(format: Format, encoding: Encoding, source: &[u8]) -> Result<Window, AxError> {
    let size = offset(source.len());
    let fits = match encoding {
        Encoding::Utf8 | Encoding::Utf8Bom if size > WINDOW_BYTES_MAX => {
            layout::blocks(format, source)
                .iter()
                .map(|block| block.end())
                .take_while(|end| *end <= WINDOW_BYTES_MAX)
                .last()
                .filter(|end| *end > 0)
        }
        Encoding::Utf8 | Encoding::Utf8Bom | Encoding::Utf16Le | Encoding::Utf16Be => None,
    };
    let wanted = Span::ordered(0, fits.unwrap_or(size));
    cut(
        encoding,
        Lifted {
            at: 0,
            bytes: source,
            size,
        },
        wanted,
    )
}

pub fn lift(wanted: Span, size: u64) -> Span {
    let start = wanted.start().min(size);
    Span::ordered(
        start.saturating_sub(REACH),
        bounded_end(wanted, size).saturating_add(REACH).min(size),
    )
}

pub fn cut(encoding: Encoding, lifted: Lifted<'_>, wanted: Span) -> Result<Window, AxError> {
    let start = lifted.boundary_at_or_before(encoding, wanted.start().min(lifted.size));
    let end = lifted
        .boundary_at_or_before(encoding, bounded_end(wanted, lifted.size))
        .max(start);
    let span = Span::ordered(start, end);
    let bytes = lifted.slice(span).ok_or_else(|| {
        AxError::failure(
            AxCode::InvalidArgs,
            "read a document window",
            format!(
                "{}..{} lies outside the bytes lifted from {}",
                span.start(),
                span.end(),
                lifted.at
            ),
        )
        .with_recovery("lift the bytes `lift` names for this window, then cut again")
    })?;
    let text = encoding.decode(bytes)?;
    Ok(Window { span, text })
}

fn bounded_end(wanted: Span, size: u64) -> u64 {
    let start = wanted.start().min(size);
    wanted
        .end()
        .min(size)
        .min(start.saturating_add(WINDOW_BYTES_MAX))
        .max(start)
}

impl Lifted<'_> {
    fn boundary_at_or_before(&self, encoding: Encoding, at: u64) -> u64 {
        let floor = self.at.min(at);
        let mut probe = at;
        while probe > floor && !self.is_boundary(encoding, probe) {
            probe = probe.saturating_sub(1);
        }
        probe
    }

    fn is_boundary(&self, encoding: Encoding, at: u64) -> bool {
        if at == 0 || at >= self.size {
            return true;
        }
        let index = at
            .checked_sub(self.at)
            .and_then(|index| usize::try_from(index).ok());
        index.is_some_and(|index| encoding.begins_at(self.bytes, index, self.at))
    }

    fn slice(&self, span: Span) -> Option<&[u8]> {
        let from = span.start().checked_sub(self.at)?;
        let to = span.end().checked_sub(self.at)?;
        let (from, to) = Span::ordered(from, to).within(self.bytes.len())?;
        self.bytes.get(from..to)
    }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "test code"
)]
mod tests;
