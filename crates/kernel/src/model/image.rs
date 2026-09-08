// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! What a picture is on the canonical conversation.
//!
//! The ledger holds a reference and four integers, never bytes: the
//! bytes live in `memory::cas` under the locator this value carries, so
//! a history stays a file a person can read and the float ban holds
//! without an exception for pixels.

use serde::{Deserialize, Serialize};

use crate::locator::Locator;

/// The picture formats this city carries.
///
/// Closed rather than `non_exhaustive`, because this is not a vocabulary
/// a provider hands us: it is the set we decided to accept, and adding
/// one has to answer [`ImageType::mime`] in the same edit.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ImageType {
    Png,
    Jpeg,
    Webp,
    Gif,
}

impl ImageType {
    /// The media type both provider wires spell this format with.
    #[must_use]
    pub fn mime(&self) -> &'static str {
        match self {
            ImageType::Png => "image/png",
            ImageType::Jpeg => "image/jpeg",
            ImageType::Webp => "image/webp",
            ImageType::Gif => "image/gif",
        }
    }
}

/// One picture, as everything except the wire refers to it.
///
/// The same value stands in a [`crate::ContentBlock::Image`] block and in
/// a tool result's attachments, because they are the same four facts. A
/// second definition of them would be a second authority: adding a field
/// would then have to be remembered twice.
///
/// `width` and `height` are pixel counts, so they are integers; a
/// display that wants a ratio computes it where the drawing happens.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImageRef {
    /// A `cas:` locator. The bytes are fetched from the content store by
    /// whoever is about to send them, and never travel in the ledger.
    pub locator: Locator,
    pub media_type: ImageType,
    pub width: u32,
    pub height: u32,
}
