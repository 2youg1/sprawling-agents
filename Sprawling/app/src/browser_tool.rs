// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

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

pub(crate) struct BrowserTool {
    meta: ToolMeta,

    browser: Mutex<Browser>,
}

struct Browser {
    port: Box<dyn BrowserPort + Send>,
    session: Session,

    context: Option<ContextId>,

    snapshot: Option<PageSnapshot>,
    generation: u64,
    cas: Cas,

    origin: storage::BlockOrigin,

    devloop: DevLoop,

    complained: bool,
}

impl BrowserTool {
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
    fn drive(&mut self, call: &ToolCall) -> Result<ToolOutcome, AxError> {
        let verb = Verb::read(&call.args)?;

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

        let mut sent = frames;

        if let Verb::Act { action, .. } = &verb
            && action.resolves_element()
        {
            let origin = ResolvedOrigin::Element(browser::shared_id_of(&last)?);
            let frame = verb.input_frame(&mut self.session, &context, Some(origin))?;
            last = self.port.send(&frame)?.into_result()?;
        }

        if let Verb::Screenshot(request) = &verb
            && request.waits_for_page()
        {
            let frame = request.capture_frame(&mut self.session, &context, &last)?;
            last = self.port.send(&frame)?.into_result()?;
            sent.push(frame);
        }

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
