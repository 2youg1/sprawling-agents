// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

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
pub use markdown::{Align, Block, Check, Construct, EXPORT_BYTES_MAX, Inline, ListItem, export};
pub use markdown::{Laid, Order, Preview, ReplyState, Row, Spacing, preview, reply};
pub use proposal::{ALIGN_CELLS_MAX, Offer, PROPOSAL_ID_TAG, Review, Slice, SliceKind, decide};
pub use selection::Selection;
pub use span::Span;
pub use window::{Lifted, WINDOW_BYTES_MAX, Window, cut, head, lift};
