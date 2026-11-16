// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::collections::BTreeSet;

use crate::keyboard::{Key, Modifier};
use crate::port::Frame;
use crate::session::{ContextId, Session};
use crate::snapshot::PageSnapshot;
use kernel::{AxCode, AxError};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Point {
    pub x: i64,
    pub y: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Origin {

    Reference(String),

    Point(Point),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    Click {
        reference: String,
    },
    Type {
        reference: String,
        text: String,
    },
    Read {
        reference: String,
    },

    Drag {
        from: Origin,
        to: Point,
        steps: u32,
    },

    Scroll {
        at: Option<Point>,
        by: Point,
    },

    Press {
        key: Key,
        modifiers: BTreeSet<Modifier>,
    },
}

pub const STEPS_MAX: u32 = 32;

impl Action {

    #[must_use]
    pub fn reference(&self) -> Option<&str> {
        match self {
            Action::Click { reference }
            | Action::Type { reference, .. }
            | Action::Read { reference } => Some(reference),
            Action::Drag {
                from: Origin::Reference(reference),
                ..
            } => Some(reference),
            Action::Drag {
                from: Origin::Point(_),
                ..
            }
            | Action::Scroll { .. }
            | Action::Press { .. } => None,
        }
    }

    #[must_use]
    pub fn resolves_element(&self) -> bool {
        matches!(
            self,
            Action::Drag {
                from: Origin::Reference(_),
                ..
            }
        )
    }
}

fn quote(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len().saturating_add(2));
    out.push('\'');
    for ch in raw.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            '\'' => out.push_str("\\'"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\u{2028}' => out.push_str("\\u2028"),
            '\u{2029}' => out.push_str("\\u2029"),
            other => out.push(other),
        }
    }
    out.push('\'');
    out
}

pub fn frame_for(
    session: &mut Session,
    context: &ContextId,
    snapshot: &PageSnapshot,
    generation: u64,
    action: &Action,
) -> Result<Frame, AxError> {
    ensure_fresh(snapshot, generation)?;
    let expression = match action {
        Action::Click { reference } => format!("{}.click()", selector_of(snapshot, reference)?),
        Action::Type { reference, text } => {
            let selector = selector_of(snapshot, reference)?;
            format!(
                "(el => {{ el.value = {}; el.dispatchEvent(new Event('input', {{ bubbles: true }})); }})({selector})",
                quote(text)
            )
        }
        Action::Read { reference } => {
            format!("({}).textContent", selector_of(snapshot, reference)?)
        }
        Action::Drag { .. } | Action::Scroll { .. } | Action::Press { .. } => {
            return Err(AxError::failure(
                AxCode::InvalidArgs,
                "act on a page",
                "an input action is not a script",
            )
            .with_recovery(
                "report this against browser::act: input actions are built by browser::input \
                 and browser::keyboard",
            ));
        }
    };
    session.evaluate(context, &expression)
}

pub fn resolve_frame(
    session: &mut Session,
    context: &ContextId,
    snapshot: &PageSnapshot,
    reference: &str,
) -> Result<Frame, AxError> {
    session.evaluate(context, &selector_of(snapshot, reference)?)
}

pub(crate) fn ensure_fresh(snapshot: &PageSnapshot, generation: u64) -> Result<(), AxError> {
    if generation == snapshot.generation() {
        return Ok(());
    }
    Err(AxError::failure(
        AxCode::InvalidArgs,
        "act on a page",
        format!(
            "the action was decided against snapshot {generation}, the page is at {}",
            snapshot.generation()
        ),
    )
    .with_recovery("take a fresh snapshot and decide again"))
}

pub(crate) fn selector_of(snapshot: &PageSnapshot, reference: &str) -> Result<String, AxError> {
    let node = snapshot.resolve(reference)?;
    let ordinal = index_of(reference)?;
    let among = snapshot
        .nodes()
        .iter()
        .take(ordinal)
        .filter(|earlier| earlier.role == node.role)
        .count();
    Ok(format!(
        "[...document.querySelectorAll('*')].filter(e => (e.getAttribute('role') || \
         e.tagName.toLowerCase()) === {})[{among}]",
        quote(&node.role),
    ))
}

fn index_of(reference: &str) -> Result<usize, AxError> {
    let digits = reference.strip_prefix('e').ok_or_else(|| {
        AxError::failure(
            AxCode::InvalidArgs,
            "read a page reference",
            reference.to_owned(),
        )
        .with_recovery("references are minted by the snapshot and look like `e1`")
    })?;
    let ordinal: usize = digits.parse().map_err(|_| {
        AxError::failure(
            AxCode::InvalidArgs,
            "read a page reference",
            reference.to_owned(),
        )
        .with_recovery("references are minted by the snapshot and look like `e1`")
    })?;
    ordinal.checked_sub(1).ok_or_else(|| {
        AxError::failure(AxCode::InvalidArgs, "read a page reference", "e0")
            .with_recovery("references start at e1")
    })
}

#[cfg(test)]
#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "test code"
)]
mod tests;
