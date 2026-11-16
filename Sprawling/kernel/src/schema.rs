// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::borrow::Cow;

use schemars::{JsonSchema, Schema, SchemaGenerator};
use serde_json::Value;

use crate::ContainerImage;
use crate::address::{Address, SESSION_NAME_MAX, SessionName};
use crate::error::AxCode;
use crate::idem::IdemKey;
use crate::locator::{B3Hash, GitOid, Locator};
use crate::tool::ServerLabel;

fn string_schema(description: &str, pattern: Option<&str>) -> Schema {
    let mut schema = Schema::default();
    schema.insert("type".to_owned(), Value::from("string"));
    schema.insert("description".to_owned(), Value::from(description));
    if let Some(pattern) = pattern {
        schema.insert("pattern".to_owned(), Value::from(pattern));
    }
    schema
}

const ADDRESS_PATTERN: &str = concat!(
    r"^(?:[^/\\:\p{Cc}]*[^/\\:\p{Cc}.\p{White_Space}])",
    r"(?:/[^/\\:\p{Cc}]*[^/\\:\p{Cc}.\p{White_Space}])*$"
);

const RUN_ID_PATTERN: &str = r"^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$";

const SERVER_LABEL_PATTERN: &str = "^[a-z0-9]+$";

fn session_name_pattern() -> String {
    let head = r"^\p{White_Space}*(?:[^\p{White_Space}/\\:\p{Cc}.]|[^\p{White_Space}/\\:\p{Cc}]";
    let longest = SESSION_NAME_MAX.saturating_sub(2);
    let middle = format!(r"[^/\\:\p{{Cc}}]{{0,{longest}}}");
    let tail = r"[^/\\:\p{Cc}.\p{White_Space}])\p{White_Space}*$";
    format!("{head}{middle}{tail}")
}

impl JsonSchema for ContainerImage {
    fn schema_name() -> Cow<'static, str> {
        Cow::Borrowed("ContainerImage")
    }

    fn json_schema(_generator: &mut SchemaGenerator) -> Schema {
        string_schema(
            "A full local immutable image ID admitted by ContainerImage::parse.",
            Some(&format!(
                r"^{}[{}]{{{}}}(?![\s\S])",
                Self::PREFIX,
                Self::HEX_DIGITS,
                Self::DIGEST_LENGTH
            )),
        )
    }
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

impl JsonSchema for ServerLabel {
    fn schema_name() -> Cow<'static, str> {
        Cow::Borrowed("ServerLabel")
    }
    fn json_schema(_generator: &mut SchemaGenerator) -> Schema {
        string_schema(
            "How one external tool server is named inside this city: ascii lowercase letters \
             and digits, at least one, as `kernel::ServerLabel::parse` accepts it.",
            Some(SERVER_LABEL_PATTERN),
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
            "What the User calls one session: one address segment of at most 64 characters, \
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
            "Where content is: `cas:b3-<hash>` or `file:<address>@<oid>`, each with an optional `#L<a>-<b>` or `#B<a>-<b>` range, as `kernel::locator` parses it.",
            None,
        )
    }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    reason = "test code"
)]
mod tests {
    use std::sync::LazyLock;

    use proptest::prelude::*;
    use regex::Regex;

    use super::*;
    use crate::address::RESERVED_PREFIX;
    use crate::address::tests::{Spelling, TABLE, Verdict};
    use crate::event::RunId;

    static ADDRESS_MATCHER: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(ADDRESS_PATTERN).unwrap());
    static RUN_ID_MATCHER: LazyLock<Regex> = LazyLock::new(|| Regex::new(RUN_ID_PATTERN).unwrap());
    static SERVER_LABEL_MATCHER: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(SERVER_LABEL_PATTERN).unwrap());
    static SESSION_NAME_MATCHER: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(&session_name_pattern()).unwrap());

    fn boundary() -> Vec<String> {
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

    fn boundary_text() -> impl Strategy<Value = String> {
        prop::collection::vec(prop::sample::select(boundary()), 0..5)
            .prop_map(|pieces| pieces.concat())
    }

    const RUN_ID_TEXT: &str =
        "([0-9a-f]{8}|[0-9A-F]{8})(-[0-9a-f]{4}|-[0-9A-F]{4}){3}(-[0-9a-f]{12}|[0-9A-F]{12})";

    fn run_id_text() -> impl Strategy<Value = String> {
        prop_oneof![
            3 => RUN_ID_TEXT,
            1 => RUN_ID_TEXT.prop_map(|id| format!("{{{id}}}")),
            1 => RUN_ID_TEXT.prop_map(|id| format!("urn:uuid:{id}")),
            1 => RUN_ID_TEXT.prop_map(|id| id.replace('-', "")),
            2 => "[0-9a-fA-F-]{0,40}",
        ]
    }

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

    fn inside(schema: &Schema, value: &Value) -> bool {
        let pattern = schema.get("pattern").and_then(Value::as_str).unwrap();
        Regex::new(pattern)
            .unwrap()
            .is_match(value.as_str().unwrap())
    }

    fn declaration<T: JsonSchema>() -> String {
        let schema = T::json_schema(&mut SchemaGenerator::default());
        let name = T::schema_name();
        assert!(
            schema.get("description").and_then(Value::as_str).is_some(),
            "{name}: schema description missing"
        );
        let body = match (schema.get("pattern"), schema.get("enum")) {
            (Some(pattern), None) => format!(
                "Schema.String.check(Schema.isPattern(new RegExp({pattern}, \"u\"))).pipe(Schema.brand(\"{name}\"))"
            ),
            (None, Some(Value::Array(codes))) => {
                let codes: Vec<String> = codes.iter().map(Value::to_string).collect();
                format!(
                    "Schema.Literals([{}]).annotate({{ identifier: \"{name}\" }})",
                    codes.join(", ")
                )
            }
            (None, None) => format!("Schema.String.pipe(Schema.brand(\"{name}\"))"),
            other => panic!("{name}: no declaration shape for {other:?}"),
        };
        format!("export const {name} = {body};")
    }

    #[test]
    fn the_committed_client_declares_every_hand_written_schema() {
        let wire_dir = concat!(env!("CARGO_MANIFEST_DIR"), "/../../client/src/wire");
        let mut client = String::new();
        let mut entries: Vec<_> = std::fs::read_dir(wire_dir)
            .unwrap_or_else(|why| panic!("read {wire_dir}: {why}"))
            .map(|entry| entry.expect("wire dir entry").path())
            .filter(|path| path.extension().is_some_and(|ext| ext == "ts"))
            .collect();
        entries.sort();
        for path in entries {
            client.push_str(
                &std::fs::read_to_string(&path)
                    .unwrap_or_else(|why| panic!("read {}: {why}", path.display()))
                    .replace("\r\n", "\n"),
            );
        }
        for declared in [
            declaration::<AxCode>(),
            declaration::<Address>(),
            declaration::<ServerLabel>(),
            declaration::<RunId>(),
            declaration::<SessionName>(),
            declaration::<IdemKey>(),
            declaration::<B3Hash>(),
            declaration::<GitOid>(),
            declaration::<Locator>(),
        ] {
            assert!(
                client.contains(&declared),
                "client/src/wire lacks\n{declared}"
            );
        }
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

    #[test]
    fn the_address_table_gets_the_same_verdict_from_the_pattern() {
        for Spelling { address, verdict } in TABLE.iter() {
            let taken = ADDRESS_MATCHER.is_match(address);
            assert_eq!(taken, *verdict == Verdict::Accepted, "{address:?}");
        }
    }

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
                        assert!(raw.trim().eq_ignore_ascii_case(RESERVED_PREFIX), "{raw:?}");
                    }
                }
            }
        }
    }

    proptest! {

        #[test]
        fn the_address_pattern_answers_what_the_constructor_answers(raw in boundary_text()) {
            prop_assert_eq!(ADDRESS_MATCHER.is_match(&raw), Address::parse(&raw).is_ok());
        }

        #[test]
        fn the_server_label_pattern_answers_what_the_constructor_answers(raw in "[a-zA-Z0-9_ -]{0,6}") {
            prop_assert_eq!(SERVER_LABEL_MATCHER.is_match(&raw), ServerLabel::parse(&raw).is_ok(), "{}", raw);
        }

        #[test]
        fn the_run_id_pattern_answers_what_the_constructor_answers(raw in run_id_text()) {
            prop_assert_eq!(RUN_ID_MATCHER.is_match(&raw), RunId::parse(&raw).is_ok(), "{}", raw);
        }

        #[test]
        fn the_session_name_pattern_parts_from_the_constructor_on_one_word(raw in boundary_text()) {
            let matched = SESSION_NAME_MATCHER.is_match(&raw);
            let parsed = SessionName::parse(&raw).is_ok();
            prop_assert!(matched || !parsed);
            if matched && !parsed {
                prop_assert!(raw.trim().eq_ignore_ascii_case(RESERVED_PREFIX), "{}", raw);
            }
        }
    }
}
