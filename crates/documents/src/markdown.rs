// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! One window of a Markdown version, laid out by the city's one Markdown
//! grammar (`crates/documents/spec/Markdown.lean`, D20-D26).
//!
//! The window is cut the way a range is cut, then, when the version runs
//! on past it, ended where its second-to-last block ends: the last block
//! may continue outside the window, and a block laid out from half its
//! lines is a different block. Only the UTF-8 encodings are laid out,
//! because a block names its bytes and only there do the bytes of the
//! text read from a window line up with the version's.

mod lowering;
mod position;
mod target;
mod tree;

pub use tree::{Align, Block, Check, Construct, Inline, ListItem, Order, Preview, Row, Spacing};

use kernel::AxError;

use crate::encoding::Encoding;
use crate::format::Format;
use crate::layout;
use crate::span::{Span, offset};
use crate::window::{Lifted, Window, cut};

/// The preview of the window `viewport` asks for, cut from bytes
/// [`lift`](crate::lift) named.
///
/// # Errors
/// The [`cut`] refusals: lifted bytes that do not hold the window, or
/// that are not text in `encoding`.
pub fn preview(encoding: Encoding, lifted: Lifted<'_>, viewport: Span) -> Result<Preview, AxError> {
    match encoding {
        Encoding::Utf16Le | Encoding::Utf16Be => Ok(Preview::Unsupported { encoding }),
        Encoding::Utf8 | Encoding::Utf8Bom => {
            let window = settled(cut(encoding, lifted, viewport)?, lifted.size);
            let blocks = lowering::blocks(&window.text, window.span.start());
            Ok(Preview::Laid {
                span: window.span,
                blocks,
            })
        }
    }
}

/// The window, ended at its second-to-last block when the version runs
/// on past it (`Documents.Markdown.settle`). A window whose only block
/// runs on, or that reaches the version's end, stays as it was cut, so
/// a reader always moves forward.
fn settled(window: Window, size: u64) -> Window {
    if window.span.end() >= size {
        return window;
    }
    let ends = layout::blocks(Format::Markdown, window.text.as_bytes());
    let Some(end) = ends
        .iter()
        .rev()
        .nth(1)
        .map(|block| block.end())
        .filter(|end| *end > 0)
    else {
        return window;
    };
    let kept = usize::try_from(end)
        .ok()
        .and_then(|end| window.text.get(..end));
    match kept {
        Some(text) => Window {
            span: Span::ordered(
                window.span.start(),
                window.span.start().saturating_add(offset(text.len())),
            ),
            text: text.to_owned(),
        },
        None => window,
    }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    reason = "test code"
)]
mod tests;
