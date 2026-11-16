// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

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

        assert!(ProviderName::parse("a-relay-we-have-never-heard-of").is_ok());
    }
}
