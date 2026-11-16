// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

mod act;
mod capture;
mod clipboard;
mod dpi;
mod encode;
mod enumerate;
mod fault;
#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "test code"
)]
mod fixture;
mod focus;
mod geometry;
mod keys;
mod reading;
mod record;
mod strokes;
mod target;
mod tree;
mod views;

use serde_json::{Value, json};

use crate::answer::Answer;
use crate::outline::{self, Line};
use crate::refusal::{Refusal, RefusalCode};
use crate::scope::Admitted;
use crate::tools::ToolName;
use geometry::Point;
use reading::{asked_for, generation, held, inset, missing, notches, region, text, whole};
use strokes::Action;
use views::{DEFAULT_DEPTH, Views};

pub(crate) struct Desk {
    views: Views,
    recordings: record::Recordings,

    pixels: Result<dpi::PhysicalPixels, Refusal>,

    reader: Option<tree::Reader>,
}

impl Desk {
    pub(crate) fn new() -> Desk {
        Desk {
            pixels: dpi::declare(),
            views: Views::new(),
            recordings: record::Recordings::new(),
            reader: None,
        }
    }

    pub(crate) fn perform(
        &mut self,
        tool: ToolName,
        arguments: &Value,
        admitted: &Admitted<'_>,
    ) -> Result<Answer, Refusal> {
        self.pixels.as_ref().map_err(Clone::clone)?;
        match tool {
            ToolName::Windows => listing(arguments, admitted).map(|facts| Answer::facts(&facts)),
            ToolName::Snapshot => self.snapshot(arguments),
            ToolName::Act => self.act(arguments).map(|facts| Answer::facts(&facts)),
            ToolName::Screenshot => screenshot(arguments),
            ToolName::Record => self.record(arguments, admitted),
            ToolName::Clipboard => use_clipboard(arguments).map(|facts| Answer::facts(&facts)),
        }
    }

    fn snapshot(&mut self, arguments: &Value) -> Result<Answer, Refusal> {
        let window = resolved(arguments)?;
        let depth = whole(arguments, "depth")?.unwrap_or(DEFAULT_DEPTH);
        let reader = match self.reader.take() {
            Some(started) => started,
            None => tree::Reader::start()?,
        };
        let read = reader.read(&window.handle, depth);
        self.reader = Some(reader);
        let walked = read?;
        let text = outline::fold(
            walked.nodes.iter().map(|node| Line {
                reference: &node.reference,
                role: node.role,
                name: &node.name,
                depth: node.depth,
            }),
            &walked.ending,
        );

        let generation = self.views.mint(sight(&window), walked.nodes);
        Ok(Answer::outline(
            text,
            &json!({
                "title": window.named.title,
                "process": window.named.process,
                "generation": generation,
            }),
        ))
    }

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

        focus::hold(&window.handle, action.lands_at())?;
        act::perform(&action, &modifiers)?;
        Ok(json!({
            "title": window.named.title,
            "action": named,
            "generation": self.views.generation(sight(&window).aim),
        }))
    }

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
            let node = self.views.resolve(sight(window), generation, reference)?;
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

    fn record(&mut self, arguments: &Value, admitted: &Admitted<'_>) -> Result<Answer, Refusal> {
        match record::asked(arguments)? {
            record::Wanted::Start(sound) => {
                let device = match sound {
                    record::Sound::Silent => None,
                    record::Sound::Asked => Some(admitted.sound()?),
                };
                self.recordings
                    .start(resolved(arguments)?, device)
                    .map(|facts| Answer::facts(&facts))
            }
            record::Wanted::Stop(id) => self.recordings.stop(id),
        }
    }
}

fn listing(arguments: &Value, admitted: &Admitted<'_>) -> Result<Value, Refusal> {
    let wanted = text(arguments, "process").map(crate::scope::Pattern::new);
    let reported: Vec<Value> = enumerate::desktop()?
        .into_iter()
        .filter(|window| admitted.visible(&window.named.title, &window.named.process))
        .filter(|window| {
            wanted
                .as_ref()
                .is_none_or(|glob| glob.matches(&window.named.process))
        })
        .enumerate()
        .map(|(at, window)| {
            json!({

                "ref": format!("w{}", at.saturating_add(1)),
                "title": window.named.title,
                "process": window.named.process,
                "bounds": window.bounds.as_json(),
            })
        })
        .collect();
    Ok(json!({ "windows": reported }))
}

fn screenshot(arguments: &Value) -> Result<Answer, Refusal> {
    let window = resolved(arguments)?;
    let whole = capture::window(&window.handle, window.bounds)?;
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
    let encoded = encode::render(&pixels, asked_for(arguments)?)?;
    let facts = json!({
        "title": window.named.title,
        "width": encoded.width,
        "height": encoded.height,

        "lossless": encoded.lossless,
    });
    Ok(Answer::picture(encoded.bytes, encoded.mime, &facts))
}

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

fn sight(window: &enumerate::Window) -> views::Sight {
    views::Sight {
        aim: focus::Aim::of(&window.handle),
        bounds: window.bounds,
    }
}

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
