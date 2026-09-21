// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The eight actions the `browser` tool offers, read from arguments and
//! turned into frames.
//!
//! One action can need more than one frame — opening a page navigates
//! and then installs the console recorder, a screenshot at a different
//! scale changes the device pixel ratio first — so the answer is a list.
//! Nothing here sends anything: the caller owns the socket, and this
//! module owns which bytes it should put on it.

mod read;

use kernel::{AxCode, AxError, Payload};
use serde_json::{Value, json};

use crate::act::{Action, Origin, frame_for, resolve_frame};
use crate::port::Frame;
use crate::session::{ContextId, Session};
use crate::shot::ShotRequest;
use crate::snapshot::PageSnapshot;
use read::{missing, number_of, read_action, side_of, text_of};

/// Installs the console recorder. Wrapping rather than replacing keeps
/// the page's own logging working, which matters because the thing being
/// developed is often the page.
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

/// Reads the recorder back. A string comes home rather than a structure,
/// because the reply's own value shape is the driver's and a string is
/// one thing to parse instead of two.
const CONSOLE_SCRIPT: &str = "JSON.stringify(window.__sprawling_console || [])";

/// How much of an accessible name the page is asked for. A ceiling on
/// what crosses the socket, not the label a model reads: the snapshot
/// cuts that to its own handle length.
const NAME_MAX_JS: usize = 200;

/// Collects the accessibility tree the snapshot reads. Role first,
/// accessible name second, and nothing else crosses.
///
/// The role an element states wins; otherwise the element is looked up
/// in the table `snapshot` keeps, and an element the table does not
/// name has no role and is dropped by the snapshot. Generated rather
/// than written out, because a page that calls a `<select>` something
/// other than what the filter looks for is a page with no controls in
/// it.
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

/// One thing a run wants the browser to do.
///
/// Exhaustive, and each variant is one of the eight names the tool
/// offers. A ninth action is a change here, not a string that reaches a
/// page.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Verb {
    Open { url: String },
    Snapshot,
    Act { generation: u64, action: Action },
    Screenshot(ShotRequest),
    Measure { references: Vec<String> },
    Console,
    Viewport { width: u32, height: u32 },
    Close,
}

impl Verb {
    /// The host this call's bytes would leave for, when it names one.
    ///
    /// The tool's own grammar answers this; the bench no longer reads an
    /// argument name by hand (M-17). A URL with no authority —
    /// `about:blank`, a `file:` path — names no host, so there is no
    /// egress to judge.
    ///
    /// # Errors
    /// Refuses an authority this build cannot read, which would leave an
    /// egress unjudged.
    pub fn destination(&self) -> Result<Option<String>, AxError> {
        match self {
            Verb::Open { url } => kernel::gate::host_of(url),
            Verb::Snapshot
            | Verb::Act { .. }
            | Verb::Screenshot(_)
            | Verb::Measure { .. }
            | Verb::Console
            | Verb::Viewport { .. }
            | Verb::Close => Ok(None),
        }
    }

    /// The frame an input action becomes once its origin is resolved.
    ///
    /// `origin` is `Some` exactly when [`Action::resolves_element`] said
    /// so; a drag from a point takes the point out of its own arguments
    /// and needs nothing between the frames.
    ///
    /// # Errors
    /// Refuses an action that is not a pointer action, and a resolved
    /// origin that does not match the action's own.
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
            Verb::Open { .. }
            | Verb::Snapshot
            | Verb::Act { .. }
            | Verb::Screenshot(_)
            | Verb::Measure { .. }
            | Verb::Console
            | Verb::Viewport { .. }
            | Verb::Close => Err(AxError::failure(
                AxCode::InvalidArgs,
                "act on a page",
                "this action is not a pointer action",
            )
            .with_recovery(
                "report this against browser::verb: only drag and scroll have input frames",
            )),
        }
    }

    /// Reads one call's arguments.
    ///
    /// # Errors
    /// Refuses an action name this version does not offer, and a
    /// required field that is absent or of the wrong kind. Nothing is
    /// defaulted: a click with no reference is a mistake, and inventing
    /// one would act on a page nobody named.
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
                "one of open, snapshot, act, screenshot, measure, console, viewport, close",
            )),
        }
    }

    /// The frames this action puts on the wire, in the order they go.
    ///
    /// # Errors
    /// Refuses an action that needs a page nobody has looked at — acting
    /// and measuring both name references a snapshot minted — and
    /// propagates the refusals of the frames themselves.
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
                Action::Drag { .. } | Action::Scroll { .. } => {
                    let looked = looked_at(snapshot, "act on a page")?;
                    if action.resolves_element() {
                        let Some(reference) = action.reference() else {
                            return Err(origin_mismatch());
                        };
                        // The page names the element first; the input
                        // frame follows once its reply arrives.
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
            Verb::Screenshot(request) => request.frames(session, context),
            Verb::Measure { references } => Ok(vec![session.evaluate(
                context,
                &measure_script(looked_at(snapshot, "measure a page")?, references)?,
            )?]),
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

fn looked_at<'a>(
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

fn read_references(args: &Payload) -> Result<Vec<String>, AxError> {
    let raw = args
        .as_map()
        .get("refs")
        .and_then(Value::as_array)
        .ok_or_else(|| missing("refs"))?;
    let mut references = Vec::new();
    for entry in raw {
        let text = entry.as_str().ok_or_else(|| {
            AxError::failure(
                AxCode::InvalidArgs,
                "read a browser action",
                "a reference that is not a string",
            )
            .with_recovery("references are minted by the snapshot and look like `e1`")
        })?;
        references.push(text.to_owned());
    }
    Ok(references)
}

/// Builds the expression that measures the named nodes.
///
/// Every reference goes through the snapshot first, so a reference the
/// model invented is refused here rather than answered with a box that
/// belongs to something else.
fn measure_script(snapshot: &PageSnapshot, references: &[String]) -> Result<String, AxError> {
    let mut parts = Vec::new();
    for reference in references {
        snapshot.resolve(reference)?;
        parts.push(format!(
            "(el => {{ const r = el ? el.getBoundingClientRect() : null; return r ? {{ ref: {}, \
             x: Math.round(r.x), y: Math.round(r.y), width: Math.round(r.width), height: \
             Math.round(r.height) }} : {{ ref: {}, x: 0, y: 0, width: 0, height: 0 }}; }})({})",
            json!(reference),
            json!(reference),
            crate::act::selector_of(snapshot, reference)?,
        ));
    }
    Ok(format!("JSON.stringify([{}])", parts.join(",")))
}

/// Reads what an evaluate came back with.
///
/// Every script this module sends returns one JSON string, so there is
/// one shape to read rather than the driver's whole remote-value
/// vocabulary.
///
/// # Errors
/// Refuses a reply whose shape this version does not read, and a string
/// that is not the JSON the script promised.
pub fn read_json(result: &Value) -> Result<Value, AxError> {
    let text = result
        .get("result")
        .and_then(|inner| inner.get("value"))
        .and_then(Value::as_str)
        .ok_or_else(|| {
            AxError::failure(
                AxCode::WireMismatch,
                "read a script result",
                "no string value",
            )
            .with_recovery("check the driver's protocol version")
        })?;
    serde_json::from_str(text).map_err(|err| {
        AxError::failure(
            AxCode::WireMismatch,
            "read a script result",
            err.to_string(),
        )
        .with_recovery("the script returns JSON.stringify of its answer")
    })
}

/// Whether the console said anything the development loop should stop
/// for. Error level only: a warning is the page talking, an error is the
/// page failing.
#[must_use]
pub fn complained(console: &Value) -> bool {
    console.as_array().is_some_and(|entries| {
        entries
            .iter()
            .any(|entry| entry.get("level").and_then(Value::as_str) == Some("error"))
    })
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
