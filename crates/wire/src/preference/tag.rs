// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The words a person files sessions under (`crates/wire/spec/Preference.lean`
//! §8-80, wire D18).
//!
//! **A tag is the person's classification, not the city's history.** It
//! changes nothing a run can observe, so it rides in the person's own
//! preferences beside the chords they rebound, and a phone reaching the
//! same city reads the same tags from the same answer.
//!
//! **A session is named the way the ledger files it**: the city, the room,
//! and the line its stretch began at (`SessionLine::began`). The city is
//! part of the name because the person's file is shared by every city
//! they run, and every city's `hall/mayor` begins near the same line.

use kernel::{Address, AxCode, AxError, Seq};
use serde::{Deserialize, Serialize};

/// The longest tag, in characters. A tag is drawn as one chip that does
/// not wrap in the narrow sessions pane; past this it is a sentence, not
/// a class.
pub const TAG_MAX: usize = 24;

/// One word a person files sessions under: one to [`TAG_MAX`] letters
/// of any script, digits, `-` or `_`, and no whitespace, so `/tag <name>`
/// reads exactly one word.
///
/// Case is not part of the grammar. The page folds what a person types
/// to lower case, so `Bug` typed twice is one tag; a tag written by hand
/// into the file in upper case is read as written.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(transparent)]
pub struct Tag(String);

impl Tag {
    /// Sole constructor.
    ///
    /// # Errors
    /// `E_INVALID_ARGS` for an empty word, a word past [`TAG_MAX`]
    /// characters, or a character that is not a letter, a digit, `-` or
    /// `_`.
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

/// The characters a tag may hold. `char::is_alphabetic` is Unicode's
/// `Alphabetic` and `char::is_numeric` is `\p{N}`, which is what lets
/// [`TAG_PATTERN`] state the same set to the client.
fn belongs(c: char) -> bool {
    !c.is_control()
}

impl<'de> Deserialize<'de> for Tag {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let raw = String::deserialize(deserializer)?;
        Tag::parse(&raw).map_err(serde::de::Error::custom)
    }
}

/// [`Tag::parse`]'s grammar as the pattern the client reads it through,
/// counted in code points as the `u` flag counts them.
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
            "description": "One word a person files sessions under: one to 24 letters of any \
                script, digits, `-` or `_`, as `wire::Tag::parse` accepts it.",
            "pattern": TAG_PATTERN,
        })
    }
}

/// One session's tags: the session named by the city, the room and the
/// line its stretch began at, and the tags the person gave it, in
/// lexical order without repeats.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct SessionTags {
    /// The city, as the handshake's `Welcome::city` names it.
    pub city: Address,
    pub room: Address,
    /// The first line of the stretch, as `SessionLine::began` states it.
    pub began: Seq,
    pub tags: Vec<Tag>,
}

impl SessionTags {
    /// Whether two entries name the same session.
    #[must_use]
    pub fn names(&self, other: &SessionTags) -> bool {
        self.city == other.city && self.room == other.room && self.began == other.began
    }
}

/// The list after one session's tags were replaced by `next`'s: an empty
/// set removes the entry, and the list stays in `(city, room, began)`
/// order with each set sorted and without repeats, so two machines that
/// tagged the same sessions in a different order hold the same file.
pub(crate) fn retagged(held: &mut Vec<SessionTags>, next: SessionTags) {
    drop((held, next));
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, reason = "test code")]
mod tests;
