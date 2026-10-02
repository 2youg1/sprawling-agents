// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! A stored object as bytes, window by window, and a Markdown version
//! exported as one HTML file (`crates/wire/spec/Answer/DocumentBytes.lean`
//! §8-80, §8-81).
//!
//! Bytes travel on the same socket as every other answer, so a device on
//! the remote door reaches them through its sealed session (wire D18).

use documents::Span;
use kernel::B3Hash;
use serde::{Deserialize, Serialize};

/// The most bytes one `Bytes` answer carries.
pub const BYTES_WINDOW_MAX: u64 = 1 << 20;

/// One window of a stored object's bytes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct BytesAnswer {
    pub version: B3Hash,
    /// The stretch answered; the next request starts at its end.
    pub span: Span,
    /// The whole object's length.
    pub size: u64,
    /// The stretch's bytes, in the standard base64 alphabet with padding.
    pub base64: String,
}

/// A Markdown version written as one HTML file that stands alone.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct ExportAnswer {
    pub version: B3Hash,
    pub html: String,
}
