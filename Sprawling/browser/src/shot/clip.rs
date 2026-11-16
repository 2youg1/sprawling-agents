// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use kernel::{AxCode, AxError, Payload};
use serde_json::{Value, json};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Rect {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

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

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Clip {
    Rect(Rect),
    Element(Element),
    Union(Union),
}

impl Rect {

    #[must_use]
    pub fn covers(&self, x: u32, y: u32) -> bool {
        let right = self.x.saturating_add(self.width);
        let bottom = self.y.saturating_add(self.height);
        x >= self.x && x < right && y >= self.y && y < bottom
    }

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

    #[must_use]
    pub fn waits_for_page(&self) -> bool {
        matches!(self, Clip::Element(_) | Clip::Union(_))
    }

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

fn mismatch(what: &str) -> AxError {
    AxError::failure(AxCode::InvalidArgs, "photograph a page", what).with_recovery(
        "report this against browser::shot::clip: an element clip is built from the resolve \
         frame's reply and a rectangle never is",
    )
}
