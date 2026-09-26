// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! A reply turned into what the model is told: the payload each verb
//! answers with, and a look folded into the development loop.

use browser::{DevLoop, Observation, PageSnapshot, Verb};
use kernel::{AxError, Payload, ToolOutcome};
use serde_json::{Map, Value};

use super::Browser;
use super::surveying::surveyed;

impl Browser {
    /// Turns the last reply of an action into what the model is told.
    ///
    /// # Errors
    /// Propagates a reply this build cannot read, a content store that
    /// will not take the bytes, and a page that answered a measurement
    /// with a fraction — the last one because a ledger payload holds
    /// integers and rounding it here would invent a pixel.
    pub(super) fn answer(&mut self, verb: &Verb, result: &Value) -> Result<ToolOutcome, AxError> {
        match verb {
            Verb::Open { url } => {
                self.began_again();
                Ok(ToolOutcome {
                    result: payload(vec![("opened", Value::String(url.clone()))])?,
                    attachments: Vec::new(),
                })
            }
            Verb::Snapshot => self.looked(result),
            Verb::Act { action, .. } => {
                self.began_again();
                Ok(ToolOutcome {
                    result: payload(vec![
                        ("acted", Value::String(acted_on(action))),
                        ("value", Value::String(result.to_string())),
                    ])?,
                    attachments: Vec::new(),
                })
            }
            Verb::Screenshot(request) => self.stored(request, result),
            Verb::Measure { .. } => Ok(ToolOutcome {
                result: payload(vec![("boxes", browser::read_json(result)?)])?,
                attachments: Vec::new(),
            }),
            // A survey is one string on purpose. The tagged report is
            // the shape an agent acts on - one `<edit>` per repair, one
            // `<at>` per place it has to be made - and splitting it into
            // a payload of parts here would be a second rendering of a
            // report that already has one.
            Verb::Survey => Ok(ToolOutcome {
                result: payload(vec![("survey", Value::String(surveyed(result)?))])?,
                attachments: Vec::new(),
            }),
            Verb::Console => {
                let entries = browser::read_json(result)?;
                self.complained = browser::complained(&entries);
                Ok(ToolOutcome {
                    result: payload(vec![
                        ("console", entries),
                        ("complained", Value::Bool(self.complained)),
                    ])?,
                    attachments: Vec::new(),
                })
            }
            Verb::Viewport { width, height } => {
                self.began_again();
                Ok(ToolOutcome {
                    result: payload(vec![
                        ("width", Value::from(*width)),
                        ("height", Value::from(*height)),
                    ])?,
                    attachments: Vec::new(),
                })
            }
            Verb::Close => Ok(ToolOutcome {
                result: payload(vec![("closed", Value::Bool(true))])?,
                attachments: Vec::new(),
            }),
        }
    }

    /// Something changed, so the looks that came before it say nothing
    /// about what the page is doing now.
    fn began_again(&mut self) {
        self.devloop = DevLoop::new();
    }

    /// One look: the page as text, folded into the development loop.
    ///
    /// # Errors
    /// Propagates a tree this build cannot read and a loop asked to
    /// continue past its own ending.
    fn looked(&mut self, result: &Value) -> Result<ToolOutcome, AxError> {
        let tree = browser::read_json(result)?;
        self.generation = self.generation.saturating_add(1);
        let snapshot = PageSnapshot::read(self.generation, &tree)?;
        let text = snapshot.to_text();
        self.snapshot = Some(snapshot);
        let step = self.devloop.observe(&Observation {
            text: text.clone(),
            complained: self.complained,
        })?;
        let (word, looks) = step_word(&step);
        if word != "look_again" {
            // The loop reached an ending; the next look is a new loop
            // rather than a ninth look at a finished one.
            self.began_again();
        }
        Ok(ToolOutcome {
            result: payload(vec![
                ("generation", Value::from(self.generation)),
                ("page", Value::String(text)),
                ("loop", Value::String(word.to_owned())),
                ("looks", Value::from(looks)),
            ])?,
            attachments: Vec::new(),
        })
    }
}

/// What a step acted on, as the ledger names it: the reference the
/// snapshot minted, or the viewport when a pointer action named none.
fn acted_on(action: &browser::Action) -> String {
    action
        .reference()
        .map_or_else(|| "viewport".to_owned(), str::to_owned)
}

/// What a step of the development loop is called on the wire. Written
/// once here because the model reads it and a person reads it in the
/// ledger, and two spellings would be two vocabularies.
fn step_word(step: &browser::Step) -> (&'static str, u32) {
    match step {
        browser::Step::Settled { looks } => ("settled", *looks),
        browser::Step::LookAgain { looks } => ("look_again", *looks),
        browser::Step::Complained { looks } => ("complained", *looks),
        browser::Step::GaveUp { looks, .. } => ("gave_up", *looks),
    }
}

pub(super) fn payload(fields: Vec<(&str, Value)>) -> Result<Payload, AxError> {
    let mut map = Map::new();
    for (key, value) in fields {
        map.insert(key.to_owned(), value);
    }
    Payload::new(map)
}
