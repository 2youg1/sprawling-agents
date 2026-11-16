// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use kernel::{AxCode, AxError};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "SpanFields", into = "SpanFields")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct Span {
    start: u64,
    end: u64,
}

impl Span {
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

    pub const fn at(at: u64) -> Span {
        Span { start: at, end: at }
    }

    pub const fn start(self) -> u64 {
        self.start
    }

    pub const fn end(self) -> u64 {
        self.end
    }

    pub const fn len(self) -> u64 {
        self.end.saturating_sub(self.start)
    }

    pub const fn is_empty(self) -> bool {
        self.start == self.end
    }

    pub(crate) fn ordered(one: u64, other: u64) -> Span {
        Span {
            start: one.min(other),
            end: one.max(other),
        }
    }

    pub(crate) fn within(self, len: usize) -> Option<(usize, usize)> {
        let start = usize::try_from(self.start).ok()?;
        let end = usize::try_from(self.end).ok()?;
        (end <= len).then_some((start, end))
    }
}

pub(crate) fn offset(index: usize) -> u64 {
    u64::try_from(index).unwrap_or(u64::MAX)
}

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
