// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The `browser` tool: every action over one session.
//!
//! It lives in the assembly layer rather than in `browser` because a
//! screenshot has to land in `storage::cas`, and the browser crate has no
//! dependency on memory and should not grow one. What crosses between
//! them is a `Shot` — bytes and two integer sides — so the decisions
//! stay in the pure crate and the bytes stay here.

use std::sync::Mutex;

use browser::{
    BrowserPort, ContextId, DevLoop, OwnListeners, PageSnapshot, ResolvedOrigin, SHOT_MAX_EDGE_PX,
    Session, SessionRequest, Shot, Verb,
};
use kernel::{
    AxCode, AxError, CostTier, Effect, GateSubject, Payload, RenderIntent, Temporal, Tool,
    ToolCall, ToolMeta, ToolName, ToolOutcome,
};
use serde_json::Value;
use storage::Cas;

mod answering;
mod building;
mod guarding;
mod person;
mod storing;
mod surveying;

use building::BUILDING_DISCLOSURE;
pub(crate) use building::for_rules;
pub(crate) use guarding::{Served, serve};

use person::PERSON_DISCLOSURE;
pub(crate) use person::Role;

/// The tool a building whose rules say `browser = true` gets.
pub(crate) struct BrowserTool {
    meta: ToolMeta,
    /// One tab, driven by one call at a time: two calls interleaving
    /// their frames would each act on a page the other changed.
    browser: Mutex<Browser>,
}

/// What driving the tab holds between calls.
struct Browser {
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
    /// The run and building a screenshot is pinned for.
    origin: storage::BlockOrigin,
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
        origin: storage::BlockOrigin,
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
            browser: Mutex::new(Browser {
                port,
                session: Session::new(),
                context: None,
                snapshot: None,
                generation: 0,
                cas,
                origin,
                devloop: DevLoop::new(),
                complained: false,
            }),
        })
    }
}

impl Tool for BrowserTool {
    fn meta(&self) -> &ToolMeta {
        &self.meta
    }

    /// The host this call names, read by the same grammar `invoke`
    /// reads, so the bench and the tool cannot disagree about which host
    /// a call names.
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

    fn invoke(&self, call: &ToolCall) -> Result<ToolOutcome, AxError> {
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
        self.browser
            .lock()
            .map_err(|_| {
                AxError::failure(
                    AxCode::ToolUnavailable,
                    "drive a browser",
                    "an earlier call left the tab locked when its thread died",
                )
                .with_recovery("dispatch the work again; the next run opens a fresh tab")
            })?
            .drive(call)
    }
}

impl Browser {
    /// One call against the tab, after `invoke` has checked the name.
    fn drive(&mut self, call: &ToolCall) -> Result<ToolOutcome, AxError> {
        let verb = Verb::read(&call.args)?;
        // Neither browser acts on the city's own pages (sprawling D74):
        // the address an action names, the page it would act on, and the
        // page an `open` lands on are each checked.
        let own = guarding::own();
        if let Some(address) = verb.address() {
            own.refuse("drive a browser", address)?;
        }
        let context = self.tab(&own)?;
        let opens = matches!(verb, Verb::Open { .. });
        if !opens {
            self.refuse_own_page(&own, &context)?;
        }
        let frames = verb.frames(&mut self.session, &context, self.snapshot.as_ref())?;
        let mut last = Value::Null;
        for frame in &frames {
            last = self.port.send(frame)?.into_result()?;
        }
        if opens {
            self.refuse_own_page(&own, &context)?;
        }
        // The frames that produced `last`, kept for the one case that has to
        // ask the same thing twice: a picture past the cap.
        let mut sent = frames;
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
        // A shot by reference is the same two frames: the page names the
        // element, and the capture then covers it. The handle is read out
        // of the reply by the request itself, so nothing else can crop to
        // an element the page never named.
        if let Verb::Screenshot(request) = &verb
            && request.waits_for_page()
        {
            let frame = request.capture_frame(&mut self.session, &context, &last)?;
            last = self.port.send(&frame)?.into_result()?;
            sent.push(frame);
        }
        // A picture past the cap is asked for once more: the density that
        // fits it is set, and then the same frames are replayed, because
        // the capture itself is what has to come back smaller. The cost of
        // touching the page's density is paid only here, where the
        // alternative is refusing the shot.
        if let Verb::Screenshot(request) = &verb {
            let shot = Shot::read(&last, request.format())?;
            if SHOT_MAX_EDGE_PX.exceeds(shot.width(), shot.height()) {
                let longest = shot.width().max(shot.height());
                let refit = request.refit_frame(&mut self.session, &context, longest)?;
                self.port.send(&refit)?.into_result()?;
                for again in &sent {
                    last = self.port.send(again)?.into_result()?;
                }
            }
        }
        self.answer(&verb, &last)
    }
}

impl Browser {
    /// Refuses when the tab is on one of the city's own pages, read from
    /// the browser's own account of where the tab is. Nothing to read
    /// when no listener is registered.
    ///
    /// # Errors
    /// `E_GATE_DENIED` for the city's own page; propagates a tree the
    /// browser will not give.
    fn refuse_own_page(&mut self, own: &OwnListeners, context: &ContextId) -> Result<(), AxError> {
        if own.is_empty() {
            return Ok(());
        }
        let tree = self.session.tree()?;
        let answered = self.port.send(&tree)?.into_result()?;
        match browser::page_address(&answered, context) {
            Some(address) => own.refuse("act on a browser tab", &address),
            None => Ok(()),
        }
    }

    /// The tab this tool drives, opening the session the first time: the
    /// first one that is not on one of the city's own pages.
    ///
    /// Both frames are minted by this tool's own `Session`, so the ids
    /// on one socket stay unique: a second counter would send two
    /// frames numbered one and take the wrong reply for each.
    ///
    /// # Errors
    /// Propagates a session the engine refuses to open, and a browser
    /// with no tab in it - which is a browser that is closing, and is
    /// not a page anybody can act on - or with every tab on one of the
    /// city's own pages.
    fn tab(&mut self, own: &OwnListeners) -> Result<ContextId, AxError> {
        if let Some(open) = &self.context {
            return Ok(open.clone());
        }
        let begin = self.session.begin(SessionRequest::default())?;
        self.port.send(&begin)?.into_result()?;
        let tree = self.session.tree()?;
        let answered = self.port.send(&tree)?.into_result()?;
        let context = Session::read_tree(&answered)?
            .into_iter()
            .find(|context| {
                browser::page_address(&answered, context).is_none_or(|address| !own.holds(&address))
            })
            .ok_or_else(|| {
                AxError::failure(
                    AxCode::BrowserUnavailable,
                    "open a browser tab",
                    "the browser has no tab open other than this city's own pages",
                )
                .with_recovery(
                    "open another tab, or start the browser again; a window with no tab is closing",
                )
            })?;
        self.context = Some(context.clone());
        Ok(context)
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
