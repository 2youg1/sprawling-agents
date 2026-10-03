// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! One spelling per value. Each value the city writes as text - into
//! the ledger, onto the wire, into a status line - has one word for it,
//! and every way of writing it out (`as_str`, `Display`, the conversion
//! into `String`, serde) gives that word, which the value's own reader
//! takes back to the same value. A second spelling is a record one
//! reader writes and another refuses.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    reason = "test code"
)]

use std::collections::BTreeSet;

use kernel::event::record::{SignalId, SignalKind};
use kernel::tool::ServerLabel;
use kernel::{
    ApprovalId, Ceiling, DOORS, DelegateKind, Effort, EnvVarName, GoalId, Mode, ModelTag,
    SecondThreshold, ToolName, Window,
};
use proptest::prelude::*;
use serde::Serialize;

/// The serde form of a value that serialises as a JSON string.
fn wire_word<T: Serialize>(value: &T) -> String {
    serde_json::to_value(value)
        .unwrap()
        .as_str()
        .expect("a string on the wire")
        .to_owned()
}

/// The closed sets: `as_str`, `Display` where it exists and serde agree
/// on every member, and no two members share a word.
#[test]
fn every_member_of_a_closed_set_has_one_word_of_its_own() {
    let modes: Vec<(String, String, String)> = Mode::ALL
        .iter()
        .map(|mode| (mode.as_str().to_owned(), mode.to_string(), wire_word(mode)))
        .collect();
    let tags: Vec<(String, String, String)> = ModelTag::ALL
        .iter()
        .map(|tag| (tag.as_str().to_owned(), tag.to_string(), wire_word(tag)))
        .collect();
    let efforts: Vec<(String, String, String)> = Effort::ALL
        .iter()
        .map(|effort| {
            (
                effort.as_str().to_owned(),
                effort.as_str().to_owned(),
                wire_word(effort),
            )
        })
        .collect();
    let delegates: Vec<(String, String, String)> =
        [DelegateKind::Resident, DelegateKind::Ephemeral]
            .iter()
            .map(|kind| {
                (
                    kind.as_str().to_owned(),
                    kind.as_str().to_owned(),
                    wire_word(kind),
                )
            })
            .collect();
    let signals: Vec<(String, String, String)> = [
        SignalKind::Mention,
        SignalKind::Thread,
        SignalKind::Broadcast,
        SignalKind::Steer,
    ]
    .iter()
    .map(|kind| {
        assert_eq!(SignalKind::parse(kind.as_str()).unwrap(), *kind);
        (
            kind.as_str().to_owned(),
            String::from(*kind),
            wire_word(kind),
        )
    })
    .collect();
    for set in [modes, tags, efforts, delegates, signals] {
        for (word, shown, wire) in &set {
            assert_eq!((shown, wire), (word, word));
        }
        let distinct: BTreeSet<&String> = set.iter().map(|(word, _, _)| word).collect();
        assert_eq!(distinct.len(), set.len(), "{set:?}");
    }
}

/// A refusal report names the door that refused, so no two doors may
/// answer to one name, and each name is the snake-case word of a function.
#[test]
fn every_door_has_a_name_of_its_own() {
    let names: BTreeSet<&str> = DOORS.iter().map(|door| door.as_str()).collect();
    assert_eq!(names.len(), DOORS.len(), "{names:?}");
    for name in names {
        assert!(
            !name.is_empty() && name.chars().all(|c| c.is_ascii_lowercase() || c == '_'),
            "{name:?}"
        );
    }
}

proptest! {
    /// An id adopted from text is that text, on the wire and back.
    #[test]
    fn an_adopted_id_is_the_text_it_was_adopted_from(raw in "[a-z0-9-]{1,24}") {
        let approval = ApprovalId::new(raw.clone()).unwrap();
        prop_assert_eq!(approval.as_str(), raw.as_str());
        let goal = GoalId::new(raw.clone()).unwrap();
        prop_assert_eq!(goal.as_str(), raw.as_str());
        let signal = SignalId::parse(&raw).unwrap();
        prop_assert_eq!(signal.as_str(), raw.as_str());
        prop_assert_eq!(wire_word(&signal), raw.clone());
        prop_assert_eq!(SignalId::parse(&String::from(signal.clone())).unwrap(), signal);
    }

    /// A name the city parsed reads out as the text it parsed, through
    /// `Display` and through the conversion into `String`, and parses back.
    #[test]
    fn a_parsed_name_reads_out_as_its_text(raw in "[A-Z][A-Z0-9_]{0,15}", label in "[a-z0-9]{1,12}") {
        if let Ok(name) = EnvVarName::parse(&raw) {
            prop_assert_eq!(name.to_string(), raw.clone());
            prop_assert_eq!(EnvVarName::parse(&String::from(name.clone())).unwrap(), name);
        }
        let server = ServerLabel::parse(&label).unwrap();
        prop_assert_eq!(server.to_string(), label.clone());
        prop_assert_eq!(wire_word(&server), label.clone());
        let tool = ToolName::parse(&label).unwrap();
        prop_assert_eq!(tool.to_string(), label);
    }

    /// A count of tokens is the number it was made from, as a person reads
    /// it and as the wire carries it.
    #[test]
    fn a_count_of_tokens_reads_out_as_its_number(tokens in 2u64..=u64::MAX) {
        let ceiling = Ceiling::new(tokens).unwrap();
        prop_assert_eq!(ceiling.get(), tokens);
        prop_assert_eq!(ceiling.to_string(), tokens.to_string());
        let window = Window::new(tokens).unwrap();
        prop_assert_eq!(window.to_string(), tokens.to_string());
        prop_assert_eq!(serde_json::to_value(window).unwrap(), serde_json::json!(tokens));
    }

    /// The second reminder rung travels as the whole percent it was
    /// parsed from, and reads back to the same rung.
    #[test]
    fn a_reminder_rung_travels_as_its_percent(percent in 31u64..=90) {
        let rung = SecondThreshold::parse(percent).unwrap();
        prop_assert_eq!(serde_json::to_value(rung).unwrap(), serde_json::json!(percent));
        prop_assert_eq!(serde_json::from_value::<SecondThreshold>(serde_json::json!(percent)).unwrap(), rung);
    }
}
