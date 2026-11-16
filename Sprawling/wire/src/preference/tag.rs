// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use kernel::{Address, AxCode, AxError, Seq};
use serde::{Deserialize, Serialize};

pub const TAG_MAX: usize = 24;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(transparent)]
pub struct Tag(String);

impl Tag {
    pub fn parse(raw: &str) -> Result<Self, AxError> {
        let length = raw.chars().count();
        if length == 0 || length > TAG_MAX || !raw.chars().all(belongs) {
            return Err(
                AxError::failure(AxCode::InvalidArgs, "tag a session", raw).with_recovery(
                    "give one word of at most 24 letters, digits, `-` or `_`, with no spaces",
                ),
            );
        }
        Ok(Tag(raw.to_owned()))
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

fn belongs(c: char) -> bool {
    c == '-' || c == '_' || c.is_alphabetic() || c.is_numeric()
}

impl<'de> Deserialize<'de> for Tag {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let raw = String::deserialize(deserializer)?;
        Tag::parse(&raw).map_err(serde::de::Error::custom)
    }
}

#[cfg(feature = "schema")]
const TAG_PATTERN: &str = r"^[-_\p{Alphabetic}\p{N}]{1,24}$";

#[cfg(feature = "schema")]
impl schemars::JsonSchema for Tag {
    fn schema_name() -> std::borrow::Cow<'static, str> {
        std::borrow::Cow::Borrowed("Tag")
    }
    fn json_schema(_generator: &mut schemars::SchemaGenerator) -> schemars::Schema {
        schemars::json_schema!({
            "type": "string",
            "description": "One word the User files sessions under: one to 24 letters of any \
                script, digits, `-` or `_`, as `wire::Tag::parse` accepts it.",
            "pattern": TAG_PATTERN,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct SessionTags {
    pub city: Address,
    pub room: Address,

    pub began: Seq,
    pub tags: Vec<Tag>,
}

impl SessionTags {
    #[must_use]
    pub fn names(&self, other: &SessionTags) -> bool {
        self.city == other.city && self.room == other.room && self.began == other.began
    }
}

pub(crate) fn retagged(held: &mut Vec<SessionTags>, next: SessionTags) {
    held.retain(|entry| !entry.names(&next));
    let mut next = next;
    next.tags.sort();
    next.tags.dedup();
    if !next.tags.is_empty() {
        held.push(next);
        held.sort_by(|one, two| {
            (&one.city, &one.room, one.began).cmp(&(&two.city, &two.room, two.began))
        });
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, reason = "test code")]
mod tests;
