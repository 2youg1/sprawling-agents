// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Nightly snapshot identity (`crates/sprawling/spec/Nightly.lean` D105).

use kernel::{AxCode, AxError};

/// A UTC-dated tag and the complete main commit it identifies.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NightlyBuild {
    tag: String,
    source_sha: String,
}

impl NightlyBuild {
    /// Reads the identity carried by a nightly build or release.
    ///
    /// # Errors
    /// When the tag or full source SHA does not satisfy the contract.
    pub fn new(tag: &str, _source_sha: &str) -> Result<NightlyBuild, AxError> {
        Err(AxError::failure(AxCode::ConfigInvalid, "read a nightly build identity", tag)
            .with_recovery("nightly identity decoding is not implemented"))
    }

    /// The UTC-dated nightly tag, without a curated version number.
    pub fn tag(&self) -> &str {
        &self.tag
    }

    /// The complete source commit used for idempotency and comparison.
    pub fn source_sha(&self) -> &str {
        &self.source_sha
    }
}

#[cfg(test)]
mod tests {
    use super::NightlyBuild;

    const SOURCE: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    const TAG: &str = "nightly-261106-aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

    #[test]
    fn a_nightly_identity_decodes_its_tag_and_complete_source() {
        let read = NightlyBuild::new(TAG, SOURCE)
            .map(|build| (build.tag().to_owned(), build.source_sha().to_owned()));
        assert_eq!(read, Ok((TAG.to_owned(), SOURCE.to_owned())));
    }
}
