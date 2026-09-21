// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The `browser` tool: every action over one session.
//!
//! It lives in the assembly layer rather than in `browser` because a
//! screenshot has to land in `memory::cas`, and the browser crate has no
//! dependency on memory and should not grow one. What crosses between
//! them is a `Shot` — bytes and two integer sides — so the decisions
//! stay in the pure crate and the bytes stay here.

use browser::{
    BrowserPort, ContextId, DevLoop, Observation, PageSnapshot, ResolvedOrigin, Session,
    SessionRequest, Shot, Verb,
};
use kernel::{
    AxCode, AxError, CostTier, Effect, GateSubject, ImageRef, ImageType, Locator, Payload,
    RenderIntent, Temporal, Tool, ToolCall, ToolMeta, ToolName, ToolOutcome,
};
use memory::Cas;
use serde_json::{Map, Value};

mod building;
mod person;
mod surveying;

use building::BUILDING_DISCLOSURE;
pub(crate) use building::for_rules;

use person::PERSON_DISCLOSURE;
pub(crate) use person::Role;
use surveying::surveyed;

/// The tool a building whose rules say `browser = true` gets.
pub(crate) struct BrowserTool {
    meta: ToolMeta,
    port: Box<dyn BrowserPort + Send>,
    session: Session,
    /// The tab this tool drives, opened on the first call rather than at
    /// construction: a building that admits the tool and never uses it
    /// should not start a browser.
    context: Option<ContextId>,
    /// The page as it was last looked at. A reference is a position in
    /// this, which is why acting without it is refused rather than
    /// guessed.
    snapshot: Option<PageSnapshot>,
    generation: u64,
    cas: Cas,
    /// The development loop. Every look folds into it, so "change
    /// something, look at it, decide" is one object rather than a habit
    /// the model has to remember.
    devloop: DevLoop,
    /// Whether the last console read found an error. Carried because a
    /// look and a complaint arrive in two different calls.
    complained: bool,
}

impl BrowserTool {
    /// # Errors
    /// Refuses a name this build cannot spell, which the literals below
    /// cannot produce, and a parameter schema that does not build.
    pub(crate) fn new(
        role: Role,
        port: Box<dyn BrowserPort + Send>,
        cas: Cas,
    ) -> Result<BrowserTool, AxError> {
        let (name, disclosure, params, effect) = match role {
            Role::Building => (
                ToolName::BROWSER,
                BUILDING_DISCLOSURE,
                Payload::empty(),
                // Every page a browser opens leaves this machine, so the
                // egress door decides it - which is also what keeps a
                // confidential building from ever holding one.
                Effect::Egress,
            ),
            Role::PersonAt { host } => (
                ToolName::USER_BROWSER,
                PERSON_DISCLOSURE,
                person::user_browser_params()?,
                Effect::AttachUserBrowser {
                    address: Some(host),
                },
            ),
            Role::PersonWaiting => (
                ToolName::USER_BROWSER,
                PERSON_DISCLOSURE,
                person::user_browser_params()?,
                Effect::AttachUserBrowser { address: None },
            ),
        };
        Ok(BrowserTool {
            meta: ToolMeta {
                name: ToolName::parse(name)?,
                disclosure: disclosure.to_owned(),
                params,
                effect,
                cost_tier: CostTier::Light,
                timeout: None,
                render: RenderIntent::Generic,
                temporal: Temporal::Timestamped,
            },
            port,
            session: Session::new(),
            context: None,
            snapshot: None,
            generation: 0,
            cas,
            devloop: DevLoop::new(),
            complained: false,
        })
    }
}

impl Tool for BrowserTool {
    fn meta(&self) -> &ToolMeta {
        &self.meta
    }

    /// The host this call names, read by the same grammar `invoke`
    /// reads (M-17). The bench used to spell `host` by hand, which this
    /// tool never wrote.
    ///
    /// # Errors
    /// Refuses arguments this tool cannot read, which is what `invoke`
    /// would refuse them with.
    fn subject(&self, call: &ToolCall) -> Result<GateSubject, AxError> {
        match Verb::read(&call.args)?.destination()? {
            Some(host) => Ok(GateSubject::Host(host)),
            None => Ok(GateSubject::None),
        }
    }

    fn invoke(&mut self, call: &ToolCall) -> Result<ToolOutcome, AxError> {
        if call.name != self.meta.name {
            return Err(AxError::failure(
                AxCode::InvalidArgs,
                "drive a browser",
                call.name.to_string(),
            )
            .with_recovery(format!(
                "this tool answers to `{}` and routes nothing else",
                self.meta.name
            )));
        }
        let verb = Verb::read(&call.args)?;
        let context = self.tab()?;
        let frames = verb.frames(&mut self.session, &context, self.snapshot.as_ref())?;
        let mut last = Value::Null;
        for frame in &frames {
            last = self.port.send(frame)?.into_result()?;
        }
        // A drag from a reference is two frames: the page names the
        // element, and the input frame then acts on it. The vocabulary
        // is `desktop.act`'s — a ref or a point, to a point — so the
        // same gesture has one description on both sides.
        if let Verb::Act { action, .. } = &verb
            && action.resolves_element()
        {
            let origin = ResolvedOrigin::Element(browser::shared_id_of(&last)?);
            let frame = verb.input_frame(&mut self.session, &context, Some(origin))?;
            last = self.port.send(&frame)?.into_result()?;
        }
        self.answer(&verb, &last)
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

fn payload(fields: Vec<(&str, Value)>) -> Result<Payload, AxError> {
    let mut map = Map::new();
    for (key, value) in fields {
        map.insert(key.to_owned(), value);
    }
    Payload::new(map)
}

impl BrowserTool {
    /// The tab this tool drives, opening the session the first time.
    ///
    /// Both frames are minted by this tool's own `Session`, so the ids
    /// on one socket stay unique: a second counter would send two
    /// frames numbered one and take the wrong reply for each.
    ///
    /// # Errors
    /// Propagates a session the engine refuses to open, and a browser
    /// with no tab in it - which is a browser that is closing, and is
    /// not a page anybody can act on.
    fn tab(&mut self) -> Result<ContextId, AxError> {
        if let Some(open) = &self.context {
            return Ok(open.clone());
        }
        let begin = self.session.begin(SessionRequest::default())?;
        self.port.send(&begin)?.into_result()?;
        let tree = self.session.tree()?;
        let answered = self.port.send(&tree)?.into_result()?;
        let context = Session::read_tree(&answered)?
            .into_iter()
            .next()
            .ok_or_else(|| {
                AxError::failure(
                    AxCode::BrowserUnavailable,
                    "open a browser tab",
                    "the browser has no tab open",
                )
                .with_recovery("start the browser again; a window with no tab is one closing")
            })?;
        self.context = Some(context.clone());
        Ok(context)
    }

    /// Turns the last reply of an action into what the model is told.
    ///
    /// # Errors
    /// Propagates a reply this build cannot read, a content store that
    /// will not take the bytes, and a page that answered a measurement
    /// with a fraction — the last one because a ledger payload holds
    /// integers and rounding it here would invent a pixel.
    fn answer(&mut self, verb: &Verb, result: &Value) -> Result<ToolOutcome, AxError> {
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
            Verb::Screenshot(request) => self.stored(result, request.format()),
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

    /// A screenshot, put where it becomes evidence.
    ///
    /// Three things happen together or none does: the bytes land in the
    /// content store, the payload carries the locator and the two sides,
    /// and the outcome carries the picture itself so the model sees it.
    ///
    /// # Errors
    /// Propagates a reply this build cannot read and a content store
    /// that will not take the bytes.
    fn stored(&mut self, result: &Value, media: ImageType) -> Result<ToolOutcome, AxError> {
        let shot = Shot::read(result, media)?;
        let hash = self
            .cas
            .put(shot.bytes())
            .map_err(memory::MemoryError::into_ax)?;
        let locator = Locator::parse(&format!("cas:b3-{hash}"))?;
        let picture = ImageRef {
            locator,
            media_type: shot.media(),
            width: shot.width(),
            height: shot.height(),
        };
        Ok(ToolOutcome {
            result: payload(vec![
                ("image", Value::String(picture.locator.to_string())),
                ("width", Value::from(picture.width)),
                ("height", Value::from(picture.height)),
                (
                    "media_type",
                    Value::String(picture.media_type.mime().to_owned()),
                ),
            ])?,
            attachments: vec![picture],
        })
    }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::wildcard_enum_match_arm,
    clippy::let_underscore_must_use,
    clippy::let_underscore_untyped,
    reason = "test code"
)]
mod tests;
