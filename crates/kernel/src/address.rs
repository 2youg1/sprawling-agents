// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Address: canonical relative path inside a city, and
//! the WriteDomain primitive `is_within`.
//!
//! Invariants owned here:
//! - one constructor: [`Address::parse`]; whatever it accepts is already
//!   canonical (no normalization happens, non-canonical spellings are
//!   rejected), so `as_str` is a byte-exact round trip.
//! - identity is byte-wise on every platform: `Eq`, `Ord` and `is_within`
//!   never fold case; `is_reserved` alone does, and it only refuses more.
//! - symlink resolution is an effect-layer concern; this type only judges
//!   already-canonical relative paths.
//! - `is_reserved` answers C17: a protected-metadata subtree
//!   ([`PROTECTED_METADATA`]) can never enter a WriteDomain, at whatever
//!   depth it sits, and every WriteDomain constructor must ask first.
//! - [`may_read`] answers the read bound: what a resident of one
//!   building may read by a path it chose (city-SPEC 8-2).

use serde::{Deserialize, Serialize};

use crate::error::{AxCode, AxError};

/// The directory name whose subtree never enters any WriteDomain (C17).
///
/// One of these belongs to each scope that has rules of its own: the
/// city's holds the ledger and the city configuration, a building's
/// holds the rules that govern it. What governs a scope is therefore
/// never writable by what runs inside it.
pub const RESERVED_PREFIX: &str = ".sprawling";

/// git's own metadata directory (kernel-SPEC 8-73). Writing it is
/// privilege escalation: a hook is code that runs at the next git
/// operation, a config key can start a program, and the fence
/// references and the restoration objects a `file_discarded` names all
/// live in there.
pub const GIT_METADATA: &str = ".git";

/// Every directory name that is protected metadata, and therefore
/// reserved at whatever depth it sits: what governs a scope, and what
/// keeps a repository itself. The one list - [`Address::is_reserved`]
/// judges it, and no caller re-spells these names.
pub const PROTECTED_METADATA: [&str; 2] = [RESERVED_PREFIX, GIT_METADATA];

/// Canonical relative path; invariants enforced at the sole constructor.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(transparent)]
pub struct Address(String);

impl Address {
    /// Sole constructor; `crate::schema` restates this grammar as one
    /// pattern for clients. Rejects: empty input, absolute paths,
    /// backslash, `:` (drive letters and NTFS streams alike), control
    /// characters, empty segments, `.`/`..`, and a segment ending in a
    /// dot or whitespace. Fail-closed: not canonical is `E_INVALID_ARGS`.
    pub fn parse(raw: &str) -> Result<Self, AxError> {
        let reject = |violation: &str| {
            Err(
                AxError::failure(AxCode::InvalidArgs, "parse address", raw).with_recovery(format!(
                    "{violation}; use a canonical relative path: `/`-separated \
                     segments without `.`/`..`, backslash, `:`, or control characters"
                )),
            )
        };
        if raw.is_empty() {
            return reject("address is empty");
        }
        if raw.starts_with('/') {
            return reject("address is absolute");
        }
        if raw.contains('\\') {
            return reject("address contains a backslash");
        }
        if raw.contains(':') {
            return reject("address contains `:`");
        }
        if raw.chars().any(char::is_control) {
            return reject("address contains a control character");
        }
        for segment in raw.split('/') {
            if segment.is_empty() {
                return reject("address contains an empty segment");
            }
            if segment == "." || segment == ".." {
                return reject("address contains a `.` or `..` segment");
            }
            // Win32 strips trailing dots and spaces from a path component
            // before opening the file, so `.sprawling.` opens `.sprawling`.
            // Refusing the alias here spares every later reader, and
            // `is_reserved` above all, from knowing that rule.
            if segment.ends_with('.') || segment.ends_with(char::is_whitespace) {
                return reject(
                    "a segment ends with a dot or whitespace, which some file systems drop, \
                     making this address a second spelling of a different one",
                );
            }
        }
        Ok(Address(raw.to_owned()))
    }

    /// WriteDomain primitive: true iff `self` equals `prefix` or lies under
    /// it on a segment boundary. Byte-wise, reflexive, transitive.
    pub fn is_within(&self, prefix: &Address) -> bool {
        match self.0.strip_prefix(&prefix.0) {
            Some(rest) => rest.is_empty() || rest.starts_with('/'),
            None => false,
        }
    }

    /// C17 primitive: true iff any segment names protected metadata
    /// ([`PROTECTED_METADATA`]), compared without ASCII case, because
    /// Windows resolves `.SPRAWLING` to the same directory and this
    /// predicate guards the directory rather than the string. ASCII
    /// folding is the whole of it: the names are ASCII, so a Unicode
    /// case table would add a second, version-dependent authority on
    /// the same question.
    ///
    /// Any segment rather than the first: a building's own rules sit at
    /// `<building>/.sprawling/`, and a run whose write domain is its
    /// building must not reach them. Widening this predicate can only
    /// refuse more, which is the direction a fail-closed check may move
    /// in without a second authority to check it against (kernel-SPEC
    /// 8-73).
    pub fn is_reserved(&self) -> bool {
        self.0.split('/').any(|segment| {
            PROTECTED_METADATA
                .iter()
                .any(|name| segment.eq_ignore_ascii_case(name))
        })
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for Address {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// What the read bound answers about one address.
///
/// Two of the three arms close, and they stay two arms because a model
/// refused by each has a different next step: ask somebody inside the
/// confidential building, or wait for a person to fix rules that do not
/// read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReadVerdict {
    /// The reader's own building, or a building whose rules say
    /// `confidential = false`.
    Open,
    /// A confidential building, asked about from outside it.
    Confidential,
    /// The rules that would answer did not read, so the building is
    /// closed: what they would have said may be `confidential = true`,
    /// and a privacy setting does not fail towards the permissive side.
    RulesUnreadable(AxError),
}

/// The read bound: whether a resident of `reader_building` may read
/// `target`. Its own building is open whole; any other is open unless
/// its rules say it is confidential or cannot be read.
///
/// `rules` answers whether the building holding `target` is
/// confidential, and is called only when `target` is outside the
/// reader's building. It is a closure rather than a value because a read
/// inside one's own building is the common case and should cost no
/// disk, and because which building holds an address and what its rules
/// say are the city crate's to answer.
pub fn may_read(
    reader_building: &Address,
    target: &Address,
    rules: impl FnOnce() -> Result<bool, AxError>,
) -> ReadVerdict {
    if target.is_within(reader_building) {
        return ReadVerdict::Open;
    }
    match rules() {
        Ok(false) => ReadVerdict::Open,
        Ok(true) => ReadVerdict::Confidential,
        Err(unread) => ReadVerdict::RulesUnreadable(unread),
    }
}

impl<'de> Deserialize<'de> for Address {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let raw = String::deserialize(deserializer)?;
        Address::parse(&raw).map_err(serde::de::Error::custom)
    }
}

/// The longest word a person may give a session. It becomes a directory
/// name on their file system, and a name longer than this is a sentence
/// that wanted to be a task. `crate::schema` states the cap in the
/// pattern it hands the client from this number.
pub(crate) const SESSION_NAME_MAX: usize = 64;

/// What a person calls one session: one address segment, and therefore
/// one directory under a building.
///
/// A `String` here would put the segment rules in whichever caller
/// remembered them. This has one constructor, so a name that cannot be
/// a room cannot be spelled, on the wire or anywhere else.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(transparent)]
pub struct SessionName(String);

impl SessionName {
    /// Sole constructor. Trims the ends - a person types a trailing
    /// space and means nothing by it - and then refuses anything that
    /// is not exactly one usable segment.
    ///
    /// # Errors
    /// `E_INVALID_ARGS` for empty input, a separator, a `.` or `..`
    /// segment, a protected metadata name, a control character, or a
    /// name past [`SESSION_NAME_MAX`].
    pub fn parse(raw: &str) -> Result<Self, AxError> {
        let trimmed = raw.trim();
        let reject = |violation: &str| {
            Err(
                AxError::failure(AxCode::InvalidArgs, "name a session", raw).with_recovery(
                    format!(
                        "{violation}; give one word or phrase this session can be found by - it \
                         becomes a folder beside the others in that building"
                    ),
                ),
            )
        };
        if trimmed.is_empty() {
            return reject("a session with no name has no folder to work in");
        }
        if trimmed.chars().count() > SESSION_NAME_MAX {
            return reject("that is longer than a name and shorter than a task");
        }
        if trimmed == "." || trimmed == ".." {
            return reject("that names a directory rather than a session");
        }
        // The name and the segment rules are Address's, asked rather
        // than restated: a second copy of them here would drift from the
        // one that judges the path this name becomes. Asking the
        // predicate rather than comparing strings is what also refuses
        // `.GIT`, the Windows alias of a protected name.
        if matches!(Address::parse(trimmed), Ok(addr) if addr.is_reserved()) {
            return reject("that name belongs to the city itself");
        }
        if trimmed.contains('/') || Address::parse(trimmed).is_err() {
            return reject(
                "a session name is one segment that stands on its own: no `/`, `\\`, `:`, \
                 control characters, and no trailing dot",
            );
        }
        Ok(SessionName(trimmed.to_owned()))
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for SessionName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl<'de> Deserialize<'de> for SessionName {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let raw = String::deserialize(deserializer)?;
        SessionName::parse(&raw).map_err(serde::de::Error::custom)
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, reason = "test code")]
pub(crate) mod tests;
