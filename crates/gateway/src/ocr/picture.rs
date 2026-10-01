// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! One picture, and the `cas:` locator its bytes hash to.
//!
//! The request names a picture by its locator and the adapter fetches
//! the bytes by that locator at the wire. Holding the two together, and
//! refusing a pair that disagrees, is what makes the bytes that are sent
//! the bytes the locator names.

use kernel::{AxCode, AxError, B3Hash, ImageRef, Locator};

/// A picture ready to show a model: what the request says about it and
/// the bytes behind it.
#[derive(Debug, Clone)]
pub struct Picture {
    seen: ImageRef,
    bytes: Vec<u8>,
}

impl Picture {
    /// # Errors
    /// `E_INVALID_ARGS` when `seen` does not name these bytes by the
    /// `cas:` locator they hash to, with no range.
    pub fn new(seen: ImageRef, bytes: Vec<u8>) -> Result<Picture, AxError> {
        let named = Locator::cas(B3Hash::digest(&bytes));
        if seen.locator != named {
            return Err(AxError::failure(
                AxCode::InvalidArgs,
                "show a picture",
                format!("{} names other bytes than {named}", seen.locator),
            )
            .with_recovery(
                "name the picture by the cas: locator its bytes hash to, with no range",
            ));
        }
        Ok(Picture { seen, bytes })
    }

    pub(super) fn seen(&self) -> &ImageRef {
        &self.seen
    }

    /// The bytes, when `at` is the locator this picture is named by.
    pub(super) fn bytes_at(&self, at: &Locator) -> Option<&[u8]> {
        (*at == self.seen.locator).then_some(self.bytes.as_slice())
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
mod tests {
    use super::*;

    fn seen(locator: Locator) -> ImageRef {
        ImageRef {
            locator,
            media_type: kernel::ImageType::Png,
            width: 1,
            height: 1,
        }
    }

    /// A locator that names other bytes would have the wire send what it
    /// was never asked to send, so the pair is refused where it is made.
    #[test]
    fn a_locator_that_names_other_bytes_is_refused() {
        let bytes = b"the pixels".to_vec();
        let other = Locator::cas(B3Hash::digest(b"other pixels"));
        let codes = [
            Picture::new(seen(Locator::cas(B3Hash::digest(&bytes))), bytes.clone())
                .err()
                .map(|err| *err.code()),
            Picture::new(seen(other), bytes)
                .err()
                .map(|err| *err.code()),
        ];
        assert_eq!(codes, [None, Some(AxCode::InvalidArgs)]);
    }
}
