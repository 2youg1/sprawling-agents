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

/// One tool, under the three fields `tools/list` publishes.
pub(crate) struct ToolCard {
    pub(crate) name: &'static str,
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
            name: "desktop.windows",
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
            name: "desktop.snapshot",
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
            name: "desktop.act",
            description: "Do one thing to one window: click, double, right, drag, scroll, type or \
                 key, at a ref from a snapshot or at a point. `generation` is the snapshot the \
                 action was decided against, and an action decided against an older view is \
                 refused. It does not retry, it does not chain several actions, and it does not \
                 fall back to a nearby element when the ref no longer resolves."
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
                "required": ["action", "generation"],
                "additionalProperties": false,
            }),
        },
        ToolCard {
            name: "desktop.screenshot",
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
            name: "desktop.record",
            description: "Start or stop recording one named window: an mp4 when ffmpeg is on \
                 this machine's PATH, otherwise a directory of PNG frames, with sound only when \
                 `audio` asks for it. The scope file has to switch recording on. It does not \
                 edit, transcode or upload anything, and it does not stop on its own."
                .to_owned(),
            schema: json!({
                "type": "object",
                "properties": properties(json!({
                    "state": { "type": "string", "enum": ["start", "stop"] },
                    "audio": { "type": "boolean", "description": "record this machine's sound as well" },
                })),
                "required": ["state"],
                "additionalProperties": false,
            }),
        },
        ToolCard {
            name: "desktop.clipboard",
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

/// One tool by name.
pub(crate) fn card(name: &str) -> Option<ToolCard> {
    table().into_iter().find(|card| card.name == name)
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

    #[test]
    fn the_table_is_the_six_tools_this_server_promises() {
        let names: Vec<&str> = table().iter().map(|card| card.name).collect();
        assert_eq!(names, EXPECTED.to_vec());
        assert!(card("desktop.act").is_some());
        assert!(card("desktop.reboot").is_none());
    }

    /// The rule this table is held to: a description says what the tool
    /// does and then what it does not do.
    #[test]
    fn every_description_says_what_the_tool_does_not_do() {
        for card in table() {
            assert!(
                card.description.contains("does not"),
                "{} never says what it does not do: {}",
                card.name,
                card.description
            );
        }
    }

    /// A schema that is not an object is a schema the caller cannot fill
    /// in, and `tools/call` sends `arguments` as an object.
    #[test]
    fn every_schema_is_an_object_schema_with_named_properties() {
        for card in table() {
            assert_eq!(card.schema["type"], "object", "{}", card.name);
            assert!(
                card.schema["properties"].is_object(),
                "{} has no properties",
                card.name
            );
        }
    }

    /// `desktop.act` carries the snapshot generation for the same reason
    /// `browser::act` does: a decision made against one view of a window
    /// is refused against another rather than landing on whatever moved
    /// into that position.
    #[test]
    fn acting_names_a_reference_and_the_generation_it_was_decided_against() {
        let act = card("desktop.act").unwrap();
        let properties = &act.schema["properties"];
        assert!(properties["ref"].is_object());
        assert!(properties["generation"].is_object());
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
        let shot = card("desktop.screenshot").unwrap();
        let formats: Vec<&str> = shot.schema["properties"]["format"]["enum"]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(Value::as_str)
            .collect();
        assert_eq!(formats, vec!["png", "jpeg", "webp"]);
        let record = card("desktop.record").unwrap();
        let states: Vec<&str> = record.schema["properties"]["state"]["enum"]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(Value::as_str)
            .collect();
        assert_eq!(states, vec!["start", "stop"]);
        let clipboard = card("desktop.clipboard").unwrap();
        assert!(clipboard.schema["required"].is_array());
    }
}
