// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The person's own browser: the one tool here that asks before it acts.
//!
//! A browser this city started holds one building's logins. The browser
//! a person is using holds every account that person has, and the
//! per-building profile isolation this crate's `Profile` keeps stops
//! applying the moment a run attaches to it. So the address is the
//! person's declaration, and a building that enabled the tool without
//! one gets the question rather than a guess.

use kernel::{AxError, Payload};
use serde_json::{Map, Value};

use super::BrowserTool;
use super::building::open_cas;

/// What the model is told about the person's browser. Two things the
/// building's own disclosure has no room for: this one needs the person
/// first, and the sane default is still the other tool.
pub(super) const PERSON_DISCLOSURE: &str = "drive the browser the person is already using, with their own logins. \
     The person must allow it first: they start that browser with remote debugging and the \
     building declares the address. Use `browser` unless the person asked you to look at the \
     page they have open.";

/// Which browser this tool drives, and therefore what its effect is.
pub(crate) enum Role {
    /// The building's own browser: this city starts it, the profile is
    /// the building's, and every page a call opens is an egress.
    Building,
    /// The person's browser at the address they declared. The attach door
    /// judges the registration; the door also scans the bytes a call
    /// would put into a page.
    PersonAt { host: String },
    /// The person's browser, enabled and not yet addressed: every call
    /// answers with the question, and nothing connects.
    PersonWaiting,
}

/// Builds the tool a building that enabled `usersbrowser` gets.
///
/// The port attaches to the address the person declared and never starts
/// or stops anything. A building that enabled the tool and declared no
/// address gets a tool that asks: what a run may reach is a person's
/// whole logged-in life, so the answer is not this city's to invent.
///
/// # Errors
/// Propagates a content store that will not open.
pub(crate) fn for_user_browser(
    city_root: &std::path::Path,
    user: &city::UserBrowser,
) -> Result<BrowserTool, AxError> {
    let cas = open_cas(city_root)?;
    match user {
        city::UserBrowser::Waiting => BrowserTool::new(
            Role::PersonWaiting,
            Box::new(crate::browser_bidi::AttachedBrowser::waiting()),
            cas,
        ),
        city::UserBrowser::At(endpoint) => BrowserTool::new(
            Role::PersonAt {
                host: endpoint.host().to_owned(),
            },
            Box::new(crate::browser_bidi::AttachedBrowser::at(endpoint.url())),
            cas,
        ),
    }
}

/// The argument grammar of the person's-browser tool, as the model reads
/// it.
///
/// Written out rather than mirrored from [`Verb::read`] because the
/// model needs the shape before it calls, and a tool whose schema is
/// empty leaves it guessing. The parser stays the authority on what is
/// accepted; this is what is disclosed, and `browser-SPEC.md` section
/// 19-6 holds the two in one change-set.
///
/// # Errors
/// Propagates a schema that does not build, which is a build-time defect.
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
            "enum": ["open", "snapshot", "act", "screenshot", "measure", "survey", "console", "viewport", "close"],
            "description": "what to do; `survey` judges the whole page against what it declares - alignment, contrast, spacing, colour - and answers one edit per repair; `close` ends this run's attachment and never closes the person's browser",
        },
        "url": text("for open: the page to open"),
        "kind": {
            "type": "string",
            "enum": ["click", "type", "read", "drag", "scroll"],
            "description": "for act: the action to perform",
        },
        "ref": text("for act, measure: a reference from the snapshot, such as `e12`"),
        "refs": {
            "type": "array",
            "items": { "type": "string" },
            "description": "for measure: the references to measure",
        },
        "text": text("for type: what to type"),
        "generation": {
            "type": "integer",
            "minimum": 0,
            "description": "for act: the snapshot the action was decided against",
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
