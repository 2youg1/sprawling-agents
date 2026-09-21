// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The six tools this server offers, as data.
//!
//! Editing this file is editing behaviour: the names, the descriptions
//! and the schemas are the whole of what a model is told. Every
//! description ends with what the tool does **not** do, because the most
//! expensive mistake a model makes with a tool table is taking one tool
//! for the one beside it.
//!
//! Outside Windows every call is refused with `E_TOOL_UNAVAILABLE`
//! (`crate::platform`), which is honest in a way that a fabricated
//! success would not be.

use serde_json::{Value, json};

/// The name of one of the six tools, as a closed set.
///
/// **The one authority on what this server's tools are called.** The
/// table below, the scope decision (`crate::scope`) and the routing in
/// `crate::platform` each used to spell the six strings out, so a
/// seventh tool, or a renamed one, was three edits that nothing checked
/// against each other. Routing now matches this enum and needs no arm
/// for a name nobody offers: [`ToolName::parse`] is where an unknown
/// name stops.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ToolName {
    Windows,
    Snapshot,
    Act,
    Screenshot,
    Record,
    Clipboard,
}

impl ToolName {
    /// Every name there is. The order the tools are *published* in is
    /// [`table`]'s, which a test below holds to this list's membership.
    pub(crate) const ALL: [ToolName; 6] = [
        ToolName::Windows,
        ToolName::Snapshot,
        ToolName::Act,
        ToolName::Screenshot,
        ToolName::Record,
        ToolName::Clipboard,
    ];

    /// What a caller writes in `tools/call`.
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            ToolName::Windows => "desktop.windows",
            ToolName::Snapshot => "desktop.snapshot",
            ToolName::Act => "desktop.act",
            ToolName::Screenshot => "desktop.screenshot",
            ToolName::Record => "desktop.record",
            ToolName::Clipboard => "desktop.clipboard",
        }
    }

    /// The name a call sent, or nothing when this server offers no such
    /// tool. The caller writes the refusal, because the sentence a
    /// person reads belongs to the door that was knocked on.
    pub(crate) fn parse(name: &str) -> Option<ToolName> {
        ToolName::ALL.into_iter().find(|tool| tool.as_str() == name)
    }
}

/// One tool, under the three fields `tools/list` publishes.
pub(crate) struct ToolCard {
    pub(crate) name: ToolName,
    pub(crate) description: String,
    pub(crate) schema: Value,
}

/// A window is named by its title, its process, or both. The two
/// properties are repeated on every tool that touches one window
/// because the scope decision reads them there (`crate::scope`).
fn naming_a_window() -> Value {
    json!({
        "title": { "type": "string", "description": "the window's title, as `desktop.windows` reported it" },
        "process": { "type": "string", "description": "the process that owns the window, for example `notepad.exe`" },
    })
}

/// Merges the window-naming properties into one tool's own.
fn properties(own: Value) -> Value {
    let mut merged = naming_a_window();
    if let (Some(into), Some(from)) = (merged.as_object_mut(), own.as_object()) {
        for (key, value) in from {
            into.insert(key.clone(), value.clone());
        }
    }
    merged
}

/// Every tool, in the order a reader meets them: look, then act.
pub(crate) fn table() -> Vec<ToolCard> {
    vec![
        ToolCard {
            name: ToolName::Windows,
            description: "List the top-level windows on this desktop: title, owning process, \
                 bounds, and a ref the other tools take. Only windows this server's scope file \
                 lists are reported. It does not focus, move, resize or close anything, and it \
                 does not read what is inside a window."
                .to_owned(),
            schema: json!({
                "type": "object",
                "properties": {
                    "process": { "type": "string", "description": "report only windows owned by this process" },
                },
                "required": [],
                "additionalProperties": false,
            }),
        },
        ToolCard {
            name: ToolName::Snapshot,
            description: "Read the accessibility tree of one named window: role, name, ref and \
                 bounds per node, with a generation number that `desktop.act` carries back. It \
                 does not return pixels, it does not expose native handles, and it does not read \
                 a window the scope file leaves out."
                .to_owned(),
            schema: json!({
                "type": "object",
                "properties": properties(json!({
                    "depth": { "type": "integer", "minimum": 1, "description": "how many levels of the tree to return" },
                })),
                "required": [],
                "additionalProperties": false,
            }),
        },
        ToolCard {
            name: ToolName::Act,
            description: "Do one thing to one window: click, double, right, drag, scroll, type or \
                 key, at a ref from a snapshot or at a point. `generation` is the snapshot the \
                 action was decided against, and an action decided against an older view is \
                 refused. The named window must hold the keyboard when the action is sent, and \
                 must be the window under the point it lands on; otherwise nothing is sent. It \
                 does not retry, it does not chain several actions, and it does not fall back \
                 to a nearby element when the ref no longer resolves."
                .to_owned(),
            schema: json!({
                "type": "object",
                "properties": properties(json!({
                    "action": {
                        "type": "string",
                        "enum": ["click", "double", "right", "drag", "scroll", "type", "key"],
                    },
                    "ref": { "type": "string", "description": "a ref minted by `desktop.snapshot`, such as `e12`" },
                    "point": {
                        "type": "object",
                        "properties": {
                            "x": { "type": "integer" },
                            "y": { "type": "integer" },
                        },
                        "required": ["x", "y"],
                        "description": "window-relative point, for what no ref names",
                    },
                    "to": {
                        "type": "object",
                        "properties": {
                            "x": { "type": "integer" },
                            "y": { "type": "integer" },
                        },
                        "required": ["x", "y"],
                        "description": "where a drag ends, or how far a scroll goes",
                    },
                    "text": { "type": "string", "description": "what `type` types" },
                    "key": { "type": "string", "description": "what `key` presses, such as `Enter` or `F2`" },
                    "modifiers": {
                        "type": "array",
                        "items": { "type": "string", "enum": ["ctrl", "alt", "shift", "win"] },
                    },
                    "generation": {
                        "type": "integer",
                        "minimum": 0,
                        "description": "the generation of the snapshot this action was decided against",
                    },
                })),
                "required": ["action"],
                // `generation` is required by what the call names, not
                // by every call: an action at a point was decided
                // against no snapshot, so demanding the number would
                // make a caller invent one.
                "dependentRequired": { "ref": ["generation"] },
                "additionalProperties": false,
            }),
        },
        ToolCard {
            name: ToolName::Screenshot,
            description: "Capture one named window, or a region of it, and return the image as \
                 base64 with its width, height and mime type. `scale` and `quality` are whole \
                 percentages, because every call is recorded and the record holds no fractions. \
                 It does not write a file, it does not read text out of the image, and it does \
                 not capture the whole screen."
                .to_owned(),
            schema: json!({
                "type": "object",
                "properties": properties(json!({
                    "region": {
                        "type": "object",
                        "properties": {
                            "x": { "type": "integer" },
                            "y": { "type": "integer" },
                            "width": { "type": "integer", "minimum": 1 },
                            "height": { "type": "integer", "minimum": 1 },
                        },
                        "required": ["x", "y", "width", "height"],
                    },
                    "format": { "type": "string", "enum": ["png", "jpeg", "webp"] },
                    "quality": { "type": "integer", "minimum": 1, "maximum": 100 },
                    "scale": { "type": "integer", "minimum": 1, "maximum": 100, "description": "percent of the original size" },
                })),
                "required": [],
                "additionalProperties": false,
            }),
        },
        ToolCard {
            name: ToolName::Record,
            description: "Start or stop recording one named window: an mp4 when ffmpeg is on \
                 this machine's PATH, otherwise a directory of PNG frames. `start` answers with \
                 a `recording` id, and `stop` takes that id back, because a window's title can \
                 change while it is being recorded. The scope file has to switch recording on. \
                 A recording ends itself after ten minutes. It does not edit, transcode or \
                 upload anything, and it does not record sound."
                .to_owned(),
            schema: json!({
                "type": "object",
                "properties": properties(json!({
                    "state": { "type": "string", "enum": ["start", "stop"] },
                    "recording": {
                        "type": "integer",
                        "minimum": 1,
                        "description": "which recording `stop` ends, as `start` reported it",
                    },
                })),
                "required": ["state"],
                "additionalProperties": false,
            }),
        },
        ToolCard {
            name: ToolName::Clipboard,
            description: "Read this machine's clipboard as text, or replace it with text. The \
                 scope file has to switch the clipboard on. It does not touch images or file \
                 lists, it does not keep a history, and it does not restore what it replaced."
                .to_owned(),
            schema: json!({
                "type": "object",
                "properties": {
                    "operation": { "type": "string", "enum": ["get", "set"] },
                    "text": { "type": "string", "description": "what `set` puts there" },
                },
                "required": ["operation"],
                "additionalProperties": false,
            }),
        },
    ]
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

    const EXPECTED: [&str; 6] = [
        "desktop.windows",
        "desktop.snapshot",
        "desktop.act",
        "desktop.screenshot",
        "desktop.record",
        "desktop.clipboard",
    ];

    fn card(name: ToolName) -> ToolCard {
        table()
            .into_iter()
            .find(|card| card.name == name)
            .expect("every name in the enum has a card")
    }

    #[test]
    fn the_table_is_the_six_tools_this_server_promises() {
        let names: Vec<&str> = table().iter().map(|card| card.name.as_str()).collect();
        assert_eq!(names, EXPECTED.to_vec());
        assert_eq!(ToolName::parse("desktop.act"), Some(ToolName::Act));
        assert_eq!(ToolName::parse("desktop.reboot"), None);
        // A name in the enum with no card would be a tool the router
        // reaches and `tools/list` never mentions.
        let published: Vec<ToolName> = table().iter().map(|card| card.name).collect();
        for name in ToolName::ALL {
            assert!(published.contains(&name), "{} has no card", name.as_str());
        }
    }

    /// The rule this table is held to: a description says what the tool
    /// does and then what it does not do.
    #[test]
    fn every_description_says_what_the_tool_does_not_do() {
        for card in table() {
            assert!(
                card.description.contains("does not"),
                "{} never says what it does not do: {}",
                card.name.as_str(),
                card.description
            );
        }
    }

    /// A schema that is not an object is a schema the caller cannot fill
    /// in, and `tools/call` sends `arguments` as an object.
    #[test]
    fn every_schema_is_an_object_schema_with_named_properties() {
        for card in table() {
            assert_eq!(card.schema["type"], "object", "{}", card.name.as_str());
            assert!(
                card.schema["properties"].is_object(),
                "{} has no properties",
                card.name.as_str()
            );
        }
    }

    /// `desktop.act` carries the snapshot generation for the same reason
    /// `browser::act` does: a decision made against one view of a window
    /// is refused against another rather than landing on whatever moved
    /// into that position.
    #[test]
    fn acting_names_a_reference_and_the_generation_it_was_decided_against() {
        let act = card(ToolName::Act);
        let properties = &act.schema["properties"];
        assert!(properties["ref"].is_object());
        assert!(properties["generation"].is_object());
        // What the schema demands is what the implementation demands:
        // a generation only where a ref was named.
        assert_eq!(act.schema["required"], json!(["action"]));
        assert_eq!(
            act.schema["dependentRequired"],
            json!({ "ref": ["generation"] })
        );
        assert!(properties["action"]["enum"].is_array());
        let actions: Vec<&str> = properties["action"]["enum"]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(Value::as_str)
            .collect();
        for verb in ["click", "double", "right", "drag", "scroll", "type", "key"] {
            assert!(actions.contains(&verb), "{verb} is not offered");
        }
    }

    #[test]
    fn a_screenshot_names_its_format_and_a_recording_names_its_two_states() {
        let shot = card(ToolName::Screenshot);
        let formats: Vec<&str> = shot.schema["properties"]["format"]["enum"]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(Value::as_str)
            .collect();
        assert_eq!(formats, vec!["png", "jpeg", "webp"]);
        let record = card(ToolName::Record);
        let states: Vec<&str> = record.schema["properties"]["state"]["enum"]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(Value::as_str)
            .collect();
        assert_eq!(states, vec!["start", "stop"]);
        // A stop names the recording rather than the window, so a
        // retitled window is still one this caller can stop.
        assert_eq!(record.schema["properties"]["recording"]["type"], "integer");
        let clipboard = card(ToolName::Clipboard);
        assert!(clipboard.schema["required"].is_array());
    }
}
