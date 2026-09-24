// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Which region a screenshot covers.
//!
//! One region, two spellings, never both: a rectangle the caller named
//! in CSS pixels, or an element a snapshot named. An element's box is
//! reported by the page to the driver rather than measured here, so
//! nothing rounds and there is no second measurement that could disagree
//! with the picture.

use kernel::{AxCode, AxError, Payload};
use serde_json::{Value, json};

/// The rectangle of the page a screenshot covers, in CSS pixels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rect {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

/// The element a screenshot covers, as a snapshot named it.
///
/// The generation travels with the reference because a reference is a
/// position in one snapshot: `act::ensure_fresh` is the one authority on
/// that rule, and this carries what it judges.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Element {
    reference: String,
    generation: u64,
}

impl Element {
    #[must_use]
    pub(crate) fn new(reference: String, generation: u64) -> Element {
        Element {
            reference,
            generation,
        }
    }

    #[must_use]
    pub(crate) fn reference(&self) -> &str {
        &self.reference
    }

    #[must_use]
    pub(crate) fn generation(&self) -> u64 {
        self.generation
    }
}

/// The elements a screenshot covers together, as a snapshot named them.
///
/// The boxes are the ones `measure` reads — the same script, the same
/// rounding — so the region a shot covers and the region a measurement
/// reports cannot disagree. A union is the one clip the protocol cannot
/// state, so it is measured first and then captured as a rectangle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Union {
    references: Vec<String>,
    generation: u64,
}

impl Union {
    #[must_use]
    pub(crate) fn new(references: Vec<String>, generation: u64) -> Union {
        Union {
            references,
            generation,
        }
    }

    #[must_use]
    pub(crate) fn references(&self) -> &[String] {
        &self.references
    }

    #[must_use]
    pub(crate) fn generation(&self) -> u64 {
        self.generation
    }

    /// The rectangle covering every box the page reported.
    ///
    /// # Errors
    /// Refuses a reply that carries no boxes, and one whose numbers this
    /// build cannot read — an element that moved off the page is a box of
    /// zero, which is still a box, so only a broken answer refuses.
    pub(crate) fn around(reply: &Value) -> Result<Rect, AxError> {
        let boxes = crate::verb::read_json(reply)?;
        let mut union: Option<Rect> = None;
        for entry in boxes.as_array().ok_or_else(no_boxes)?.iter() {
            let rect = Rect {
                x: side(entry, "x")?,
                y: side(entry, "y")?,
                width: side(entry, "width")?,
                height: side(entry, "height")?,
            };
            union = Some(match union {
                None => rect,
                Some(held) => held.covering(rect)?,
            });
        }
        union.ok_or_else(no_boxes)
    }
}

/// One side of one reported box.
///
/// # Errors
/// Refuses a box missing a side, and a number that is not a whole one
/// inside a rectangle's range.
fn side(entry: &Value, field: &str) -> Result<u32, AxError> {
    let raw = entry.get(field).and_then(Value::as_u64).ok_or_else(|| {
        AxError::failure(
            AxCode::WireMismatch,
            "measure a page",
            format!("a box came back without `{field}`"),
        )
        .with_recovery("check the driver's protocol version, then this crate's box script")
    })?;
    u32::try_from(raw).map_err(|_| {
        AxError::failure(
            AxCode::WireMismatch,
            "measure a page",
            format!("`{field}` is larger than a page can be"),
        )
        .with_recovery("check the driver's protocol version, then this crate's box script")
    })
}

fn no_boxes() -> AxError {
    AxError::failure(
        AxCode::InvalidArgs,
        "photograph a page",
        "no boxes came back to cover",
    )
    .with_recovery("name at least one reference the snapshot minted")
}

/// The reference a call names, when it names one.
///
/// # Errors
/// Refuses a `ref` that is present and is not a reference: silently
/// ignoring it would photograph the whole page while the caller believed
/// it had asked for one element.
pub(crate) fn reference_of(args: &Payload) -> Result<Option<String>, AxError> {
    match args.as_map().get("ref") {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(reference)) if !reference.is_empty() => Ok(Some(reference.clone())),
        Some(Value::String(_)) | Some(_) => Err(AxError::failure(
            AxCode::InvalidArgs,
            "read a screenshot request",
            "`ref` is not a reference",
        )
        .with_recovery("a reference is minted by the snapshot and looks like `e12`")),
    }
}

/// The references a call names to cover together, when it names any.
///
/// # Errors
/// Refuses `refs` that is not a list of references, and a list with
/// nothing in it: a union of no boxes has no region to cover.
pub(crate) fn references_of(args: &Payload) -> Result<Option<Vec<String>>, AxError> {
    let not_a_list = || {
        AxError::failure(
            AxCode::InvalidArgs,
            "read a screenshot request",
            "`refs` is not a list of references",
        )
        .with_recovery("pass `refs` as a list, the way `measure` takes it")
    };
    match args.as_map().get("refs") {
        None | Some(Value::Null) => Ok(None),
        Some(Value::Array(items)) if items.is_empty() => Err(AxError::failure(
            AxCode::InvalidArgs,
            "photograph a page",
            "`refs` names nothing to cover",
        )
        .with_recovery("name at least one reference the snapshot minted")),
        Some(Value::Array(items)) => {
            let mut references = Vec::new();
            for item in items {
                match item.as_str() {
                    Some(reference) if !reference.is_empty() => {
                        references.push(reference.to_owned());
                    }
                    Some(_) | None => return Err(not_a_list()),
                }
            }
            Ok(Some(references))
        }
        Some(_) => Err(not_a_list()),
    }
}

/// The snapshot generation a reference was read against.
///
/// # Errors
/// Refuses a missing or malformed generation: a reference without one is
/// a position in no snapshot, and `act` refuses it the same way.
pub(crate) fn generation_of(args: &Payload) -> Result<u64, AxError> {
    let bad = || {
        AxError::failure(
            AxCode::InvalidArgs,
            "photograph a page",
            "a `ref` or `refs` names a position in one snapshot, and its generation came with none",
        )
        .with_recovery("pass the `generation` the snapshot answered with, the way `act` does")
    };
    match args.as_map().get("generation") {
        Some(Value::Number(number)) => number.as_u64().ok_or_else(bad),
        None | Some(Value::Null) | Some(_) => Err(bad()),
    }
}

/// The one region a screenshot covers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Clip {
    Rect(Rect),
    Element(Element),
    Union(Union),
}

impl Rect {
    /// The smallest rectangle covering both.
    ///
    /// # Errors
    /// Refuses a rectangle whose sides cannot be added, which is a page
    /// no longer describing itself in pixels.
    pub(crate) fn covering(self, other: Rect) -> Result<Rect, AxError> {
        let unreadable = || {
            AxError::failure(
                AxCode::WireMismatch,
                "measure a page",
                "two boxes cannot be covered by one rectangle",
            )
            .with_recovery("check the driver's protocol version, then this crate's box script")
        };
        let x = self.x.min(other.x);
        let y = self.y.min(other.y);
        let right = self
            .x
            .checked_add(self.width)
            .ok_or_else(unreadable)?
            .max(other.x.checked_add(other.width).ok_or_else(unreadable)?);
        let bottom = self
            .y
            .checked_add(self.height)
            .ok_or_else(unreadable)?
            .max(other.y.checked_add(other.height).ok_or_else(unreadable)?);
        Ok(Rect {
            x,
            y,
            width: right.checked_sub(x).ok_or_else(unreadable)?,
            height: bottom.checked_sub(y).ok_or_else(unreadable)?,
        })
    }
}

impl Clip {
    /// The region a call named, refusing a call that named more than one.
    ///
    /// # Errors
    /// Refuses any pair of a rectangle, a reference and a set of
    /// references: a shot covers one region, and a caller that named more
    /// than one has not said which.
    pub fn of(
        rect: Option<Rect>,
        element: Option<Element>,
        union: Option<Union>,
    ) -> Result<Option<Clip>, AxError> {
        let named = usize::from(rect.is_some())
            .saturating_add(usize::from(element.is_some()))
            .saturating_add(usize::from(union.is_some()));
        if named > 1 {
            return Err(AxError::failure(
                AxCode::InvalidArgs,
                "photograph a page",
                "more than one of `clip`, `ref` and `refs` names the region to cover",
            )
            .with_recovery(
                "pass one: a rectangle in CSS pixels, a reference from the snapshot, or a set \
                 of references to cover together",
            ));
        }
        Ok(match (rect, element, union) {
            (Some(rect), None, None) => Some(Clip::Rect(rect)),
            (None, Some(element), None) => Some(Clip::Element(element)),
            (None, None, Some(union)) => Some(Clip::Union(union)),
            (None, None, None) => None,
            _ => return Err(Clip::more_than_one()),
        })
    }

    fn more_than_one() -> AxError {
        AxError::failure(
            AxCode::InvalidArgs,
            "photograph a page",
            "more than one of `clip`, `ref` and `refs` names the region to cover",
        )
        .with_recovery(
            "pass one: a rectangle in CSS pixels, a reference from the snapshot, or a set of \
             references to cover together",
        )
    }

    /// Whether the page has to answer before this clip can be captured:
    /// an element the page names, or a set of boxes it reports.
    #[must_use]
    pub fn waits_for_page(&self) -> bool {
        matches!(self, Clip::Element(_) | Clip::Union(_))
    }

    /// The clip as the capture states it.
    ///
    /// `element` is the handle the page minted for this clip's element,
    /// read out of the resolve frame's reply. A rectangle states itself
    /// and is handed `None`.
    ///
    /// # Errors
    /// Refuses an element without a handle, and a rectangle with one:
    /// neither is a request the protocol can carry.
    pub(crate) fn wire(&self, element: Option<&str>) -> Result<Value, AxError> {
        match (self, element) {
            (Clip::Rect(rect), None) => Ok(json!({
                "type": "box",
                "x": rect.x,
                "y": rect.y,
                "width": rect.width,
                "height": rect.height,
            })),
            (Clip::Element(_), Some(shared_id)) => Ok(json!({
                "type": "element",
                "element": { "sharedId": shared_id },
            })),
            (Clip::Rect(_), Some(_)) => Err(mismatch(
                "a rectangle states its own four sides and resolves nothing",
            )),
            (Clip::Element(_), None) => Err(mismatch(
                "an element clip needs the handle the page minted for it",
            )),
            (Clip::Union(_), _) => Err(mismatch(
                "a union is measured before it is captured, and captured as a rectangle",
            )),
        }
    }
}

/// A clip the caller and the resolve frame disagree about.
///
/// The two arms are reached by two different calls, so this is a defect
/// in the caller rather than a page's answer: the recovery says so.
fn mismatch(what: &str) -> AxError {
    AxError::failure(AxCode::InvalidArgs, "photograph a page", what).with_recovery(
        "report this against browser::shot::clip: an element clip is built from the resolve \
         frame's reply and a rectangle never is",
    )
}
