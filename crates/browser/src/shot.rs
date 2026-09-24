// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! A screenshot: what was asked for, and what came back.
//!
//! Every number a caller gives is an integer, including the two the
//! protocol spells as fractions. Quality and scale arrive as percentages
//! and become JSON numbers by formatting digits, so no floating-point
//! value exists in this crate at all — which is what lets a screenshot
//! request sit in a ledger payload beside everything else.

mod cap;
mod clip;
mod result;

use kernel::{AxCode, AxError, ImageType, Payload};
use serde_json::{Map, Value, json};

pub use cap::{SHOT_MAX_EDGE_PX, ShotMaxEdge};
pub use clip::{Clip, Element, Rect};
pub use result::Shot;

use crate::port::Frame;
use crate::session::{ContextId, Session};
use crate::snapshot::PageSnapshot;

/// What one screenshot was asked for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShotRequest {
    clip: Option<Clip>,
    format: ImageType,
    /// 0..=100. Read by the two lossy formats and ignored by PNG, which
    /// is what the protocol does with it too.
    quality: Option<u8>,
    /// A percentage of the device pixel ratio; 100 leaves it alone.
    scale: Option<u32>,
}

impl Default for ShotRequest {
    fn default() -> ShotRequest {
        ShotRequest {
            clip: None,
            format: ImageType::Png,
            quality: None,
            scale: None,
        }
    }
}

fn integer(args: &Payload, field: &str) -> Result<Option<u32>, AxError> {
    match args.as_map().get(field) {
        None | Some(Value::Null) => Ok(None),
        Some(value) => {
            let raw = value.as_u64().ok_or_else(|| {
                AxError::failure(
                    AxCode::InvalidArgs,
                    "read a screenshot request",
                    format!("`{field}` is not a whole number"),
                )
                .with_recovery("pass whole numbers; this city keeps floats out of payloads")
            })?;
            let narrowed = u32::try_from(raw).map_err(|_| {
                AxError::failure(
                    AxCode::InvalidArgs,
                    "read a screenshot request",
                    format!("`{field}` is larger than a page can be"),
                )
                .with_recovery("pass a pixel count that fits in 32 bits")
            })?;
            Ok(Some(narrowed))
        }
    }
}

fn required(field: &str) -> AxError {
    AxError::failure(
        AxCode::InvalidArgs,
        "read a screenshot request",
        format!("a clip needs `{field}`"),
    )
    .with_recovery("a clip is x, y, width and height together")
}

/// The reference a call names, when it names one.
///
/// # Errors
/// Refuses a `ref` that is present and is not a reference: silently
/// ignoring it would photograph the whole page while the caller believed
/// it had asked for one element.
fn reference_of(args: &Payload) -> Result<Option<String>, AxError> {
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

/// The snapshot generation a reference was read against.
///
/// # Errors
/// Refuses a missing or malformed generation: a reference without one is
/// a position in no snapshot, and `act` refuses it the same way.
fn generation_of(args: &Payload) -> Result<u64, AxError> {
    let bad = || {
        AxError::failure(
            AxCode::InvalidArgs,
            "photograph a page",
            "a `ref` names a position in one snapshot, and its generation came with none",
        )
        .with_recovery("pass the `generation` the snapshot answered with, the way `act` does")
    };
    match args.as_map().get("generation") {
        Some(Value::Number(number)) => number.as_u64().ok_or_else(bad),
        None | Some(Value::Null) | Some(_) => Err(bad()),
    }
}

/// A JSON number built out of digits rather than out of a float.
///
/// # Errors
/// Refuses a percentage this build cannot spell, which the two callers
/// below cannot produce.
fn ratio(percent: u32) -> Result<Value, AxError> {
    let whole = percent
        .checked_div(100)
        .ok_or_else(|| unreadable(percent))?;
    let rest = percent
        .checked_rem(100)
        .ok_or_else(|| unreadable(percent))?;
    serde_json::from_str(&format!("{whole}.{rest:02}")).map_err(|_| unreadable(percent))
}

fn unreadable(percent: u32) -> AxError {
    AxError::failure(
        AxCode::InvalidArgs,
        "read a screenshot request",
        format!("{percent} is not a percentage this build can spell"),
    )
    .with_recovery("pass a percentage between 0 and 1000")
}

impl ShotRequest {
    /// Reads the four optional fields of a `screenshot` call.
    ///
    /// # Errors
    /// Refuses a clip missing one of its four sides, a format this city
    /// does not carry, and any number that is not whole.
    pub fn read(args: &Payload) -> Result<ShotRequest, AxError> {
        let rect = match args.as_map().get("clip") {
            None | Some(Value::Null) => None,
            Some(value) => {
                let inner = Payload::new(value.as_object().ok_or_else(|| required("x"))?.clone())?;
                Some(Rect {
                    x: integer(&inner, "x")?.ok_or_else(|| required("x"))?,
                    y: integer(&inner, "y")?.ok_or_else(|| required("y"))?,
                    width: integer(&inner, "width")?.ok_or_else(|| required("width"))?,
                    height: integer(&inner, "height")?.ok_or_else(|| required("height"))?,
                })
            }
        };
        let element = match reference_of(args)? {
            None => None,
            Some(reference) => Some(Element::new(reference, generation_of(args)?)),
        };
        let clip = Clip::of(rect, element)?;
        let format = match args.as_map().get("format").and_then(Value::as_str) {
            None | Some("png") => ImageType::Png,
            Some("jpeg") => ImageType::Jpeg,
            Some("webp") => ImageType::Webp,
            Some(other) => {
                return Err(AxError::failure(
                    AxCode::InvalidArgs,
                    "read a screenshot request",
                    other.to_owned(),
                )
                .with_recovery("one of png, jpeg, webp"));
            }
        };
        let quality = match integer(args, "quality")? {
            None => None,
            Some(percent) => Some(u8::try_from(percent.min(100)).map_err(|_| {
                AxError::failure(
                    AxCode::InvalidArgs,
                    "read a screenshot request",
                    "quality is a percentage",
                )
                .with_recovery("pass 0 to 100")
            })?),
        };
        Ok(ShotRequest {
            clip,
            format,
            quality,
            scale: integer(args, "scale")?,
        })
    }

    #[must_use]
    pub fn format(&self) -> ImageType {
        self.format
    }

    /// Whether this shot has to wait for the page to name an element.
    #[must_use]
    pub fn resolves_element(&self) -> bool {
        self.clip.as_ref().is_some_and(Clip::resolves_element)
    }

    /// The frames one screenshot needs.
    ///
    /// A scale other than 100 changes the device pixel ratio first,
    /// because the protocol has no per-capture scale and a caller who
    /// asked for one wants more pixels rather than a bigger file. A shot
    /// that names an element ends here: the page has to name the element
    /// before the capture can cover it, so this is the resolve frame and
    /// [`ShotRequest::capture_frame`] is the one after it — the two-phase
    /// shape `Verb::Act` already uses for a drag from a reference.
    ///
    /// # Errors
    /// Refuses a shot by reference with no snapshot to resolve it in, and
    /// propagates the frames' own refusals.
    pub(crate) fn frames(
        &self,
        session: &mut Session,
        context: &ContextId,
        snapshot: Option<&PageSnapshot>,
    ) -> Result<Vec<Frame>, AxError> {
        let mut frames = self.density_frames(session, context)?;
        match &self.clip {
            Some(Clip::Element(element)) => {
                let looked = crate::verb::looked_at(snapshot, "photograph a page")?;
                crate::act::ensure_fresh(looked, element.generation())?;
                frames.push(crate::act::resolve_frame(
                    session,
                    context,
                    looked,
                    element.reference(),
                )?);
            }
            Some(Clip::Rect(_)) | None => frames.push(self.capture(session, context, None)?),
        }
        Ok(frames)
    }

    /// The capture frame, once the page has named the element this shot
    /// covers.
    ///
    /// `reply` is the resolve frame's own reply: the handle the page
    /// minted is read out of it here rather than handed in, so no caller
    /// can crop to an element the page never named.
    ///
    /// # Errors
    /// Refuses a shot that covers a rectangle, which has nothing to
    /// resolve, and propagates a reply that carries no handle.
    pub fn capture_frame(
        &self,
        session: &mut Session,
        context: &ContextId,
        reply: &Value,
    ) -> Result<Frame, AxError> {
        match &self.clip {
            Some(Clip::Element(_)) => {
                let shared_id = crate::input::shared_id_of(reply)?;
                self.capture(session, context, Some(&shared_id))
            }
            Some(Clip::Rect(_)) | None => Err(AxError::failure(
                AxCode::InvalidArgs,
                "photograph a page",
                "this shot covers a rectangle, so nothing was resolved",
            )
            .with_recovery(
                "report this against browser::shot: a capture frame follows a resolve frame \
                 only when the shot names an element",
            )),
        }
    }

    /// The frame that captures this region again, small enough to fit.
    ///
    /// A capture that came back with a side past the cap is asked for
    /// once more at the density that brings that side down to the cap,
    /// composed with the density the caller asked for. It is the only
    /// lever the protocol gives: `imageSize` is stated and ignored, and
    /// the cost — a page that queries `resolution` may lay out
    /// differently at the new ratio — is paid only where the alternative
    /// is a refusal of the shot.
    ///
    /// # Errors
    /// Refuses a side this build cannot state a density for.
    pub fn refit_frame(
        &self,
        session: &mut Session,
        context: &ContextId,
        longest: u32,
    ) -> Result<Frame, AxError> {
        let ratio = SHOT_MAX_EDGE_PX.fit(longest, self.scale.unwrap_or(100))?;
        session.frame(
            "browsingContext.setViewport",
            json!({ "context": context.as_str(), "devicePixelRatio": ratio }),
        )
    }

    /// The device-ratio change a scale asks for, which comes before
    /// anything is captured.
    fn density_frames(
        &self,
        session: &mut Session,
        context: &ContextId,
    ) -> Result<Vec<Frame>, AxError> {
        let mut frames = Vec::new();
        if let Some(scale) = self.scale.filter(|percent| *percent != 100) {
            frames.push(session.frame(
                "browsingContext.setViewport",
                json!({ "context": context.as_str(), "devicePixelRatio": ratio(scale)? }),
            )?);
        }
        Ok(frames)
    }

    /// One capture frame. `element` is the page's handle for the element
    /// this shot covers, and is `Some` only on that one arm.
    fn capture(
        &self,
        session: &mut Session,
        context: &ContextId,
        element: Option<&str>,
    ) -> Result<Frame, AxError> {
        let clip = match (&self.clip, element) {
            (Some(clip), handle) => Some(clip.wire(handle)?),
            (None, None) => None,
            (None, Some(_)) => {
                return Err(AxError::failure(
                    AxCode::InvalidArgs,
                    "photograph a page",
                    "a handle arrived for a shot that covers no element",
                )
                .with_recovery(
                    "report this against browser::shot: a handle is minted only for a shot by \
                     reference",
                ));
            }
        };
        let mut params = Map::new();
        params.insert(
            "context".to_owned(),
            Value::String(context.as_str().to_owned()),
        );
        let mut format = Map::new();
        format.insert(
            "type".to_owned(),
            Value::String(self.format.mime().to_owned()),
        );
        if let Some(quality) = self.quality {
            format.insert("quality".to_owned(), ratio(u32::from(quality))?);
        }
        params.insert("format".to_owned(), Value::Object(format));
        params.insert("imageSize".to_owned(), SHOT_MAX_EDGE_PX.image_size());
        if let Some(clip) = clip {
            params.insert("clip".to_owned(), clip);
        }
        session.frame("browsingContext.captureScreenshot", Value::Object(params))
    }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "test code"
)]
mod tests;
