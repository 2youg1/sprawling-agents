// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! What the emitter is held to: a named bare string is branded, an
//! externally tagged enum is a union, a definition comes after what it
//! refers to, anything outside the subset is refused by name, and the
//! real wire comes out whole.

use serde_json::{Value, json};

use super::emit::emit;
use super::first_difference;

fn document(defs: Value) -> Value {
    json!({ "$defs": defs })
}

fn emitted(defs: Value) -> String {
    emit(&document(defs), 13, "ab12").unwrap_or_else(|refused| {
        panic!("refused at {}: {}", refused.at, refused.why);
    })
}

#[test]
fn a_named_bare_string_is_a_branded_string_and_an_integer_a_branded_int() {
    let text = emitted(json!({
        "Address": { "type": "string", "description": "A path." },
        "Seq": { "type": "integer", "format": "uint64", "minimum": 0 },
    }));
    assert!(text.contains("/**\n * A path.\n */\n"), "{text}");
    assert!(
        text.contains("export const Address = Schema.String.pipe(Schema.brand(\"Address\"));\n")
    );
    assert!(text.contains("export type Address = typeof Address.Type;\n"));
    assert!(text.contains("export const Seq = Schema.Int.pipe(Schema.brand(\"Seq\"));\n"));
    assert!(text.contains("export const WIRE_V = 13 as const;\n"));
    assert!(text.contains("export const WIRE_HASH = \"ab12\" as const;\n"));
}

#[test]
fn an_externally_tagged_enum_is_a_union_of_literals_and_structs() {
    let text = emitted(json!({
        "Seq": { "type": "integer" },
        "Query": { "oneOf": [
            { "type": "string", "enum": ["city_view", "metrics"] },
            { "type": "string", "const": "endpoint_view", "description": "documented" },
            { "type": "object", "additionalProperties": false, "required": ["history"],
              "properties": { "history": {
                  "type": "object", "required": ["limit"],
                  "properties": {
                      "before": { "anyOf": [ { "$ref": "#/$defs/Seq" }, { "type": "null" } ] },
                      "limit": { "type": "integer" },
                      "token": { "type": ["string", "null"] },
                      "tags": { "type": "array", "items": { "type": "string" } },
                      "pair": { "type": "array", "prefixItems": [ { "type": "string" }, { "$ref": "#/$defs/Seq" } ], "minItems": 2, "maxItems": 2 },
                      "data": { "type": "object", "additionalProperties": true },
                      "never": false,
                      "any": true
                  } } } },
        ],
        }
    }));
    let expected = "export const Query = Schema.Union(
  Schema.Literal(\"city_view\", \"metrics\"),
  Schema.Literal(\"endpoint_view\"),
  Schema.Struct({
    history: Schema.Struct({
      any: Schema.optional(Schema.Unknown),
      before: Schema.optional(Schema.NullOr(Seq)),
      data: Schema.optional(Schema.Record({ key: Schema.String, value: Schema.Unknown })),
      limit: Schema.Int,
      never: Schema.optional(Schema.Never),
      pair: Schema.optional(Schema.Tuple(Schema.String, Seq)),
      tags: Schema.optional(Schema.Array(Schema.String)),
      token: Schema.optional(Schema.NullOr(Schema.String)),
    }),
  }),
).annotations({ identifier: \"Query\" });
export type Query = typeof Query.Type;
";
    assert!(text.contains(expected), "{text}");
}

#[test]
fn a_definition_comes_after_what_it_refers_to() {
    let text = emitted(json!({
        "Alpha": { "type": "object", "properties": { "z": { "$ref": "#/$defs/Zeta" } }, "required": ["z"] },
        "Zeta": { "type": "object", "properties": { "m": { "$ref": "#/$defs/Mid" } }, "required": ["m"] },
        "Mid": { "type": "string" },
    }));
    let at = |name: &str| text.find(&format!("export const {name} ")).unwrap();
    assert!(at("Mid") < at("Zeta") && at("Zeta") < at("Alpha"), "{text}");
}

#[test]
fn a_keyword_outside_the_subset_is_refused_by_name_and_place() {
    let refused = emit(
        &document(json!({
            "Odd": { "type": "object", "properties": { "x": { "allOf": [ { "type": "string" } ] } } },
        })),
        13,
        "ab12",
    )
    .expect_err("allOf is outside the subset");
    assert_eq!(refused.at, "Odd/properties/x");
    assert!(refused.why.contains("allOf"), "{}", refused.why);
}

#[test]
fn a_defaulted_field_is_optional_and_its_default_is_not_translated() {
    let text = emitted(json!({
        "Limits": {
            "type": "object",
            "required": ["shell"],
            "properties": {
                "shell": { "type": "boolean" },
                "names": { "type": "array", "items": { "type": "string" }, "default": [] },
            },
        },
    }));
    assert!(
        text.contains("names: Schema.optional(Schema.Array(Schema.String)),"),
        "{text}"
    );
    assert!(!text.contains("default"), "{text}");
}

#[test]
fn a_type_that_refers_to_itself_is_refused() {
    let refused = emit(
        &document(json!({
            "Node": { "type": "object", "properties": { "next": { "$ref": "#/$defs/Node" } } },
        })),
        13,
        "ab12",
    )
    .expect_err("a cycle has no definition order");
    assert!(refused.why.contains("Node"), "{}", refused.why);
}

#[test]
fn the_real_wire_comes_out_whole() {
    let text = emit(&channels::wire_schema(), channels::WIRE_V, "ab12")
        .unwrap_or_else(|refused| panic!("refused at {}: {}", refused.at, refused.why));
    for name in [
        "ClientFrame",
        "ServerFrame",
        "Command",
        "Query",
        "Answer",
        "Delta",
        "EventRecord",
    ] {
        assert!(text.contains(&format!("export const {name} = ")), "{name}");
    }
    assert!(
        text.contains("value: NoSecret,"),
        "put_secret carries a value nothing satisfies"
    );
    assert!(text.contains(
        "export const NoSecret = Schema.Never.annotations({ identifier: \"NoSecret\" });"
    ));
}

#[test]
fn the_first_differing_line_is_named_with_both_spellings() {
    let expected = "a\nb\nc\n";
    assert_eq!(first_difference(expected, "a\nb\nc\n"), None);
    assert_eq!(
        first_difference(expected, "a\nB\nc\n"),
        Some((2, "b".to_owned(), "B".to_owned()))
    );
    assert_eq!(
        first_difference(expected, "a\nb\n"),
        Some((3, "c".to_owned(), String::new()))
    );
}
