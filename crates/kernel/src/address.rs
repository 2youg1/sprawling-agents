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
//! - `is_reserved` answers C17: a `.sprawling` subtree can never enter a
//!   WriteDomain, at whatever depth it sits, and every WriteDomain
//!   constructor must ask first.

use serde::{Deserialize, Serialize};

use crate::error::{AxCode, AxError};

/// The directory name whose subtree never enters any WriteDomain (C17).
///
/// One of these belongs to each scope that has rules of its own: the
/// city's holds the ledger and the city configuration, a building's
/// holds the rules that govern it. What governs a scope is therefore
/// never writable by what runs inside it.
pub const RESERVED_PREFIX: &str = ".sprawling";

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

    /// C17 primitive: true iff any segment is [`RESERVED_PREFIX`],
    /// compared without ASCII case, because Windows resolves
    /// `.SPRAWLING` to the same directory and this predicate guards the
    /// directory rather than the string. ASCII folding is the whole of
    /// it: the reserved name is ASCII, so a Unicode case table would add
    /// a second, version-dependent authority on the same question.
    ///
    /// Any segment rather than the first: a building's own rules sit at
    /// `<building>/.sprawling/`, and a run whose write domain is its
    /// building must not reach them. Widening this predicate can only
    /// refuse more, which is the direction a fail-closed check may move
    /// in without a second authority to check it against.
    pub fn is_reserved(&self) -> bool {
        self.0
            .split('/')
            .any(|segment| segment.eq_ignore_ascii_case(RESERVED_PREFIX))
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

impl<'de> Deserialize<'de> for Address {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let raw = String::deserialize(deserializer)?;
        Address::parse(&raw).map_err(serde::de::Error::custom)
    }
}

/// The longest word a person may give a session. It becomes a directory
/// name on their file system, and a name longer than this is a sentence
/// that wanted to be a task.
const SESSION_NAME_MAX: usize = 64;

/// What a person calls one session: one address segment, and therefore
/// one directory under a building.
///
/// A `String` here would put the segment rules in whichever caller
/// remembered them. This has one constructor, so a name that cannot be
/// a room cannot be spelled, on the wire or anywhere else.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(transparent)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct SessionName(String);

impl SessionName {
    /// Sole constructor. Trims the ends - a person types a trailing
    /// space and means nothing by it - and then refuses anything that
    /// is not exactly one usable segment.
    ///
    /// # Errors
    /// `E_INVALID_ARGS` for empty input, a separator, a `.` or `..`
    /// segment, the reserved directory name, a control character, or a
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
        if trimmed == RESERVED_PREFIX {
            return reject("that name belongs to the city itself");
        }
        // The segment rules are Address's, asked rather than restated: a
        // second copy of them here would drift from the one that judges
        // the path this name becomes.
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
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "test code"
)]
mod tests {
    use super::*;
    use crate::error::AxCode;

    #[test]
    fn parse_accepts_canonical_relative_paths() {
        for ok in [
            "a",
            "a/b",
            "docs/notes.md",
            "role@building.1/JOB.md",
            ".sprawling/ledger",
        ] {
            let addr = Address::parse(ok).unwrap();
            assert_eq!(addr.as_str(), ok);
            assert_eq!(addr.to_string(), ok);
        }
    }

    #[test]
    fn parse_rejects_every_banned_form() {
        for bad in [
            "",                  // empty
            "/abs",              // absolute
            "a//b",              // empty segment
            "a/",                // trailing slash
            "/",                 // both
            "..",                // parent escape
            "a/../b",            // parent escape inside
            ".",                 // dot segment
            "a/./b",             // dot segment inside
            "a\\b",              // backslash
            "C:/x",              // drive letter (colon)
            "a/b:stream",        // NTFS ADS (colon)
            "a\u{0}b",           // NUL
            "a\tb",              // control character
            ".sprawling.",       // Win32 opens this as `.sprawling`
            ".sprawling ",       // and this too
            "lab/.sprawling./x", // the alias one level down
            "a /b",              // trailing space on an inner segment
            "docs./notes.md",
        ] {
            let err = Address::parse(bad).unwrap_err();
            assert_eq!(err.code(), &AxCode::InvalidArgs, "should reject {bad:?}");
        }
    }

    #[test]
    fn is_within_respects_segment_boundaries() {
        let a = Address::parse("a/b/c").unwrap();
        let prefix = Address::parse("a/b").unwrap();
        let stranger = Address::parse("a/bc").unwrap();
        assert!(a.is_within(&prefix));
        assert!(a.is_within(&a));
        assert!(!stranger.is_within(&prefix));
        assert!(!prefix.is_within(&a));
    }

    #[test]
    fn a_session_name_is_one_segment_a_person_typed() {
        // What it is for: the word a person gives a session becomes the
        // directory that session works in, so anything that is not a
        // single segment is a path they did not mean to write.
        let named = SessionName::parse(" refactor the ledger ").unwrap();
        assert_eq!(
            named.as_str(),
            "refactor the ledger",
            "the ends are trimmed and the middle is theirs"
        );
        for wrong in [
            "",
            "   ",
            "lab/room1",
            ".",
            "..",
            ".sprawling",
            "a\\b",
            "c:name",
        ] {
            let err = SessionName::parse(wrong).unwrap_err();
            assert_eq!(err.code(), &AxCode::InvalidArgs, "{wrong:?} was accepted");
            assert!(!err.recovery().is_empty(), "{wrong:?}");
        }
        // Long enough to be a sentence is long enough to be a mistake:
        // this becomes a directory name on somebody's file system.
        assert!(SessionName::parse(&"x".repeat(65)).is_err());
        assert!(SessionName::parse(&"x".repeat(64)).is_ok());
    }

    #[test]
    fn a_reserved_subtree_is_reserved_at_whatever_depth_it_sits() {
        assert!(Address::parse(".sprawling").unwrap().is_reserved());
        assert!(Address::parse(".sprawling/ledger/x").unwrap().is_reserved());
        assert!(!Address::parse(".sprawlingx/a").unwrap().is_reserved());
        // The rule this card widened. A building keeps what governs it -
        // its rules, its configuration, its own skills - in a reserved
        // subtree of its own, and the run that works in that building has
        // the whole building as its write domain unless BUILDING.md says
        // otherwise. Reserved on the first segment only meant the file
        // declaring the write domain sat inside the write domain.
        assert!(Address::parse("lab/.sprawling").unwrap().is_reserved());
        assert!(
            Address::parse("lab/.sprawling/CONFIG.toml")
                .unwrap()
                .is_reserved()
        );
        assert!(
            Address::parse("lab/room1/.sprawling/CONFIG.toml")
                .unwrap()
                .is_reserved()
        );
        assert!(
            !Address::parse("lab/sprawling/notes.md")
                .unwrap()
                .is_reserved(),
            "a segment that merely looks like it is not it"
        );
        // S-01: Windows opens each of these as the reserved directory, so
        // a byte-wise predicate handed a run a subtree it was refused.
        for alias in [".SPRAWLING", ".Sprawling/ledger", "lab/.SpRaWlInG/C.toml"] {
            assert!(Address::parse(alias).unwrap().is_reserved(), "{alias}");
        }
        let upper = Address::parse(".SPRAWLING").unwrap();
        let lower = Address::parse(".sprawling").unwrap();
        assert_ne!(
            upper, lower,
            "identity stays byte-wise; only the guard folds"
        );
    }

    #[test]
    fn deserialize_revalidates() {
        let ok: Address = serde_json::from_str("\"a/b\"").unwrap();
        assert_eq!(ok.as_str(), "a/b");
        assert!(serde_json::from_str::<Address>("\"../up\"").is_err());
    }

    proptest::proptest! {
        #[test]
        fn accepted_addresses_roundtrip_and_never_contain_banned_parts(
            segs in proptest::collection::vec("[a-zA-Z0-9._@-]{1,8}", 1..5)
        ) {
            let raw = segs.join("/");
            if let Ok(addr) = Address::parse(&raw) {
                proptest::prop_assert_eq!(addr.as_str(), raw.as_str());
                proptest::prop_assert!(!raw.split('/').any(|s| s.is_empty() || s == "." || s == ".."));
                proptest::prop_assert!(
                    !raw.split('/').any(|s| s.ends_with('.') || s.ends_with(char::is_whitespace))
                );
            }
        }

        /// S-01: every spelling a Windows file system resolves to the
        /// reserved directory is refused or judged reserved, whatever its
        /// case, trailing dots and spaces, and depth.
        #[test]
        fn a_windows_alias_of_the_reserved_directory_is_never_writable(
            depth in "(lab/){0,2}",
            name in "[.][sS][pP][rR][aA][wW][lL][iI][nN][gG]",
            trailing in "[. ]{0,3}",
        ) {
            let raw = format!("{depth}{name}{trailing}/CONFIG.toml");
            let slipped = matches!(Address::parse(&raw), Ok(addr) if !addr.is_reserved());
            proptest::prop_assert!(!slipped, "{raw} reaches the reserved subtree");
        }

        #[test]
        fn is_within_agrees_with_string_prefix_plus_boundary(
            a in "[a-z]{1,3}(/[a-z]{1,3}){0,3}",
            b in "[a-z]{1,3}(/[a-z]{1,3}){0,3}",
        ) {
            let x = Address::parse(&a).unwrap();
            let y = Address::parse(&b).unwrap();
            let expect = a == b || a.starts_with(&format!("{b}/"));
            proptest::prop_assert_eq!(x.is_within(&y), expect);
        }
    }
}
