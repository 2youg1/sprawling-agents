// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! One file of the city, as a version (wire-SPEC.md 8-69).
//!
//! A page that edits a file has to know which version it read, so the
//! answer names it; and "nothing is here", "something is here that will
//! not read" and "an empty file" are three answers, because a page offers
//! a different next step for each. What the bytes are - text in which
//! encoding, or not text - and where the first window ends are the
//! `documents` crate's rules; this module only spells the answer.

use documents::{Encoding, Format, Window};
use kernel::{Address, B3Hash};
use serde::{Deserialize, Serialize};

/// One file, and what state it is in.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct DocumentAnswer {
    pub at: Address,
    pub state: DocumentState,
}

/// What stands at an address in the tree.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum DocumentState {
    /// No file is at this address; a save would create one.
    Missing,
    /// Something is here and will not read - a directory, a file this
    /// process may not open - in the operating system's own words.
    Unreadable { reason: String },
    /// A file with no bytes. It has a version all the same, because an
    /// empty file is something a save can be made on.
    Empty { version: B3Hash, format: Format },
    /// A file with bytes.
    Held(Box<HeldDocument>),
}

/// A file with bytes: which version, how long, and what of it a reader
/// is given now.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct HeldDocument {
    /// The BLAKE3 digest of every byte, which is also the address the
    /// content store gives them (`crates/documents/Spec.lean` D3).
    pub version: B3Hash,
    pub format: Format,
    /// The whole version's length.
    pub bytes: u64,
    pub body: DocumentBody,
}

/// What the bytes of one version are to a reader.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum DocumentBody {
    /// Text in `encoding`. `head` is the first window, counted from byte
    /// zero, a byte-order mark included as the text's first character.
    Text {
        encoding: Encoding,
        head: Window,
        coverage: Coverage,
    },
    /// Not text in any encoding this city reads: the version and its
    /// length are all a reader is given.
    Opaque,
}

/// How much of the version the first window holds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum Coverage {
    /// All of it.
    Whole,
    /// Its head only. The version is in the content store, and
    /// `Query::Range` reads the rest of it by its version.
    Head,
}
