// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The JSON Schema of the values a derive cannot state.
//!
//! Every other value on the wire derives its schema from the declaration
//! serde reads, so the two cannot drift. These are the strings a grammar
//! judges: `Address`, `RunId` and `SessionName` serialise through
//! `Display` and read through their own constructor, so a derive would
//! say a field is a string and nothing about what it must say. Each
//! schema states that grammar as a pattern, which `cargo xtask wire-ts`
//! turns into the client's own check (kernel-SPEC.md section 8-45).
//!
//! **The pattern is not the authority.** `Address::parse` answers which
//! rule a string broke, which is what a person can act on; two
//! statements of one grammar drift unless something binds them, so the
//! tests below bind every pattern to its constructor over input.

use std::borrow::Cow;

use schemars::{JsonSchema, Schema, SchemaGenerator};
use serde_json::Value;

use crate::address::{Address, SESSION_NAME_MAX, SessionName};
use crate::error::AxCode;
use crate::idem::IdemKey;
use crate::locator::{B3Hash, GitOid, Locator};

/// A string schema with a description and, where the grammar is one line
/// long, the pattern that states it.
fn string_schema(description: &str, pattern: Option<&str>) -> Schema {
    let mut schema = Schema::default();
    schema.insert("type".to_owned(), Value::from("string"));
    schema.insert("description".to_owned(), Value::from(description));
    if let Some(pattern) = pattern {
        schema.insert("pattern".to_owned(), Value::from(pattern));
    }
    schema
}

/// The grammar [`Address::parse`] enforces: one segment, then further
/// segments after a `/`; each at least one character, none of them `/`,
/// `\`, `:` or control, and the last neither a dot nor whitespace, which
/// is also what refuses `.` and `..`. The two properties are the ones the
/// constructor asks: `\p{Cc}` is `char::is_control` and `\p{White_Space}`
/// is `char::is_whitespace`, so every reader applies this with Unicode
/// semantics - the `u` flag or its equivalent.
const ADDRESS_PATTERN: &str = concat!(
    r"^(?:[^/\\:\p{Cc}]*[^/\\:\p{Cc}.\p{White_Space}])",
    r"(?:/[^/\\:\p{Cc}]*[^/\\:\p{Cc}.\p{White_Space}])*$"
);

/// The grammar [`RunId::parse`](crate::event::RunId::parse) enforces: the
/// one spelling this city writes a run id in, a hyphenated uuid in lower
/// case. `uuid::Uuid::parse_str` also reads bare hex, braces, the
/// `urn:uuid:` prefix and upper case; a city that took those would name
/// runs in identities its own reader calls unknown.
const RUN_ID_PATTERN: &str = r"^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$";

/// The grammar [`SessionName::parse`] enforces, as a pattern: the
/// whitespace the constructor trims from either end, one address segment
/// of one to [`SESSION_NAME_MAX`] characters that ends in neither a dot
/// nor whitespace, and the whitespace it trims.
///
/// **One word parts the pattern from the constructor.** `.sprawling` is
/// the reserved directory, and refusing one word among all words needs
/// look-around, which neither the Rust engine nor the ECMAScript the
/// client runs has. The pattern takes that name and
/// `SessionName::parse` refuses it, so the client may offer a name the
/// city will decline and may never fail to read one the city wrote; the
/// tests here hold the disagreement to that single word.
///
/// The cap is read from [`SESSION_NAME_MAX`] rather than repeated: the
/// head and the tail of the body carry one character each, and the
/// repetition between them carries the rest.
fn session_name_pattern() -> String {
    let head = r"^\p{White_Space}*(?:[^\p{White_Space}/\\:\p{Cc}.]|[^\p{White_Space}/\\:\p{Cc}]";
    let longest = SESSION_NAME_MAX.saturating_sub(2);
    let middle = format!(r"[^/\\:\p{{Cc}}]{{0,{longest}}}");
    let tail = r"[^/\\:\p{Cc}.\p{White_Space}])\p{White_Space}*$";
    format!("{head}{middle}{tail}")
}

impl JsonSchema for AxCode {
    fn schema_name() -> Cow<'static, str> {
        Cow::Borrowed("AxCode")
    }
    fn json_schema(_generator: &mut SchemaGenerator) -> Schema {
        let codes: Vec<Value> = AxCode::ALL
            .iter()
            .map(|code| Value::from(code.as_str()))
            .collect();
        let mut schema = string_schema(
            "One of the closed set of error codes, as `AxCode::as_str` spells it.",
            None,
        );
        schema.insert("enum".to_owned(), Value::Array(codes));
        schema
    }
}

impl JsonSchema for Address {
    fn schema_name() -> Cow<'static, str> {
        Cow::Borrowed("Address")
    }
    fn json_schema(_generator: &mut SchemaGenerator) -> Schema {
        string_schema(
            "A canonical relative path inside the city: `/`-separated segments, none empty, \
             none `.` or `..`, no backslash, no `:`, no control character, and no segment \
             ending in a dot or whitespace, as `kernel::Address::parse` accepts it.",
            Some(ADDRESS_PATTERN),
        )
    }
}

impl JsonSchema for crate::event::RunId {
    fn schema_name() -> Cow<'static, str> {
        Cow::Borrowed("RunId")
    }
    fn json_schema(_generator: &mut SchemaGenerator) -> Schema {
        string_schema(
            "A run's identity in the one spelling the city writes it: a hyphenated uuid in \
             lower case, as `kernel::RunId::parse` accepts it. Bare hex, braces, the \
             `urn:uuid:` prefix and upper case are refused.",
            Some(RUN_ID_PATTERN),
        )
    }
}

impl JsonSchema for SessionName {
    fn schema_name() -> Cow<'static, str> {
        Cow::Borrowed("SessionName")
    }
    fn json_schema(_generator: &mut SchemaGenerator) -> Schema {
        let pattern = session_name_pattern();
        string_schema(
            "What a person calls one session: one address segment of at most 64 characters, \
             with the whitespace at either end trimmed, as `kernel::address::SessionName::parse` \
             accepts it. The city refuses the reserved directory's own name as well.",
            Some(&pattern),
        )
    }
}

impl JsonSchema for IdemKey {
    fn schema_name() -> Cow<'static, str> {
        Cow::Borrowed("IdemKey")
    }
    fn json_schema(_generator: &mut SchemaGenerator) -> Schema {
        string_schema(
            "The deduplication key of one outward action: `idem1-` then 32 lowercase hex digits.",
            Some("^idem1-[0-9a-f]{32}$"),
        )
    }
}

impl JsonSchema for B3Hash {
    fn schema_name() -> Cow<'static, str> {
        Cow::Borrowed("B3Hash")
    }

    fn json_schema(_generator: &mut SchemaGenerator) -> Schema {
        string_schema(
            "A BLAKE3 digest: exactly 64 lowercase hex digits.",
            Some("^[0-9a-f]{64}$"),
        )
    }
}

impl JsonSchema for GitOid {
    fn schema_name() -> Cow<'static, str> {
        Cow::Borrowed("GitOid")
    }
    fn json_schema(_generator: &mut SchemaGenerator) -> Schema {
        string_schema(
            "A git object id: exactly 40 lowercase hex digits.",
            Some("^[0-9a-f]{40}$"),
        )
    }
}

impl JsonSchema for Locator {
    fn schema_name() -> Cow<'static, str> {
        Cow::Borrowed("Locator")
    }
    fn json_schema(_generator: &mut SchemaGenerator) -> Schema {
        string_schema(
            "Where content is: `cas:b3-<hash>` or `file:<address>@<oid>`, each with an              optional `#L<a>-<b>` or `#B<a>-<b>` range, as `kernel::locator` parses it.",
            None,
        )
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, reason = "test code")]
mod tests {
    use std::sync::LazyLock;

    use proptest::prelude::*;
    use regex::Regex;

    use super::*;
    use crate::address::RESERVED_PREFIX;
    use crate::address::tests::{ACCEPTED, REFUSED};
    use crate::event::RunId;

    /// Each pattern compiled the way every reader reads it: `\p{...}` is a
    /// Unicode property, so the matcher needs Unicode semantics.
    static ADDRESS_MATCHER: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(ADDRESS_PATTERN).unwrap());
    static RUN_ID_MATCHER: LazyLock<Regex> = LazyLock::new(|| Regex::new(RUN_ID_PATTERN).unwrap());
    static SESSION_NAME_MATCHER: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(&session_name_pattern()).unwrap());

    /// Where a boundary sits: nothing, the words and dots a segment is
    /// made of, both reserved spellings, a separator, every kind of
    /// whitespace and non-ASCII text, and the lengths either side of the
    /// cap. One list, read by the sweep and by the generator.
    fn boundary() -> Vec<String> {
        // The words, separated by a character none of them carries.
        let mut pieces: Vec<String> = "a|ab|notes.md|.|..|.sprawling|.SPRAWLING|x."
            .split('|')
            .map(str::to_owned)
            .collect();
        pieces.insert(0, String::new());
        for glyph in "\u{0}\t \u{85}\u{a0}\u{200b}\u{feff}/\\:é中🦀".chars() {
            pieces.push(glyph.to_string());
        }
        for length in [61, 62, 63, 64, 65, 70] {
            pieces.push("x".repeat(length));
        }
        pieces
    }

    /// Strings assembled from those pieces, so what a longer string hides
    /// between two of them is explored as well.
    fn boundary_text() -> impl Strategy<Value = String> {
        prop::collection::vec(prop::sample::select(boundary()), 0..5)
            .prop_map(|pieces| pieces.concat())
    }

    /// A run id whose every group is spelled in one case, which is what
    /// makes the property ask about one group alone.
    const RUN_ID_TEXT: &str =
        "([0-9a-f]{8}|[0-9A-F]{8})(-[0-9a-f]{4}|-[0-9A-F]{4}){3}(-[0-9a-f]{12}|[0-9A-F]{12})";

    /// A run id: the city's spelling with each group in either case, or one
    /// of the spellings `uuid` reads beyond it - braces, the `urn:uuid:`
    /// prefix, the bare 32 digits - or loose text from the same alphabet.
    ///
    /// Either case is what makes this bite: `uuid` reads a capital and the
    /// city writes lower, so a pattern that took the other case in one
    /// group alone is asked about that group.
    fn run_id_text() -> impl Strategy<Value = String> {
        prop_oneof![
            3 => RUN_ID_TEXT,
            1 => RUN_ID_TEXT.prop_map(|id| format!("{{{id}}}")),
            1 => RUN_ID_TEXT.prop_map(|id| format!("urn:uuid:{id}")),
            1 => RUN_ID_TEXT.prop_map(|id| id.replace('-', "")),
            2 => "[0-9a-fA-F-]{0,40}",
        ]
    }

    /// The serialised form of each hand-written value must satisfy the
    /// schema this file states for it, or the client is told one grammar
    /// and sent another.
    #[test]
    fn every_hand_written_value_serialises_inside_its_own_schema() {
        let generator = &mut SchemaGenerator::default();
        let run = serde_json::to_value(RunId::CITY).unwrap();
        assert!(inside(&RunId::json_schema(generator), &run), "{run}");
        let name = serde_json::to_value(SessionName::parse("ship it").unwrap()).unwrap();
        let spelled = SessionName::json_schema(generator);
        assert!(inside(&spelled, &name), "{name}");
        let hash = serde_json::to_value(B3Hash::digest(b"x")).unwrap();
        assert!(inside(&B3Hash::json_schema(generator), &hash), "{hash}");
        let oid = serde_json::to_value(GitOid::from_bytes([7u8; 20])).unwrap();
        assert!(inside(&GitOid::json_schema(generator), &oid), "{oid}");
        let key = serde_json::to_value(IdemKey::derive(
            &RunId::from_bytes([1u8; 16]),
            crate::event::Seq::FIRST,
            b"act",
        ))
        .unwrap();
        assert!(inside(&IdemKey::json_schema(generator), &key), "{key}");
    }

    /// The pattern a schema carries, applied to a serialised value.
    fn inside(schema: &Schema, value: &Value) -> bool {
        let pattern = schema.get("pattern").and_then(Value::as_str).unwrap();
        Regex::new(pattern)
            .unwrap()
            .is_match(value.as_str().unwrap())
    }

    #[test]
    fn the_code_list_is_the_whole_table_in_table_order() {
        let schema = AxCode::json_schema(&mut SchemaGenerator::default());
        let listed: Vec<&str> = schema
            .get("enum")
            .and_then(Value::as_array)
            .unwrap()
            .iter()
            .map(|v| v.as_str().unwrap())
            .collect();
        let table: Vec<&str> = AxCode::ALL.iter().map(AxCode::as_str).collect();
        assert_eq!(listed, table);
    }

    /// Both `\p{...}` classes are the predicates the constructors ask, so
    /// the pattern and the generator mean the same thing by "control" and
    /// "whitespace" rather than two tables drifting apart. Every Unicode
    /// scalar is asked: the exceptions are exactly the characters no
    /// hand-written list would hold.
    #[test]
    fn the_two_unicode_classes_are_the_char_predicates() {
        let control = Regex::new(r"\p{Cc}").unwrap();
        let space = Regex::new(r"\p{White_Space}").unwrap();
        for code in 0..=0x10_FFFF {
            let Some(glyph) = char::from_u32(code) else {
                continue;
            };
            let text = glyph.to_string();
            assert_eq!(control.is_match(&text), glyph.is_control(), "{code:#x}");
            assert_eq!(space.is_match(&text), glyph.is_whitespace(), "{code:#x}");
        }
    }

    /// The spellings the constructor's own tests use, judged by the pattern
    /// the client is handed: a case added on one side is a case both
    /// readers answer.
    #[test]
    fn the_address_table_gets_the_same_verdict_from_the_pattern() {
        for accepted in ACCEPTED {
            assert!(ADDRESS_MATCHER.is_match(accepted), "refused {accepted:?}");
        }
        for refused in REFUSED {
            assert!(!ADDRESS_MATCHER.is_match(refused), "took {refused:?}");
        }
    }

    /// Three pieces at once, every combination there is: a sweep rather
    /// than a sample, so a pattern that parts from its constructor is found
    /// by walking where the two can differ.
    #[test]
    fn the_patterns_and_the_constructors_agree_at_every_boundary() {
        for first in boundary() {
            for second in boundary() {
                for third in boundary() {
                    let raw = format!("{first}{second}{third}");
                    assert_eq!(
                        ADDRESS_MATCHER.is_match(&raw),
                        Address::parse(&raw).is_ok(),
                        "address {raw:?}"
                    );
                    let matched = SESSION_NAME_MATCHER.is_match(&raw);
                    let parsed = SessionName::parse(&raw).is_ok();
                    assert!(matched || !parsed, "session name {raw:?}");
                    if matched && !parsed {
                        assert_eq!(raw.trim(), RESERVED_PREFIX, "{raw:?}");
                    }
                }
            }
        }
    }

    proptest! {
        /// One grammar, two statements. A pattern stricter than
        /// `Address::parse` turns a record the city wrote into a decode
        /// failure in the page, which a person cannot act on.
        #[test]
        fn the_address_pattern_answers_what_the_constructor_answers(raw in boundary_text()) {
            prop_assert_eq!(ADDRESS_MATCHER.is_match(&raw), Address::parse(&raw).is_ok());
        }

        /// The same question for run identity: the city takes the one
        /// spelling it writes, and the wire says so.
        #[test]
        fn the_run_id_pattern_answers_what_the_constructor_answers(raw in run_id_text()) {
            prop_assert_eq!(RUN_ID_MATCHER.is_match(&raw), RunId::parse(&raw).is_ok(), "{}", raw);
        }

        /// `SessionName::parse` refuses one word the pattern cannot: the
        /// reserved directory's own name. Every other disagreement is a
        /// defect, and the assertion inside says which one remains.
        #[test]
        fn the_session_name_pattern_parts_from_the_constructor_on_one_word(raw in boundary_text()) {
            let matched = SESSION_NAME_MATCHER.is_match(&raw);
            let parsed = SessionName::parse(&raw).is_ok();
            prop_assert!(matched || !parsed);
            if matched && !parsed {
                prop_assert_eq!(raw.trim(), RESERVED_PREFIX);
            }
        }
    }
}
