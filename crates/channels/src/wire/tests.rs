// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use super::*;

/// The table is the variant list, so what is left to check is that
/// one frame reads back the entry the generator wrote for it.
#[test]
fn a_query_names_itself_with_its_entry_in_the_table() {
    assert_eq!(Query::CityView.name(), "CityView");
    assert!(QUERY_NAMES.contains(&Query::CityView.name()));
    assert!(QUERY_NAMES.contains(&Query::Release.name()));
}

#[test]
fn a_client_frame_round_trips_through_json() {
    let frame = ClientFrame::Query(Query::CityView);
    let text = serde_json::to_string(&frame).unwrap();
    let back: ClientFrame = serde_json::from_str(&text).unwrap();
    assert_eq!(frame, back);
}

/// The document names both roots, every command by its wire name,
/// and the one frame a socket cannot spell as a value nothing
/// satisfies — so the client generated from it refuses the same
/// bytes the server refuses.
#[cfg(feature = "schema")]
#[test]
fn the_schema_document_holds_both_roots_and_every_command() {
    let document = wire_schema();
    let defs = document.get("$defs").and_then(|d| d.as_object()).unwrap();
    assert!(defs.contains_key("ClientFrame"), "client root");
    assert!(defs.contains_key("ServerFrame"), "server root");
    let command = serde_json::to_string(defs.get("Command").unwrap()).unwrap();
    for name in COMMAND_NAMES {
        let mut snake = String::new();
        for (index, ch) in name.chars().enumerate() {
            if ch.is_ascii_uppercase() && index > 0 {
                snake.push('_');
            }
            snake.push(ch.to_ascii_lowercase());
        }
        assert!(
            command.contains(&format!("\"{snake}\"")),
            "{name} on the wire"
        );
    }
    assert!(
        defs.get("NoSecret") == Some(&serde_json::Value::Bool(false)),
        "a credential over the wire satisfies nothing"
    );
    assert_eq!(wire_schema(), document, "the document is a pure function");
}
