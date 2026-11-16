// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Rectangles, the two coordinate spaces, and what a percentage scales a
//! size to.
//!
//! Two spaces meet in this server and are easy to confuse, so they are
//! two types rather than two conventions: a [`Point`] is on the virtual
//! screen, and a [`Inset`] is inside one window. Everything a caller
//! sends is an inset, because a caller that could name a screen point
//! could reach a window the scope file left out; everything Win32 wants
//! is a point. [`Bounds::at`] is the one crossing.
//!
//! Every arithmetic operation here is checked. A window can sit at a
//! negative coordinate on a multi-monitor desktop, and a caller can send
//! any integer its JSON can hold, so overflow is reachable input rather
//! than a hypothetical.

use crate::refusal::{Refusal, RefusalCode};
use serde_json::{Value, json};

/// A point on the virtual screen, which is where Win32 wants one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Point {
    pub(crate) x: i32,
    pub(crate) y: i32,
}

/// A point inside one window, which is the only kind a caller may send.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Inset {
    pub(crate) x: i32,
    pub(crate) y: i32,
}

/// One rectangle on the virtual screen, held as its origin and its size
/// so that a negative width cannot be spelled.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Bounds {
    left: i32,
    top: i32,
    width: u32,
    height: u32,
}

impl Bounds {
    /// Reads the two corners Win32 reports.
    ///
    /// # Errors
    /// Refuses a rectangle whose right edge is left of its left edge, or
    /// whose corners are far enough apart to overflow a signed
    /// subtraction. Both mean the window moved or closed between the
    /// call that named it and the call that measured it, which is a
    /// refusal rather than a size to guess at.
    pub(crate) fn from_corners(
        left: i32,
        top: i32,
        right: i32,
        bottom: i32,
    ) -> Result<Bounds, Refusal> {
        let width = right.checked_sub(left).filter(|span| *span > 0);
        let height = bottom.checked_sub(top).filter(|span| *span > 0);
        let (Some(width), Some(height)) = (width, height) else {
            return Err(unusable("the window has no visible area"));
        };
        Ok(Bounds {
            left,
            top,
            width: u32::try_from(width).map_err(|_| unusable("the window is impossibly wide"))?,
            height: u32::try_from(height).map_err(|_| unusable("the window is impossibly tall"))?,
        })
    }

    pub(crate) fn width(self) -> u32 {
        self.width
    }

    pub(crate) fn height(self) -> u32 {
        self.height
    }

    /// Where an inset lands on the virtual screen.
    ///
    /// # Errors
    /// Refuses an inset that leaves this rectangle, so a point argument
    /// cannot reach a window the scope file did not list, and refuses
    /// one whose addition overflows.
    pub(crate) fn at(self, inset: Inset) -> Result<Point, Refusal> {
        let inside = i32::try_from(self.width).is_ok_and(|span| inset.x >= 0 && inset.x < span)
            && i32::try_from(self.height).is_ok_and(|span| inset.y >= 0 && inset.y < span);
        if !inside {
            return Err(Refusal::new(
                RefusalCode::InvalidArgs,
                "use the desktop",
                format!(
                    "the point ({}, {}) is outside a window that is {}x{}",
                    inset.x, inset.y, self.width, self.height
                ),
                "send a point inside the window, or a `ref` from `desktop.snapshot`; a point \
                 outside the window would land on whatever else is there",
            ));
        }
        let (Some(x), Some(y)) = (
            self.left.checked_add(inset.x),
            self.top.checked_add(inset.y),
        ) else {
            return Err(unusable("the point does not fit on this desktop"));
        };
        Ok(Point { x, y })
    }

    /// The middle of this rectangle, which is where a ref is acted on.
    ///
    /// # Errors
    /// Refuses a rectangle whose middle overflows.
    pub(crate) fn centre(self) -> Result<Point, Refusal> {
        let half = |span: u32| i32::try_from(span.wrapping_div(2)).ok();
        let (Some(dx), Some(dy)) = (half(self.width), half(self.height)) else {
            return Err(unusable("the element is impossibly large"));
        };
        let (Some(x), Some(y)) = (self.left.checked_add(dx), self.top.checked_add(dy)) else {
            return Err(unusable("the element does not fit on this desktop"));
        };
        Ok(Point { x, y })
    }

    /// The four numbers a caller reads.
    pub(crate) fn as_json(self) -> Value {
        json!({ "x": self.left, "y": self.top, "width": self.width, "height": self.height })
    }
}

/// What `scale` percent of a size is, in whole pixels.
///
/// # Errors
/// Refuses a percentage that would scale this image away entirely. An
/// empty image is not a smaller image, and answering with one would send
/// a model onward from a picture of nothing.
pub(crate) fn scaled(width: u32, height: u32, percent: u32) -> Result<(u32, u32), Refusal> {
    let apply = |span: u32| {
        span.checked_mul(percent)
            .map(|product| product.wrapping_div(100))
            .filter(|scaled| *scaled > 0)
    };
    let (Some(width), Some(height)) = (apply(width), apply(height)) else {
        return Err(Refusal::new(
            RefusalCode::InvalidArgs,
            "capture a window",
            format!("scaling this window to {percent}% leaves it no pixels"),
            "raise `scale`, or leave it out and take the window at its own size",
        ));
    };
    Ok((width, height))
}

/// A measurement this server could not use. The recovery is one sentence
/// because there is one thing to do about every one of them.
fn unusable(because: &str) -> Refusal {
    Refusal::new(
        RefusalCode::InvalidArgs,
        "use the desktop",
        because.to_owned(),
        "call `desktop.windows` again; the window may have moved or closed since it was named",
    )
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "test code"
)]
mod tests {
    use super::*;

    /// A window can sit at a negative coordinate on a second monitor,
    /// and its size is still a positive number of pixels.
    #[test]
    fn a_window_left_of_the_primary_monitor_still_has_a_positive_size() {
        let bounds = Bounds::from_corners(-1920, -100, -1120, 500).unwrap();
        assert_eq!(bounds.width(), 800);
        assert_eq!(bounds.height(), 600);
        assert_eq!(bounds.as_json()["x"], -1920);
        assert_eq!(bounds.centre().unwrap(), Point { x: -1520, y: 200 });
    }

    /// A window that closed between being named and being measured
    /// reports a collapsed rectangle. That is a refusal, not a zero-sized
    /// image.
    #[test]
    fn a_collapsed_rectangle_is_refused_rather_than_measured() {
        for (right, bottom) in [(0, 100), (100, 0), (-5, -5)] {
            let refusal = Bounds::from_corners(0, 0, right, bottom)
                .expect_err("a window with no area cannot be worked on");
            assert_eq!(refusal.as_error()["data"]["code"], "E_INVALID_ARGS");
        }
        // Corners far enough apart to overflow the subtraction are the
        // same answer, not a panic.
        assert!(Bounds::from_corners(i32::MIN, i32::MIN, i32::MAX, i32::MAX).is_err());
    }

    /// The rule that keeps a point argument from becoming a second way
    /// out of the scope file: it is inside the window or it is refused.
    #[test]
    fn a_point_outside_the_window_is_refused_rather_than_clamped() {
        let bounds = Bounds::from_corners(100, 200, 500, 600).unwrap();
        assert_eq!(
            bounds.at(Inset { x: 0, y: 0 }).unwrap(),
            Point { x: 100, y: 200 }
        );
        assert_eq!(
            bounds.at(Inset { x: 399, y: 399 }).unwrap(),
            Point { x: 499, y: 599 }
        );
        for outside in [
            Inset { x: 400, y: 0 },
            Inset { x: 0, y: 400 },
            Inset { x: -1, y: 0 },
            Inset {
                x: i32::MAX,
                y: i32::MAX,
            },
        ] {
            let refusal = bounds
                .at(outside)
                .expect_err("a point outside the window is refused");
            assert!(
                refusal.as_error()["data"]["recovery"]
                    .as_str()
                    .unwrap()
                    .contains("inside the window")
            );
        }
    }

    #[test]
    fn a_percentage_scales_both_sides_and_never_to_nothing() {
        assert_eq!(scaled(800, 600, 100).unwrap(), (800, 600));
        assert_eq!(scaled(800, 600, 50).unwrap(), (400, 300));
        assert_eq!(scaled(801, 601, 50).unwrap(), (400, 300));
        // 1% of a ten-pixel window is nothing, and nothing is not an
        // image.
        let refusal = scaled(10, 10, 1).expect_err("an empty image is not a smaller image");
        assert!(
            refusal.as_error()["data"]["recovery"]
                .as_str()
                .unwrap()
                .contains("scale")
        );
        // A width big enough to overflow the multiplication is the same
        // answer rather than a wrapped size.
        assert!(scaled(u32::MAX, 10, 100).is_err());
    }
}
