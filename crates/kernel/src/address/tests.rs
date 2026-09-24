// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Fixtures for the address grammar (kernel-SPEC 8-2).

use super::*;

/// The spellings the grammar must take. `crate::schema` applies the
/// pattern it hands the client to these same spells, so a case added
/// here is judged by both readers.
pub(crate) const ACCEPTED: [&str; 5] = [
    "a",
    "a/b",
    "docs/notes.md",
    "role@building.1/JOB.md",
    ".sprawling/ledger",
];

/// What it must refuse, one spelling per rule.
pub(crate) const REFUSED: [&str; 19] = [
    "",                  // empty
    "/abs",              // absolute
    "a//b",              // empty segment
    "a/",                // trailing separator
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
];

#[test]
fn every_spelling_in_the_table_gets_its_verdict() {
    for ok in ACCEPTED {
        let addr = Address::parse(ok).unwrap();
        assert_eq!(addr.as_str(), ok);
        assert_eq!(addr.to_string(), ok);
    }
    for bad in REFUSED {
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
        ".git",
        ".GIT",
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
    // the whole building as its write domain unless RULES.toml says
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
fn protected_metadata_is_reserved_at_whatever_depth_it_sits() {
    // Writing `.git` is privilege escalation: a hook is code that runs
    // at the next git operation, the config keys execute programs,
    // and the fence references and the restoration objects live in
    // there too. One predicate guards both names, at any depth.
    for reserved in [
        ".git",
        ".git/hooks/pre-run",
        "lab/.git/config",
        ".GIT/x",
        "lab/.GiT/y",
    ] {
        assert!(
            Address::parse(reserved).unwrap().is_reserved(),
            "{reserved}"
        );
    }
    for open in [
        "git/config",
        "lab/github/notes.md",
        "a.git/x",
        "git.md",
        ".gits/x",
        "lab/.gitlab/notes.md",
    ] {
        assert!(
            !Address::parse(open).unwrap().is_reserved(),
            "{open} is a name that merely looks like it"
        );
    }
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

    /// S-01: every spelling a Windows file system resolves to a
    /// protected metadata directory is refused or judged reserved,
    /// whatever its case, trailing dots and spaces, and depth. The
    /// two names are one list, so the same property covers both.
    #[test]
    fn a_windows_alias_of_protected_metadata_is_never_writable(
        depth in "(lab/){0,2}",
        name in "[.]([sS][pP][rR][aA][wW][lL][iI][nN][gG]|[gG][iI][tT])",
        trailing in "[. ]{0,3}",
    ) {
        let raw = format!("{depth}{name}{trailing}/CONFIG.toml");
        let slipped = matches!(Address::parse(&raw), Ok(addr) if !addr.is_reserved());
        proptest::prop_assert!(!slipped, "{raw} reaches a protected subtree");
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
