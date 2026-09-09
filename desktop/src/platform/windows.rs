// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The Windows arm: the desk one connection holds, and which of the six
//! carries out an admitted call.
//!
//! Everything above this file has already been decided by the time a
//! call arrives: `crate::scope` has said the window is one this server
//! may touch, and `crate::session` has said the tool exists and the
//! handshake finished. What is left is reading the arguments the schema
//! promised and handing them to the module that does the work.
//!
//! **Two things live across calls, and both are here rather than in a
//! global**: the snapshot generations, so an action decided against an
//! old view can be refused, and the recordings, so a `stop` knows what a
//! `start` began. Both belong to one connection, which is why the desk
//! is a value the session owns rather than process state — a second
//! connection to a second scope file must not be able to stop the first
//! one's recording.
//!
//! Reading arguments is deliberately strict. A missing `action`, an
//! unknown format, a `point` outside the window: each is refused by name
//! rather than defaulted, because every default here is a guess about
//! what somebody wanted to happen on their own desktop.

mod act;
mod capture;
mod clipboard;
mod encode;
mod enumerate;
mod fault;
mod geometry;
mod keys;
mod reading;
mod record;
mod target;
mod tree;
mod views;

use serde_json::{Value, json};

use crate::refusal::{Refusal, RefusalCode};
use act::Action;
use geometry::Point;
use keys::Modifier;
use reading::{asked_for, generation, inset, missing, notches, region, text, whole};
use views::{DEFAULT_DEPTH, Views};

/// What one connection remembers between calls.
#[derive(Default)]
pub(crate) struct Desk {
    views: Views,
    recordings: record::Recordings,
}

impl Desk {
    pub(crate) fn new() -> Desk {
        Desk {
            views: Views::new(),
            recordings: record::Recordings::new(),
        }
    }

    /// Carries out one admitted call.
    ///
    /// # Errors
    /// Refuses arguments the schema allows but this machine cannot act
    /// on, a window this desktop does not have, an action decided
    /// against a view that has moved on, and anything the operating
    /// system itself refuses.
    pub(crate) fn perform(&mut self, tool: &str, arguments: &Value) -> Result<Value, Refusal> {
        match tool {
            "desktop.windows" => listing(arguments),
            "desktop.snapshot" => self.snapshot(arguments),
            "desktop.act" => self.act(arguments),
            "desktop.screenshot" => screenshot(arguments),
            "desktop.record" => self.record(arguments),
            "desktop.clipboard" => use_clipboard(arguments),
            unknown => Err(Refusal::new(
                RefusalCode::ToolUnknown,
                "use the desktop",
                format!("`{unknown}` is not a tool this server offers"),
                "read `tools/list`; this server offers six tools and no others",
            )),
        }
    }

    /// `desktop.snapshot`: one window's tree, and the generation the
    /// refs in it belong to.
    fn snapshot(&mut self, arguments: &Value) -> Result<Value, Refusal> {
        let window = resolved(arguments)?;
        let depth = whole(arguments, "depth")?.unwrap_or(DEFAULT_DEPTH);
        let nodes = tree::read(window.handle, depth)?;
        let described: Vec<Value> = nodes
            .iter()
            .map(|node| {
                json!({
                    "ref": node.reference,
                    "role": node.role,
                    "name": node.name,
                    "depth": node.depth,
                    "bounds": node.bounds.as_json(),
                })
            })
            .collect();
        let generation = self.views.mint(&window.named.title, nodes);
        Ok(json!({
            "title": window.named.title,
            "process": window.named.process,
            "generation": generation,
            "nodes": described,
        }))
    }

    /// `desktop.act`: one action, at a ref from a snapshot or at a point
    /// inside the window.
    fn act(&mut self, arguments: &Value) -> Result<Value, Refusal> {
        let window = resolved(arguments)?;
        let named = text(arguments, "action").ok_or_else(|| {
            Refusal::new(
                RefusalCode::InvalidArgs,
                "act on a window",
                "the call names no action".to_owned(),
                "send `action`, one of click, double, right, drag, scroll, type or key",
            )
        })?;
        let modifiers = held(arguments)?;
        let action = self.decided(&window, named, arguments)?;
        act::perform(&action, &modifiers)?;
        Ok(json!({
            "title": window.named.title,
            "action": named,
            "generation": self.views.generation(&window.named.title),
        }))
    }

    /// Which action this call is, with every coordinate already resolved
    /// to a place on the screen.
    fn decided(
        &self,
        window: &enumerate::Window,
        named: &str,
        arguments: &Value,
    ) -> Result<Action, Refusal> {
        let at = self.where_at(window, arguments, "point")?;
        Ok(match named {
            "click" => Action::Click { at: at? },
            "double" => Action::Double { at: at? },
            "right" => Action::Right { at: at? },
            "drag" => Action::Drag {
                from: at?,
                to: self.where_at(window, arguments, "to")??,
            },
            "scroll" => Action::Scroll {
                at: at?,
                notches: notches(arguments),
            },
            "type" => Action::Type {
                text: text(arguments, "text")
                    .ok_or_else(|| missing("type", "text", "the text to type"))?
                    .to_owned(),
            },
            "key" => Action::Key {
                code: keys::code(
                    text(arguments, "key")
                        .ok_or_else(|| missing("key", "key", "the key to press"))?,
                )?,
            },
            other => {
                return Err(Refusal::new(
                    RefusalCode::InvalidArgs,
                    "act on a window",
                    format!("`{other}` is not an action this server carries out"),
                    "send one of click, double, right, drag, scroll, type or key",
                ));
            }
        })
    }

    /// Where an action lands: the middle of a ref from the current
    /// generation, or a point measured inside the window.
    ///
    /// The inner `Result` is deferred on purpose. `type` and `key` need
    /// no place at all, so a call that names neither a ref nor a point
    /// is only refused for the actions that would have used one.
    fn where_at(
        &self,
        window: &enumerate::Window,
        arguments: &Value,
        field: &str,
    ) -> Result<Result<Point, Refusal>, Refusal> {
        if let Some(reference) = text(arguments, "ref") {
            let generation = generation(arguments)?.ok_or_else(|| {
                Refusal::new(
                    RefusalCode::InvalidArgs,
                    "act on a window",
                    "a ref was given without the generation it was decided against".to_owned(),
                    "send `generation` from the `desktop.snapshot` that minted this ref; without \
                     it the action could land on whatever moved into that position",
                )
            })?;
            let node = self
                .views
                .resolve(&window.named.title, generation, reference)?;
            return Ok(node.bounds.centre());
        }
        let Some(inset) = inset(arguments, field) else {
            return Ok(Err(Refusal::new(
                RefusalCode::InvalidArgs,
                "act on a window",
                format!("the call names neither a `ref` nor a `{field}`"),
                "take a `desktop.snapshot` and act on a ref from it, or send a point measured \
                 from the window's own top-left corner",
            )));
        };
        Ok(window.bounds.at(inset))
    }

    /// `desktop.record`: start or stop.
    fn record(&mut self, arguments: &Value) -> Result<Value, Refusal> {
        let window = resolved(arguments)?;
        let state = text(arguments, "state").ok_or_else(|| {
            Refusal::new(
                RefusalCode::InvalidArgs,
                "record a window",
                "the call says neither start nor stop".to_owned(),
                "send `state: start` or `state: stop`",
            )
        })?;
        match state {
            "start" => {
                let audio = arguments
                    .get("audio")
                    .and_then(Value::as_bool)
                    .unwrap_or(false);
                let into = self.recordings.start(
                    &window.named.title,
                    window.handle,
                    window.bounds,
                    audio,
                )?;
                Ok(json!({
                    "state": "started",
                    "title": window.named.title,
                    "into": into.display().to_string(),
                }))
            }
            "stop" => {
                let (into, how) = self.recordings.stop(&window.named.title)?;
                Ok(json!({
                    "state": "stopped",
                    "title": window.named.title,
                    "into": into.display().to_string(),
                    "as": how,
                }))
            }
            other => Err(Refusal::new(
                RefusalCode::InvalidArgs,
                "record a window",
                format!("`{other}` is neither start nor stop"),
                "send `state: start` or `state: stop`",
            )),
        }
    }
}

/// The modifiers this call holds down for the whole of its action.
///
/// An unknown one is refused rather than dropped: a caller that asked
/// for `ctrl` and got a bare click has had a different thing happen than
/// the one it asked for, and nothing would say so.
fn held(arguments: &Value) -> Result<Vec<Modifier>, Refusal> {
    let Some(named) = arguments.get("modifiers") else {
        return Ok(Vec::new());
    };
    let Some(listed) = named.as_array() else {
        return Err(Refusal::new(
            RefusalCode::InvalidArgs,
            "act on a window",
            "`modifiers` is not a list".to_owned(),
            "send `modifiers` as an array of ctrl, alt, shift or win",
        ));
    };
    listed
        .iter()
        .map(|one| {
            one.as_str()
                .ok_or_else(|| {
                    Refusal::new(
                        RefusalCode::InvalidArgs,
                        "act on a window",
                        "a modifier in the list is not a name".to_owned(),
                        "send `modifiers` as an array of ctrl, alt, shift or win",
                    )
                })
                .and_then(Modifier::parse)
        })
        .collect()
}

/// `desktop.windows`: what this scope's caller may name.
fn listing(arguments: &Value) -> Result<Value, Refusal> {
    let wanted = text(arguments, "process").map(crate::scope::Pattern::new);
    let reported: Vec<Value> = enumerate::desktop()?
        .into_iter()
        .filter(|window| {
            wanted
                .as_ref()
                .is_none_or(|glob| glob.matches(&window.named.process))
        })
        .enumerate()
        .map(|(at, window)| {
            json!({
                // A name for this window in this connection's answers.
                // It is deliberately not accepted back by the other
                // tools: the scope file judges a title and a process, so
                // a second way to name a window would be a second door
                // onto the same permission (desktop-SPEC.md §8.6, third
                // pair). It carries no native handle for the same reason.
                "ref": format!("w{}", at.saturating_add(1)),
                "title": window.named.title,
                "process": window.named.process,
                "bounds": window.bounds.as_json(),
            })
        })
        .collect();
    Ok(json!({ "windows": reported }))
}

/// `desktop.screenshot`: one window, or a region of it.
fn screenshot(arguments: &Value) -> Result<Value, Refusal> {
    let window = resolved(arguments)?;
    let whole = capture::window(window.handle, window.bounds)?;
    let pixels = match region(arguments)? {
        Some((left, top, width, height)) => {
            if left.saturating_add(width) > whole.width()
                || top.saturating_add(height) > whole.height()
            {
                return Err(Refusal::new(
                    RefusalCode::InvalidArgs,
                    "capture a window",
                    format!(
                        "that region leaves a window that is {}x{}",
                        whole.width(),
                        whole.height()
                    ),
                    "ask for a region inside the bounds `desktop.windows` reports for this window",
                ));
            }
            image::imageops::crop_imm(&whole, left, top, width, height).to_image()
        }
        None => whole,
    };
    let mut answer = encode::render(&pixels, asked_for(arguments)?)?;
    if let Some(object) = answer.as_object_mut() {
        object.insert("title".to_owned(), json!(window.named.title));
    }
    Ok(answer)
}

/// `desktop.clipboard`: get or set text.
fn use_clipboard(arguments: &Value) -> Result<Value, Refusal> {
    let operation = text(arguments, "operation").ok_or_else(|| {
        Refusal::new(
            RefusalCode::InvalidArgs,
            "use the clipboard",
            "the call says neither get nor set".to_owned(),
            "send `operation: get` or `operation: set`",
        )
    })?;
    match operation {
        "get" => Ok(json!({ "operation": "get", "text": clipboard::read()? })),
        "set" => {
            let text = text(arguments, "text").ok_or_else(|| {
                Refusal::new(
                    RefusalCode::InvalidArgs,
                    "use the clipboard",
                    "`set` was asked for without any text".to_owned(),
                    "send `text` alongside `operation: set`",
                )
            })?;
            clipboard::write(text)?;
            Ok(json!({ "operation": "set" }))
        }
        other => Err(Refusal::new(
            RefusalCode::InvalidArgs,
            "use the clipboard",
            format!("`{other}` is neither get nor set"),
            "send `operation: get` or `operation: set`",
        )),
    }
}

/// The one window this call named.
fn resolved(arguments: &Value) -> Result<enumerate::Window, Refusal> {
    let mut open = enumerate::desktop()?;
    let named: Vec<target::Named> = open.iter().map(|window| window.named.clone()).collect();
    let at = target::choose(&named, text(arguments, "title"), text(arguments, "process"))?;
    if at >= open.len() {
        return Err(Refusal::new(
            RefusalCode::ToolUnavailable,
            "use the desktop",
            "the window list changed while it was being read".to_owned(),
            "try again",
        ));
    }
    Ok(open.swap_remove(at))
}
