// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

mod read;
pub(crate) mod script;

use kernel::{AxCode, AxError, Payload};
use serde_json::json;

use crate::act::{Action, Origin, frame_for, resolve_frame};
use crate::port::Frame;
use crate::session::{ContextId, Session};
use crate::shot::ShotRequest;
use crate::snapshot::PageSnapshot;
use read::{fetched_url, number_of, read_action, side_of, text_of};
use script::{fetch_script, measure_script, read_references};

pub use script::{complained, read_json};

const RECORDER_SCRIPT: &str = concat!(
    "(() => { if (window.__sprawling_console) { return 'ready'; } ",
    "window.__sprawling_console = []; ",
    "for (const level of ['log', 'info', 'warn', 'error']) { ",
    "const original = console[level].bind(console); ",
    "console[level] = (...parts) => { ",
    "window.__sprawling_console.push({ level, text: parts.map(String).join(' ') }); ",
    "original(...parts); }; } ",
    "window.addEventListener('error', e => window.__sprawling_console.push(",
    "{ level: 'error', text: String(e.message) })); ",
    "return 'ready'; })()"
);

const CONSOLE_SCRIPT: &str = "JSON.stringify(window.__sprawling_console || [])";

const NAME_MAX_JS: usize = 200;

fn tree_script() -> String {
    format!(
        "JSON.stringify([...document.querySelectorAll('*')].map(e => {{ \
         const m = {map}; \
         const t = e.tagName.toLowerCase(); \
         const k = (e.getAttribute('type') || 'text').toLowerCase(); \
         return {{ role: e.getAttribute('role') || m[t + '|' + k] || m[t] || '', \
         name: (e.getAttribute('aria-label') || e.textContent || '').trim().slice(0, {NAME_MAX_JS}) \
         }}; }}))",
        map = crate::snapshot::role_lookup_js()
    )
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Verb {
    Open { url: String },
    Snapshot,
    Act { generation: u64, action: Action },
    Screenshot(ShotRequest),
    Measure { references: Vec<String> },
    Survey,

    Fetch { url: String },
    Console,
    Viewport { width: u32, height: u32 },
    Close,
}

impl Verb {
    pub fn destination(&self) -> Result<Option<String>, AxError> {
        self.address()
            .map(kernel::gate::host_of)
            .transpose()
            .map(Option::flatten)
    }

    #[must_use]
    pub fn address(&self) -> Option<&str> {
        match self {
            Verb::Open { url } | Verb::Fetch { url } => Some(url.as_str()),
            Verb::Snapshot
            | Verb::Act { .. }
            | Verb::Screenshot(_)
            | Verb::Measure { .. }
            | Verb::Survey
            | Verb::Console
            | Verb::Viewport { .. }
            | Verb::Close => None,
        }
    }

    pub fn input_frame(
        &self,
        session: &mut Session,
        context: &ContextId,
        origin: Option<crate::input::Origin>,
    ) -> Result<Frame, AxError> {
        match self {
            Verb::Act {
                action: Action::Drag { from, to, steps },
                ..
            } => {
                let from = match (from, origin) {
                    (Origin::Point(at), None) => crate::input::Origin::Viewport(*at),
                    (Origin::Reference(_), Some(resolved @ crate::input::Origin::Element(_))) => {
                        resolved
                    }
                    (_, _) => return Err(origin_mismatch()),
                };
                crate::input::pointer_frame(
                    session,
                    context,
                    &crate::input::DragPath {
                        from,
                        to: *to,
                        steps: *steps,
                    },
                )
            }
            Verb::Act {
                action: Action::Scroll { at, by },
                ..
            } => {
                if origin.is_some() {
                    return Err(origin_mismatch());
                }
                crate::input::wheel_frame(session, context, *at, *by)
            }
            Verb::Act {
                action: Action::Press { key, modifiers },
                ..
            } => {
                if origin.is_some() {
                    return Err(origin_mismatch());
                }
                crate::keyboard::key_frame(session, context, *key, modifiers)
            }
            Verb::Open { .. }
            | Verb::Snapshot
            | Verb::Act { .. }
            | Verb::Screenshot(_)
            | Verb::Measure { .. }
            | Verb::Survey
            | Verb::Fetch { .. }
            | Verb::Console
            | Verb::Viewport { .. }
            | Verb::Close => Err(AxError::failure(
                AxCode::InvalidArgs,
                "act on a page",
                "this action is not an input action",
            )
            .with_recovery(
                "report this against browser::verb: only drag, scroll and press have input frames",
            )),
        }
    }

    pub fn read(args: &Payload) -> Result<Verb, AxError> {
        let action = text_of(args, "action")?;
        match action.as_str() {
            "open" => Ok(Verb::Open {
                url: text_of(args, "url")?,
            }),
            "snapshot" => Ok(Verb::Snapshot),
            "act" => Ok(Verb::Act {
                generation: number_of(args, "generation")?,
                action: read_action(args)?,
            }),
            "screenshot" => Ok(Verb::Screenshot(ShotRequest::read(args)?)),
            "measure" => Ok(Verb::Measure {
                references: read_references(args)?,
            }),
            "survey" => Ok(Verb::Survey),
            "fetch" => Ok(Verb::Fetch {
                url: fetched_url(args)?,
            }),
            "console" => Ok(Verb::Console),
            "viewport" => Ok(Verb::Viewport {
                width: side_of(args, "width")?,
                height: side_of(args, "height")?,
            }),
            "close" => Ok(Verb::Close),
            other => Err(AxError::failure(
                AxCode::InvalidArgs,
                "read a browser action",
                other.to_owned(),
            )
            .with_recovery(
                "one of open, snapshot, act, screenshot, measure, survey, fetch, console, \
                 viewport, close",
            )),
        }
    }

    pub fn frames(
        &self,
        session: &mut Session,
        context: &ContextId,
        snapshot: Option<&PageSnapshot>,
    ) -> Result<Vec<Frame>, AxError> {
        match self {
            Verb::Open { url } => Ok(vec![
                session.navigate(context, url)?,
                session.evaluate(context, RECORDER_SCRIPT)?,
            ]),
            Verb::Snapshot => Ok(vec![session.evaluate(context, &tree_script())?]),
            Verb::Act { generation, action } => match action {
                Action::Drag { .. } | Action::Scroll { .. } | Action::Press { .. } => {
                    let looked = looked_at(snapshot, "act on a page")?;
                    if action.resolves_element() {
                        let Some(reference) = action.reference() else {
                            return Err(origin_mismatch());
                        };

                        crate::act::ensure_fresh(looked, *generation)?;
                        Ok(vec![resolve_frame(session, context, looked, reference)?])
                    } else {
                        Ok(vec![self.input_frame(session, context, None)?])
                    }
                }
                Action::Click { .. } | Action::Type { .. } | Action::Read { .. } => {
                    Ok(vec![frame_for(
                        session,
                        context,
                        looked_at(snapshot, "act on a page")?,
                        *generation,
                        action,
                    )?])
                }
            },
            Verb::Screenshot(request) => request.frames(session, context, snapshot),
            Verb::Measure { references } => Ok(vec![session.evaluate(
                context,
                &measure_script(looked_at(snapshot, "measure a page")?, references)?,
            )?]),

            Verb::Survey => {
                Ok(vec![session.evaluate(
                    context,
                    &crate::survey::probe::evaluated(None),
                )?])
            }
            Verb::Fetch { url } => Ok(vec![session.evaluate(context, &fetch_script(url))?]),
            Verb::Console => Ok(vec![session.evaluate(context, CONSOLE_SCRIPT)?]),
            Verb::Viewport { width, height } => Ok(vec![session.frame(
                "browsingContext.setViewport",
                json!({
                    "context": context.as_str(),
                    "viewport": { "width": width, "height": height },
                }),
            )?]),
            Verb::Close => Ok(vec![session.end()?]),
        }
    }
}

pub(crate) fn looked_at<'a>(
    snapshot: Option<&'a PageSnapshot>,
    doing: &'static str,
) -> Result<&'a PageSnapshot, AxError> {
    snapshot.ok_or_else(|| {
        AxError::failure(AxCode::InvalidArgs, doing, "nobody has looked at it yet")
            .with_recovery("take a snapshot first; a reference is a position in one")
    })
}

fn origin_mismatch() -> AxError {
    AxError::failure(
        AxCode::InvalidArgs,
        "act on a page",
        "the resolved origin does not match the action",
    )
    .with_recovery(
        "report this against browser::verb: a drag from a reference is resolved before its \
         input frame is built, and nothing else is",
    )
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
