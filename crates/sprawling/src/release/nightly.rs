// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Nightly snapshot identity (`crates/sprawling/spec/Nightly.lean` D105).

use kernel::{AxCode, AxError};

fn refused(tag: &str) -> AxError {
    AxError::failure(AxCode::ConfigInvalid, "read a nightly build identity", tag).with_recovery(
        "use nightly-YYMMDD-fullSHA, with a real month and day and the same complete \
             40-character lowercase hexadecimal SHA in the tag and target_commitish",
    )
}

fn is_lower_hex(text: &str) -> bool {
    text.bytes()
        .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

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
    pub fn new(tag: &str, source_sha: &str) -> Result<NightlyBuild, AxError> {
        let body = tag.strip_prefix("nightly-").ok_or_else(|| refused(tag))?;
        let (date, commit) = body.split_once('-').ok_or_else(|| refused(tag))?;
        if date.len() != 6 || !date.bytes().all(|byte| byte.is_ascii_digit()) {
            return Err(refused(tag));
        }
        let number = |range: std::ops::Range<usize>| {
            date.get(range)
                .and_then(|part| part.parse::<u32>().ok())
                .ok_or_else(|| refused(tag))
        };
        if !(1..=12).contains(&number(2..4)?) || !(1..=31).contains(&number(4..6)?) {
            return Err(refused(tag));
        }
        if source_sha.len() != 40 || !is_lower_hex(source_sha) || commit != source_sha {
            return Err(refused(tag));
        }
        Ok(NightlyBuild {
            tag: tag.to_owned(),
            source_sha: source_sha.to_owned(),
        })
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

    #[test]
    fn malformed_identities_are_refused() {
        for (tag, full) in [
            ("v0.0.15-Alpha-261102", SOURCE),
            ("nightly-261300-bc930372", SOURCE),
            ("nightly-261132-bc930372", SOURCE),
            ("nightly-261106-bc93037", SOURCE),
            (
                "nightly-261106-AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA",
                SOURCE,
            ),
            (TAG, "bc930372"),
            (TAG, "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"),
        ] {
            assert!(
                NightlyBuild::new(tag, full).is_err(),
                "accepted {tag} / {full}"
            );
        }
    }
}
