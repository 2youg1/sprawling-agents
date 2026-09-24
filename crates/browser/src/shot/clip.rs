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

use kernel::{AxCode, AxError};
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

/// The one region a screenshot covers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Clip {
    Rect(Rect),
    Element(Element),
}

impl Clip {
    /// The region a call named, refusing a call that named two.
    ///
    /// # Errors
    /// Refuses a rectangle and a reference together: a shot covers one
    /// region, and a caller that named both has not said which.
    pub fn of(rect: Option<Rect>, element: Option<Element>) -> Result<Option<Clip>, AxError> {
        match (rect, element) {
            (Some(_), Some(_)) => Err(AxError::failure(
                AxCode::InvalidArgs,
                "photograph a page",
                "both `clip` and `ref` name the region to cover",
            )
            .with_recovery(
                "pass one of the two: a rectangle in CSS pixels, or a reference from the snapshot",
            )),
            (Some(rect), None) => Ok(Some(Clip::Rect(rect))),
            (None, Some(element)) => Ok(Some(Clip::Element(element))),
            (None, None) => Ok(None),
        }
    }

    /// Whether the page has to name this clip's element before the
    /// capture can be asked for.
    #[must_use]
    pub fn resolves_element(&self) -> bool {
        matches!(self, Clip::Element(_))
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
