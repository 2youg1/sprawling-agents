// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use crate::refusal::{Refusal, RefusalCode};
use serde_json::{Value, json};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Point {
    pub(crate) x: i32,
    pub(crate) y: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Inset {
    pub(crate) x: i32,
    pub(crate) y: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Bounds {
    left: i32,
    top: i32,
    width: u32,
    height: u32,
}

impl Bounds {
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

    pub(crate) fn as_json(self) -> Value {
        json!({ "x": self.left, "y": self.top, "width": self.width, "height": self.height })
    }
}

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

    #[test]
    fn a_window_left_of_the_primary_monitor_still_has_a_positive_size() {
        let bounds = Bounds::from_corners(-1920, -100, -1120, 500).unwrap();
        assert_eq!(bounds.width(), 800);
        assert_eq!(bounds.height(), 600);
        assert_eq!(bounds.as_json()["x"], -1920);
        assert_eq!(bounds.centre().unwrap(), Point { x: -1520, y: 200 });
    }

    #[test]
    fn a_collapsed_rectangle_is_refused_rather_than_measured() {
        for (right, bottom) in [(0, 100), (100, 0), (-5, -5)] {
            let refusal = Bounds::from_corners(0, 0, right, bottom)
                .expect_err("a window with no area cannot be worked on");
            assert_eq!(refusal.as_error()["data"]["code"], "E_INVALID_ARGS");
        }

        assert!(Bounds::from_corners(i32::MIN, i32::MIN, i32::MAX, i32::MAX).is_err());
    }

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

        let refusal = scaled(10, 10, 1).expect_err("an empty image is not a smaller image");
        assert!(
            refusal.as_error()["data"]["recovery"]
                .as_str()
                .unwrap()
                .contains("scale")
        );

        assert!(scaled(u32::MAX, 10, 100).is_err());
    }
}
