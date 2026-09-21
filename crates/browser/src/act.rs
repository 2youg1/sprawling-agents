// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Turning an intention into frames.
//!
//! Every action names a reference from the snapshot the caller is
//! looking at, so acting on a page nobody looked at is not expressible.
//! The generation travels with the action for the same reason: a click
//! decided against one view of a page is refused against another rather
//! than landing on whatever moved into that position.

use crate::port::Frame;
use crate::session::{ContextId, Session};
use crate::snapshot::PageSnapshot;
use kernel::{AxCode, AxError};

/// A viewport position in CSS pixels. Negative `y` in a scroll delta is
/// a wheel turned up; a pointer coordinate outside the viewport is the
/// page's own business rather than something to clamp here.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Point {
    pub x: i64,
    pub y: i64,
}

/// Where a pointer action starts.
///
/// Exhaustive: either the snapshot named the element, or the caller
/// named a point. A drag that starts nowhere is not expressible, which
/// is the point of the type.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Origin {
    /// A reference the snapshot minted. The page is asked for the
    /// element's own id before the input frame is built, because BiDi
    /// names an input origin by element reference and not by selector.
    Reference(String),
    /// A viewport point.
    Point(Point),
}

/// What a run wants done. Exhaustive: an action this crate cannot spell
/// should be a compile error at the caller, not a string that reaches a
/// page.
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
    /// A pointer press at `from`, moved to `to`, released. `steps` is
    /// how many intermediate moves the pointer makes on the way, which
    /// is what a page that tracks a drag reads.
    Drag {
        from: Origin,
        to: Point,
        steps: u32,
    },
    /// One wheel turn at `at` (or at the viewport's own origin), by
    /// `by` in CSS pixels.
    Scroll {
        at: Option<Point>,
        by: Point,
    },
}

/// How many intermediate pointer moves one drag may make. A ceiling on
/// what crosses the socket, not a taste: a page that needs more than
/// this to notice the drag is a page a person cannot drag either.
pub const STEPS_MAX: u32 = 32;

impl Action {
    /// The snapshot reference this action names, when it names one.
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
            | Action::Scroll { .. } => None,
        }
    }

    /// Whether this action needs the page to name an element between
    /// two frames before its input frame can be built.
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

/// Escapes a string for a JavaScript single-quoted literal. Page text is
/// somebody else's, and the one place it becomes code is here.
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

/// Builds the frame that performs `action` against the page `snapshot`
/// describes.
///
/// Pointer actions ([`Action::Drag`], [`Action::Scroll`]) do not come
/// through here: they are BiDi `input.performActions`, which is a
/// different module of the protocol, and they are built by
/// [`crate::input`]. The refusal below is a guard against a caller that
/// routes one here, not a state a run reaches.
///
/// # Errors
/// Refuses a reference the snapshot did not mint, one minted against
/// a different generation — the second is the stale-page case, and it is
/// refused rather than retried because the caller has to look again to
/// know what it is now clicking — and an input action routed here.
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
        Action::Drag { .. } | Action::Scroll { .. } => {
            return Err(AxError::failure(
                AxCode::InvalidArgs,
                "act on a page",
                "a pointer action is not a script",
            )
            .with_recovery(
                "report this against browser::act: pointer actions are built by browser::input",
            ));
        }
    };
    session.evaluate(context, &expression)
}

/// The frame whose reply names the element a pointer action starts from.
///
/// BiDi's `input.performActions` names an element origin by the page's
/// own shared id, which only a `script.evaluate` that returns the node
/// carries back. The caller sends this frame, reads
/// [`crate::input::shared_id_of`] out of the reply, and then builds the
/// input frame.
///
/// # Errors
/// Propagates a reference the snapshot did not mint.
pub fn resolve_frame(
    session: &mut Session,
    context: &ContextId,
    snapshot: &PageSnapshot,
    reference: &str,
) -> Result<Frame, AxError> {
    session.evaluate(context, &selector_of(snapshot, reference)?)
}

/// Refuses a decision made against a page that has since changed.
///
/// One authority for the rule: `frame_for` and the element-origin resolve
/// both ask it, so a caller that checks freshness once checks it the same
/// way the other arm does.
///
/// # Errors
/// Refuses a generation older or newer than the snapshot's own.
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

/// The expression that names one node of `snapshot` in the page.
///
/// One authority for the whole crate: acting on a node and measuring one
/// have to reach the same element, and two spellings of "which element"
/// would drift the moment either was corrected.
///
/// The page's own text goes in as data through [`quote`], never as code,
/// and the position is counted among the nodes sharing this node's role
/// — which is the list the expression filters, so the two agree.
///
/// # Errors
/// Propagates a reference this snapshot did not mint.
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

/// The zero-based position a reference names. References are minted as
/// `e1`, `e2`, … by the snapshot, so this is the inverse of that.
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
