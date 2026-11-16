// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use kernel::{AxError, Payload};
use serde_json::{Map, Value};

use super::BrowserTool;
use super::building::open_cas;

pub(super) const PERSON_DISCLOSURE: &str = "drive the browser the User is already using, with their own logins. \
     The User must allow it first: they start that browser with remote debugging and the \
     building declares the address. Use `browser` unless the User asked you to look at the \
     page they have open.";

pub(crate) enum Role {
    Building,

    PersonAt { host: String },

    PersonWaiting,
}

pub(crate) fn for_user_browser(
    city_root: &std::path::Path,
    user: &city::UserBrowser,
    origin: storage::BlockOrigin,
) -> Result<BrowserTool, AxError> {
    let cas = open_cas(city_root)?;
    match user {
        city::UserBrowser::Waiting => BrowserTool::new(
            Role::PersonWaiting,
            Box::new(crate::browser_bidi::AttachedBrowser::waiting()),
            cas,
            origin,
        ),
        city::UserBrowser::At(endpoint) => BrowserTool::new(
            Role::PersonAt {
                host: endpoint.host().to_owned(),
            },
            Box::new(crate::browser_bidi::AttachedBrowser::at(endpoint.url())),
            cas,
            origin,
        ),
    }
}

pub(super) fn user_browser_params() -> Result<Payload, AxError> {
    let text =
        |description: &str| serde_json::json!({ "type": "string", "description": description });
    let point = |description: &str| {
        serde_json::json!({
            "type": "object",
            "properties": {
                "x": { "type": "integer" },
                "y": { "type": "integer" },
            },
            "required": ["x", "y"],
            "description": description,
        })
    };
    let properties = serde_json::json!({
        "action": {
            "type": "string",
            "enum": ["open", "snapshot", "act", "screenshot", "measure", "survey", "fetch", "console", "viewport", "close"],
            "description": "what to do; `survey` judges the whole page against what it declares - alignment, contrast, spacing, colour - and answers one edit per repair; `close` ends this run's attachment and never closes the User's browser",
        },
        "url": text("for open: the page to open; for fetch: the absolute http(s) address the open page fetches with its own cookies, answered as readable text"),
        "kind": {
            "type": "string",
            "enum": ["click", "type", "read", "drag", "scroll", "press"],
            "description": "for act: the action to perform; `press` presses one key on whatever the page has focused, so click the element first",
        },
        "ref": text("a reference from the snapshot, such as `e12`: for act, the element acted on; for measure, the element measured; for screenshot, the region to cover, whose box the page reports so nothing is guessed"),
        "refs": {
            "type": "array",
            "items": { "type": "string" },
            "description": "references from the snapshot: for measure, the elements to measure; for screenshot, the elements to cover together as one region, boxed by the same measurement `measure` reads",
        },
        "text": text("for type: what to type"),
        "key": text("for press: the key, as a DOM `KeyboardEvent.key` name such as `Enter`, `Tab` or `ArrowDown`, or one character; a name this tool cannot press is refused with the names it can"),
        "modifiers": {
            "type": "array",
            "items": { "type": "string" },
            "description": "for press: the keys held around it, by their DOM `KeyboardEvent.key` names such as `Control` or `Shift`",
        },
        "generation": {
            "type": "integer",
            "minimum": 0,
            "description": "for act, or for screenshot with `ref`: the snapshot the decision was made against",
        },
        "to": point("for drag: where the drag ends; for scroll: how far it moves, in CSS pixels"),
        "point": point("for drag: a viewport point to start from, when no ref does"),
        "at": point("for scroll: the viewport point the wheel turns over"),
        "steps": {
            "type": "integer",
            "minimum": 1,
            "maximum": 32,
            "description": "for drag: how many intermediate pointer moves to make",
        },
        "width": { "type": "integer", "description": "for viewport: pixels wide" },
        "height": { "type": "integer", "description": "for viewport: pixels high" },
    });
    let mut schema = Map::new();
    schema.insert("type".to_owned(), Value::String("object".to_owned()));
    schema.insert("properties".to_owned(), properties);
    schema.insert(
        "required".to_owned(),
        Value::Array(vec![Value::String("action".to_owned())]),
    );
    Payload::new(schema)
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

    fn disclosed() -> Map<String, Value> {
        user_browser_params().unwrap().as_map().clone()
    }

    #[test]
    fn every_argument_a_key_press_is_read_with_is_disclosed() {
        let press = serde_json::json!({
            "action": "act", "generation": 1, "kind": "press",
            "key": "Enter", "modifiers": ["Control"],
        });
        let Value::Object(fields) = press else {
            panic!("the press is an object");
        };
        let args = Payload::new(fields.clone()).unwrap();
        assert!(
            browser::Verb::read(&args).is_ok(),
            "the parser reads the press"
        );
        let schema = disclosed();
        let properties = schema["properties"].as_object().unwrap();
        let undisclosed: Vec<&String> = fields
            .keys()
            .filter(|name| !properties.contains_key(name.as_str()))
            .collect();
        let kinds = properties["kind"]["enum"].as_array().unwrap();
        assert_eq!(
            (undisclosed, kinds.contains(&Value::from("press"))),
            (Vec::<&String>::new(), true),
            "{properties:?}"
        );
    }

    #[test]
    fn a_reference_is_described_for_every_action_that_takes_one() {
        let schema = disclosed();
        let properties = schema["properties"].as_object().unwrap();
        let describes = |name: &str| properties[name]["description"].as_str().unwrap().to_owned();
        let (one, many) = (describes("ref"), describes("refs"));
        assert_eq!(
            (
                ["act", "measure", "screenshot"].map(|verb| one.contains(verb)),
                ["measure", "screenshot"].map(|verb| many.contains(verb)),
            ),
            ([true; 3], [true; 2]),
            "ref: {one}; refs: {many}"
        );
    }
}
