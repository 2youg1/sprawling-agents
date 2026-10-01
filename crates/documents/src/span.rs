// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! A stretch of one document version, counted in bytes from its first
//! byte (`crates/documents/Spec.lean` D2).

use kernel::{AxCode, AxError};
use serde::{Deserialize, Serialize};

/// A half-open byte interval `[start, end)` of one document version.
///
/// Half-open, so an empty span - an insertion point, the whole of an
/// empty document - can be spelled, which the closed byte range of a
/// `Locator` cannot do. [`Span::new`] is the one construction point and
/// the wire reads through it, so a span whose start lies past its end
/// is never held.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "SpanFields", into = "SpanFields")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct Span {
    start: u64,
    end: u64,
}

impl Span {
    /// The interval from `start` up to, not including, `end`.
    ///
    /// # Errors
    /// `E_INVALID_ARGS` naming both offsets when `start` lies past `end`.
    pub fn new(start: u64, end: u64) -> Result<Span, AxError> {
        if start > end {
            return Err(AxError::failure(
                AxCode::InvalidArgs,
                "name a stretch of a document",
                format!("{start}..{end}"),
            )
            .with_recovery("give the start first: a span runs from its start up to its end"));
        }
        Ok(Span { start, end })
    }

    /// The empty span at `at`: where an insertion lands.
    pub const fn at(at: u64) -> Span {
        Span { start: at, end: at }
    }

    pub const fn start(self) -> u64 {
        self.start
    }

    pub const fn end(self) -> u64 {
        self.end
    }

    /// How many bytes the span covers.
    pub const fn len(self) -> u64 {
        self.end.saturating_sub(self.start)
    }

    pub const fn is_empty(self) -> bool {
        self.start == self.end
    }

    /// The span from the lesser of two offsets to the greater, for the
    /// callers in this crate that computed both and know their order.
    pub(crate) fn ordered(one: u64, other: u64) -> Span {
        Span {
            start: one.min(other),
            end: one.max(other),
        }
    }

    /// The same span as indices into a byte slice of `len` bytes, or
    /// `None` when it reaches past the slice.
    pub(crate) fn within(self, len: usize) -> Option<(usize, usize)> {
        let start = usize::try_from(self.start).ok()?;
        let end = usize::try_from(self.end).ok()?;
        (end <= len).then_some((start, end))
    }
}

/// A byte count or index as an offset. Every slice this crate reads fits
/// in a `u64`, so the ceiling is never reached on a machine this runs on.
pub(crate) fn offset(index: usize) -> u64 {
    u64::try_from(index).unwrap_or(u64::MAX)
}

/// The wire spelling of a [`Span`], read back through [`Span::new`].
#[derive(Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "schema", schemars(rename = "Span"))]
struct SpanFields {
    start: u64,
    end: u64,
}

impl TryFrom<SpanFields> for Span {
    type Error = AxError;

    fn try_from(fields: SpanFields) -> Result<Span, AxError> {
        Span::new(fields.start, fields.end)
    }
}

impl From<Span> for SpanFields {
    fn from(span: Span) -> SpanFields {
        SpanFields {
            start: span.start,
            end: span.end,
        }
    }
}
