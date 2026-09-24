// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The equivalence suite that keeps the hand-written scanner and the
//! serde reference one implementation in two languages. The oracle is
//! serde's own parser with each value captured whole as a raw value - the
//! borrow shape the scanner mirrors - and the property asserts the two
//! solve every input in the stated domain identically (mem-SPEC.md
//! sections 2 and 3).

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "test code: an oracle comparison may assert loudly"
)]

use proptest::prelude::*;
use serde::de::{Deserialize, Deserializer, MapAccess, Visitor};
use serde_json::value::RawValue;

use super::*;

/// The refusal classes both sides are compared on. `TooDeep` is the
/// scanner's own floor and `Other` is "this adapter cannot vouch for the
/// answer" - neither is a verdict the reference gives.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kind {
    Malformed,
    Duplicate,
    TooDeep,
    Other,
}

fn kind_of(refusal: KernelRefusal) -> Kind {
    match refusal {
        KernelRefusal::Malformed => Kind::Malformed,
        KernelRefusal::DuplicateName => Kind::Duplicate,
        KernelRefusal::TooDeep => Kind::TooDeep,
        KernelRefusal::UnknownCode(_) | KernelRefusal::SpanOutsideInput => Kind::Other,
    }
}

/// `(key name, raw value text)` for the keys present, in slot order.
type Answer = Result<Vec<(String, String)>, Kind>;

fn scanned(input: &[u8]) -> Answer {
    let (code, raw) = ask(input);
    let spans = match classify(code) {
        Ok(()) => located(input, &raw),
        Err(refusal) => return Err(kind_of(refusal)),
    }
    .map_err(kind_of)?;
    let mut named = Vec::new();
    for key in EnvelopeKey::ALL {
        if let Some(text) = spans.get(key) {
            let text = String::from_utf8(text.to_vec()).map_err(|_| Kind::Other)?;
            named.push((key.name().to_owned(), text));
        }
    }
    Ok(named)
}

/// The serde reference: one object, every entry kept in order, each
/// value captured whole.
struct Entries(Vec<(String, Box<RawValue>)>);

impl<'de> Deserialize<'de> for Entries {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct EntryVisitor;

        impl<'de> Visitor<'de> for EntryVisitor {
            type Value = Entries;

            fn expecting(&self, out: &mut std::fmt::Formatter) -> std::fmt::Result {
                out.write_str("a JSON object")
            }

            fn visit_map<M: MapAccess<'de>>(self, mut map: M) -> Result<Entries, M::Error> {
                let mut entries = Vec::new();
                while let Some(entry) = map.next_entry::<String, Box<RawValue>>()? {
                    entries.push(entry);
                }
                Ok(Entries(entries))
            }
        }

        deserializer.deserialize_map(EntryVisitor)
    }
}

fn reference(input: &[u8]) -> Answer {
    let entries = match serde_json::from_slice::<Entries>(input) {
        Ok(entries) => entries,
        Err(_) => return Err(Kind::Malformed),
    };
    // A duplicated envelope name refuses and a duplicated other name
    // passes: the derive probe reads "duplicate field" for the fields it
    // names and skips what it does not (mem-SPEC.md section 12, decision 4).
    for key in EnvelopeKey::ALL {
        if entries
            .0
            .iter()
            .filter(|(name, _)| name == key.name())
            .count()
            > 1
        {
            return Err(Kind::Duplicate);
        }
    }
    let mut named = Vec::new();
    for key in EnvelopeKey::ALL {
        if let Some((_, raw)) = entries.0.iter().find(|(name, _)| name == key.name()) {
            named.push((key.name().to_owned(), raw.get().to_owned()));
        }
    }
    Ok(named)
}

fn scalar() -> impl Strategy<Value = String> {
    prop_oneof![
        Just(String::from("null")),
        any::<bool>().prop_map(|b| b.to_string()),
        any::<i64>().prop_map(|n| n.to_string()),
        Just(String::from("1e999")),
        Just(String::from("-0")),
        prop::collection::vec(any::<char>(), 0..8).prop_map(|chars| {
            serde_json::Value::String(chars.into_iter().collect()).to_string()
        }),
    ]
}

fn name() -> impl Strategy<Value = String> {
    prop::collection::vec(any::<char>(), 0..4)
        .prop_map(|chars| serde_json::Value::String(chars.into_iter().collect()).to_string())
}

fn json(depth: u32) -> BoxedStrategy<String> {
    let Some(below) = depth.checked_sub(1) else {
        return scalar().boxed();
    };
    prop_oneof![
        scalar().boxed(),
        prop::collection::vec(json(below), 0..3)
            .prop_map(|items| format!("[{}]", items.join(",")))
            .boxed(),
        prop::collection::vec((name(), json(below)), 0..3)
            .prop_map(|pairs| {
                let body: Vec<String> = pairs
                    .into_iter()
                    .map(|(n, value)| format!("{n}:{value}"))
                    .collect();
                format!("{{{}}}", body.join(","))
            })
            .boxed(),
    ]
    .boxed()
}

/// The envelope names plus two neighbours that are not them, each
/// spelled literally or with escapes - an escaped spelling must not hide
/// a duplicate from either side.
fn envelope_name() -> impl Strategy<Value = String> {
    let choice = prop_oneof![
        Just(String::from("v")),
        Just(String::from("seq")),
        Just(String::from("prev")),
        Just(String::from("kind")),
        Just(String::from("ig")),
        Just(String::from("x")),
        Just(String::from("later")),
    ];
    (choice, any::<bool>()).prop_map(|(name, escaped)| {
        if escaped {
            let mut spelled = String::from("\"");
            for byte in name.bytes() {
                spelled.push_str(&format!("\\u{byte:04x}"));
            }
            spelled.push('"');
            spelled
        } else {
            serde_json::Value::String(name).to_string()
        }
    })
}

fn envelope() -> impl Strategy<Value = String> {
    prop::collection::vec((envelope_name(), json(2)), 0..6).prop_map(|members| {
        let body: Vec<String> = members
            .into_iter()
            .map(|(n, value)| format!("{n} : {value}"))
            .collect();
        format!("{{ {} }}", body.join(" , "))
    })
}

fn solve_both(input: &[u8]) {
    assert_eq!(scanned(input), reference(input));
}

proptest! {
    #[test]
    fn generated_envelopes_solve_identically(text in envelope()) {
        solve_both(text.as_bytes());
    }

    #[test]
    fn mutated_bytes_solve_identically(
        text in envelope(),
        patches in prop::collection::vec((any::<usize>(), prop::collection::vec(any::<u8>(), 0..8)), 0..3),
    ) {
        let mut bytes = text.into_bytes();
        for (at, added) in patches {
            let at = at.min(bytes.len());
            bytes.splice(at..at, added);
        }
        solve_both(&bytes);
    }
}

#[test]
fn the_five_keys_land_in_their_slots_borrowed_from_the_input() {
    let line = br#"{"v":1,"seq":"2","prev":{"x":[1,2]},"kind":"run","ig":false,"later":9}"#;
    let spans = scan(line).map_err(|err| err.to_string()).unwrap();
    assert_eq!(spans.get(EnvelopeKey::V), Some(&b"1"[..]));
    assert_eq!(spans.get(EnvelopeKey::Seq), Some(&b"\"2\""[..]));
    assert_eq!(spans.get(EnvelopeKey::Prev), Some(&b"{\"x\":[1,2]}"[..]));
    assert_eq!(spans.get(EnvelopeKey::Kind), Some(&b"\"run\""[..]));
    assert_eq!(spans.get(EnvelopeKey::Ig), Some(&b"false"[..]));
}

#[test]
fn a_key_absent_from_the_object_stays_absent() {
    let spans = scan(br#"{"kind":"run"}"#)
        .map_err(|err| err.to_string())
        .unwrap();
    assert_eq!(spans.get(EnvelopeKey::Kind), Some(&b"\"run\""[..]));
    assert_eq!(spans.get(EnvelopeKey::V), None);
}

#[test]
fn an_unknown_code_is_refused_as_a_wire_mismatch_not_read_as_anything() {
    let refusal = classify(7).expect_err("a code the mapping does not name must refuse");
    let err = refusal.into_error();
    assert_eq!(err.code(), &AxCode::WireMismatch);
}

#[test]
fn nesting_past_the_depth_cap_is_refused_by_the_scanner_alone() {
    // The one known divergence (mem-SPEC.md section 3): the serde
    // reference accepts past the cap until its own stack gives out, so
    // the equivalence property is stated over the shallower domain and
    // this boundary is pinned here instead.
    let mut deep = String::from("{\"k\":");
    deep.push_str(&"[".repeat(1024));
    deep.push_str(&"]".repeat(1024));
    deep.push('}');
    assert_eq!(scanned(deep.as_bytes()), Err(Kind::TooDeep));
}
