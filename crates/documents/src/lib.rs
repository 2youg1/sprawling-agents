// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The rules of a document a person reads and edits through a page:
//! which version it is, which characters its bytes spell, where its
//! blocks lie, how much of it one answer carries, what one save
//! changes, and how a suggestion about it is read and decided
//! (`crates/documents/Spec.lean`).
//!
//! No I/O: the bytes arrive as slices and leave as values. Reading the
//! disk and the content store, and recording a save, belong to the
//! crates that own those (D1).

mod edit;
mod encoding;
mod format;
mod layout;
mod markdown;
mod proposal;
mod selection;
mod span;
mod window;

pub use edit::{Applied, Edit, TextEdit, Transaction, save};
pub use encoding::{Encoding, Reading};
pub use format::Format;
pub use markdown::{
    Align, Block, Check, Construct, Inline, ListItem, Order, Preview, Row, Spacing, preview,
};
pub use proposal::{ALIGN_CELLS_MAX, Offer, PROPOSAL_ID_TAG, Review, Slice, SliceKind, decide};
pub use selection::Selection;
pub use span::Span;
pub use window::{Lifted, WINDOW_BYTES_MAX, Window, cut, head, lift};
