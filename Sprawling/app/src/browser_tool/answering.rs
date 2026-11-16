// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use browser::{DevLoop, Observation, PageSnapshot, Verb};
use kernel::{AxError, Payload, ToolOutcome};
use serde_json::{Map, Value};

use super::Browser;
use super::surveying::surveyed;

impl Browser {
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

            Verb::Survey => Ok(ToolOutcome {
                result: payload(vec![("survey", Value::String(surveyed(result)?))])?,
                attachments: Vec::new(),
            }),

            Verb::Fetch { .. } => {
                let Value::Object(fields) = browser::read_json(result)? else {
                    return Err(AxError::failure(
                        kernel::AxCode::WireMismatch,
                        "read what the page fetched",
                        "the page answered with something other than an object",
                    )
                    .with_recovery(
                        "report this against browser::verb: the fetch script returns one object",
                    ));
                };
                Ok(ToolOutcome {
                    result: Payload::new(fields)?,
                    attachments: Vec::new(),
                })
            }
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

    fn began_again(&mut self) {
        self.devloop = DevLoop::new();
    }

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

fn acted_on(action: &browser::Action) -> String {
    match action {
        browser::Action::Press { .. } => "focus".to_owned(),
        browser::Action::Click { .. }
        | browser::Action::Type { .. }
        | browser::Action::Read { .. }
        | browser::Action::Drag { .. }
        | browser::Action::Scroll { .. } => action
            .reference()
            .map_or_else(|| "viewport".to_owned(), str::to_owned),
    }
}

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

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "test code"
)]
mod tests {
    use std::collections::BTreeSet;

    use super::*;

    #[test]
    fn a_key_press_is_recorded_as_acting_on_the_focus() {
        let press = browser::Action::Press {
            key: browser::Key::parse("Enter").unwrap(),
            modifiers: BTreeSet::new(),
        };
        let scroll = browser::Action::Scroll {
            at: None,
            by: browser::Point { x: 0, y: 40 },
        };
        let click = browser::Action::Click {
            reference: "e3".to_owned(),
        };
        assert_eq!(
            [&press, &scroll, &click].map(acted_on),
            ["focus", "viewport", "e3"].map(str::to_owned)
        );
    }
}
