// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The JSON Schema of the five values whose serde is written by hand.
//!
//! Every other value on the wire derives its schema from the declaration
//! serde reads, so the two cannot drift. These five serialise through
//! `Display` and parse through their own grammar, and a derive would
//! describe the struct rather than the string; what a client must know
//! is stated here instead. Each schema is a string, with the pattern
//! where the grammar is one line long, and the code list of `AxCode`
//! taken from the same table that spells the codes (kernel-SPEC.md
//! section 8-45).

use std::borrow::Cow;

use schemars::{JsonSchema, Schema, SchemaGenerator};
use serde_json::Value;

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
    use super::*;

    /// The serialised form of each hand-written value must satisfy the
    /// schema this file states for it, or the client is told one grammar
    /// and sent another.
    #[test]
    fn every_hand_written_value_serialises_inside_its_own_schema() {
        let generator = &mut SchemaGenerator::default();
        let hash = serde_json::to_value(B3Hash::digest(b"x")).unwrap();
        assert_matches_pattern(&B3Hash::json_schema(generator), &hash);
        let oid = serde_json::to_value(GitOid::from_bytes([7u8; 20])).unwrap();
        assert_matches_pattern(&GitOid::json_schema(generator), &oid);
        let key = serde_json::to_value(IdemKey::derive(
            &crate::event::RunId::from_bytes([1u8; 16]),
            crate::event::Seq::FIRST,
            b"act",
        ))
        .unwrap();
        assert_matches_pattern(&IdemKey::json_schema(generator), &key);
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

    /// The patterns are fixed-width hex, so matching them needs no regex
    /// engine: one prefix, one width, one alphabet.
    fn assert_matches_pattern(schema: &Schema, value: &Value) {
        let pattern = schema.get("pattern").and_then(|p| p.as_str()).unwrap();
        let text = value.as_str().unwrap();
        let (prefix, rest) = pattern
            .strip_prefix('^')
            .unwrap()
            .split_once("[0-9a-f]{")
            .unwrap();
        let width: usize = rest.strip_suffix("}$").unwrap().parse().unwrap();
        let hex = text.strip_prefix(prefix).unwrap();
        assert_eq!(hex.len(), width, "{text} against {pattern}");
        assert!(
            hex.chars()
                .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase())
        );
    }
}
