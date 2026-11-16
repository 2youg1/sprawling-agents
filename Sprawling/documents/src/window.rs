// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! How much of a version one answer carries, and where it is cut
//! (`crates/documents/spec/Window.lean`, D7).
//!
//! A window never splits a character, never runs past the version, and
//! never carries more than [`WINDOW_BYTES_MAX`] bytes past the offset it
//! was asked from. A cut moves backwards to the nearest character
//! boundary rather than forwards, so a window is never longer than it
//! was allowed to be; a start inside a character moves back to that
//! character's first byte, so a reader that asked from the middle of one
//! is given the whole of it.

use kernel::{AxCode, AxError};
use serde::{Deserialize, Serialize};

use crate::encoding::Encoding;
use crate::format::Format;
use crate::layout;
use crate::span::{Span, offset};

/// The most bytes one window carries past the offset it was asked from.
///
/// The bound the document answer had before it carried versions, kept so
/// a page's first answer costs what it cost; the reading that would move
/// it is the first-viewport time over a fixed corpus (refrain roadmap
/// A11), which no machine has taken yet.
pub const WINDOW_BYTES_MAX: u64 = 64 * 1024;

/// How far a boundary can lie behind a byte in any encoding read here:
/// three continuation bytes in UTF-8, one odd byte and a low surrogate in
/// UTF-16.
const REACH: u64 = 3;

/// One stretch of a version, and the characters it spells.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct Window {
    pub span: Span,
    pub text: String,
}

/// Bytes lifted out of a version so a window can be cut from them.
#[derive(Debug, Clone, Copy)]
pub struct Lifted<'a> {
    /// The offset of `bytes[0]` in the version.
    pub at: u64,
    pub bytes: &'a [u8],
    /// The length of the whole version.
    pub size: u64,
}

/// The first window of a whole version.
///
/// A version that fits is given whole. One that does not ends at the
/// last block that fits, so the first screen a reader sees holds no half
/// paragraph; when not even the first block fits, at the last character
/// boundary that does. Blocks are read only in the UTF-8 encodings,
/// whose line endings are the bytes the layout looks for.
///
/// # Errors
/// The [`Encoding::decode`] refusal, for bytes that are not text in
/// `encoding`.
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

/// The bytes to lift out of a version of `size` bytes to cut the window
/// `wanted` asks for: up to [`REACH`] bytes before it, so a start inside
/// a character can step back to that character, and up to [`REACH`]
/// after its bounded end, so the end can tell whether a character runs
/// on past it.
pub fn lift(wanted: Span, size: u64) -> Span {
    let start = wanted.start().min(size);
    Span::ordered(
        start.saturating_sub(REACH),
        bounded_end(wanted, size).saturating_add(REACH).min(size),
    )
}

/// The window `wanted` asks for, cut from bytes [`lift`] named.
///
/// # Errors
/// The [`Encoding::decode`] refusal, for bytes that are not text in
/// `encoding` - which is also what a cut that found no boundary inside
/// the lifted bytes decodes to - and `E_INVALID_ARGS` when `lifted`
/// does not hold the bytes the window needs.
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

/// Where a window asked for as `wanted` may end at most: inside the
/// version, and no more than [`WINDOW_BYTES_MAX`] past its start.
fn bounded_end(wanted: Span, size: u64) -> u64 {
    let start = wanted.start().min(size);
    wanted
        .end()
        .min(size)
        .min(start.saturating_add(WINDOW_BYTES_MAX))
        .max(start)
}

impl Lifted<'_> {
    /// The nearest character boundary at or before `at`, never stepping
    /// below the first lifted byte. The version's two ends are
    /// boundaries whatever bytes sit beside them.
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
