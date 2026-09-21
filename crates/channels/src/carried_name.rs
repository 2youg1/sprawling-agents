// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Names of things this crate does not own, on their way through it.
//!
//! A provider, a template, a toolkit slug: each arrives as text and
//! leaves as a type that cannot be empty and cannot carry a control
//! character. That is the whole of what this crate may judge.
//!
//! **The legal value set stays upstream, and only where it is open.**
//! `gateway` owns which providers exist, `city` owns the templates, a
//! broker's directory owns the slugs; each of those grows without this
//! crate hearing about it, so a closed list here would be a second
//! authority that goes stale the moment either side adds one. An
//! unknown value is therefore an error where the authority is, and it
//! says so with the name in hand. A set that is *closed* does not
//! belong here at all: a run's mode is [`kernel::model::Mode`], which
//! refuses an unknown word at this boundary rather than carrying it
//! inward to be guessed at.
//!
//! One macro rather than three hand-written newtypes: they differ in
//! their name and their doc line and in nothing else, and writing the
//! same constructor three times is three places for it to drift.

use kernel::{AxCode, AxError};
use serde::{Deserialize, Serialize};

macro_rules! carried_name {
    ($name:ident, $doc:literal) => {
        #[doc = $doc]
        #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
        #[serde(transparent)]
        #[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
        pub struct $name(String);

        impl $name {
            /// Sole constructor. Rejects empty and control characters only:
            /// the legal value set belongs to the upstream authority, and a
            /// second copy of it here would be a second authority.
            pub fn parse(raw: &str) -> Result<Self, AxError> {
                if raw.is_empty() || raw.chars().any(char::is_control) {
                    return Err(AxError::failure(
                        AxCode::WireMismatch,
                        concat!("read ", stringify!($name), " from a frame"),
                        "the field is empty or holds control characters",
                    )
                    .with_recovery("send a non-empty single-line value"));
                }
                Ok(Self(raw.to_owned()))
            }

            /// The carried text. Whether it names something that exists is
            /// answered upstream, not here.
            #[must_use]
            pub fn as_str(&self) -> &str {
                &self.0
            }
        }
    };
}

carried_name!(
    ProviderName,
    "A provider name in transit. Authority for the provider set is `gateway`."
);
carried_name!(
    TemplateName,
    "A Building template name in transit. Authority is `city`."
);
carried_name!(
    ToolkitSlug,
    "An outside application's id at the broker that connects it. Authority for which ids exist is the broker's directory, and percent-encoding it into a request path belongs to whoever builds that path."
);
#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, reason = "test code")]
mod tests {
    use super::*;

    #[test]
    fn a_carried_name_rejects_empty_and_control_characters() {
        assert!(ProviderName::parse("").is_err());
        assert!(ProviderName::parse("openai\nsteal").is_err());
        assert_eq!(ProviderName::parse("openai").unwrap().as_str(), "openai");
        // No closed list: a provider this build never heard of is the
        // gateway's to refuse, not this boundary's.
        assert!(ProviderName::parse("a-relay-we-have-never-heard-of").is_ok());
    }
}
