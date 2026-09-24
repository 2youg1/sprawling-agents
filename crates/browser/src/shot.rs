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
pub use clip::{Clip, Element, Rect, Union};
use clip::{generation_of, reference_of, references_of};
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
        let union = match references_of(args)? {
            None => None,
            Some(references) => Some(Union::new(references, generation_of(args)?)),
        };
        let clip = Clip::of(rect, element, union)?;
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

    /// Whether the page has to answer before this shot can be captured:
    /// an element it names, or a set of boxes it reports.
    #[must_use]
    pub fn waits_for_page(&self) -> bool {
        self.clip.as_ref().is_some_and(Clip::waits_for_page)
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
            Some(Clip::Union(union)) => {
                let looked = crate::verb::looked_at(snapshot, "photograph a page")?;
                crate::act::ensure_fresh(looked, union.generation())?;
                // The boxes come from the script `measure` sends, so the
                // region a shot covers and the region a measurement
                // reports are read once and cannot disagree.
                frames.push(session.evaluate(
                    context,
                    &crate::verb::script::measure_script(looked, union.references())?,
                )?);
            }
            Some(Clip::Rect(_)) | None => {
                let clip = match &self.clip {
                    Some(clip) => Some(clip.wire(None)?),
                    None => None,
                };
                frames.push(self.capture(session, context, clip)?);
            }
        }
        Ok(frames)
    }

    /// The capture frame, once the page has answered about the region.
    ///
    /// `reply` is the frame this shot sent first: the resolve frame's
    /// reply for an element, the measure frame's for a set of boxes. The
    /// handle or the boxes are read out of it here rather than handed in,
    /// so no caller can crop to an element or a region the page never
    /// reported.
    ///
    /// # Errors
    /// Refuses a shot that covers a rectangle, which had nothing to
    /// resolve, and propagates a reply this build cannot read.
    pub fn capture_frame(
        &self,
        session: &mut Session,
        context: &ContextId,
        reply: &Value,
    ) -> Result<Frame, AxError> {
        match &self.clip {
            Some(clip @ Clip::Element(_)) => {
                let shared_id = crate::input::shared_id_of(reply)?;
                self.capture(session, context, Some(clip.wire(Some(&shared_id))?))
            }
            Some(Clip::Union(_)) => {
                let rect = Union::around(reply)?;
                self.capture(session, context, Some(Clip::Rect(rect).wire(None)?))
            }
            Some(Clip::Rect(_)) | None => Err(AxError::failure(
                AxCode::InvalidArgs,
                "photograph a page",
                "this shot covers a rectangle, so nothing was resolved",
            )
            .with_recovery(
                "report this against browser::shot: a capture frame follows a page's answer \
                 only when the shot names an element or a set of references",
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

    /// One capture frame, with the clip the caller has already resolved.
    fn capture(
        &self,
        session: &mut Session,
        context: &ContextId,
        clip: Option<Value>,
    ) -> Result<Frame, AxError> {
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
