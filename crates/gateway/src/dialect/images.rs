// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The bytes behind the pictures one request refers to.
//!
//! A leaf value both dialect families read, in the same position
//! `mismatch` holds: nothing here reaches back up. The translation stays
//! a pure function of data because the bytes arrive as a parameter —
//! whoever is about to send the request resolves each `cas:` locator
//! first, so the same pair of values always produces the same bytes on
//! the wire and a replay can rebuild them.

use std::collections::BTreeMap;

use base64::Engine as _;
use kernel::{AxError, Locator};

use crate::mismatch::mismatch;

/// Pictures by locator.
///
/// Keyed by the locator's canonical spelling, which is what makes the
/// iteration order a fact about the content rather than about the run.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ImageBytes(BTreeMap<String, Vec<u8>>);

impl ImageBytes {
    pub fn insert(&mut self, at: &Locator, bytes: Vec<u8>) {
        self.0.insert(at.to_string(), bytes);
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.0.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// The picture as both wires carry it: standard base64, padded.
    ///
    /// # Errors
    /// `E_WIRE_MISMATCH` when nobody resolved this locator. Sending the
    /// request without the picture would leave the model looking at a
    /// gap it was told about, so the whole call is refused instead.
    pub(crate) fn encoded(&self, at: &Locator) -> Result<String, AxError> {
        let bytes = self.0.get(&at.to_string()).ok_or_else(|| {
            mismatch(
                "content.image.locator",
                "no bytes were resolved for this picture",
            )
        })?;
        Ok(base64::engine::general_purpose::STANDARD.encode(bytes))
    }
}
