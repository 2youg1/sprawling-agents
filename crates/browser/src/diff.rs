// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! What changed between two screenshots.
//!
//! The answer is two things a caller can act on: how much of the picture
//! moved, and where. The amount is a ten-thousandth rather than a
//! percentage with a decimal point, because it goes into a ledger
//! payload and this city keeps floats out of those. The places are
//! boxes of whole pixels, for the same reason.
//!
//! Two pictures of different sizes are not compared. Scaling one onto
//! the other would answer a question about the scaling algorithm rather
//! than about the page.

use kernel::{AxCode, AxError, ImageType};

use crate::shot::Shot;

/// How wide a tile is. Changed pixels are grouped by tile before they
/// are grouped into boxes, so a box is a region a person can look at
/// rather than a list of pixels nobody can read.
const TILE: u32 = 32;

/// A rectangle of whole pixels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Box2 {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

impl Box2 {
    /// Whether this box covers the pixel at `(x, y)`.
    #[must_use]
    pub fn covers(&self, x: u32, y: u32) -> bool {
        let right = self.x.saturating_add(self.width);
        let bottom = self.y.saturating_add(self.height);
        x >= self.x && x < right && y >= self.y && y < bottom
    }
}

/// What one comparison found.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Difference {
    changed_q4: u32,
    boxes: Vec<Box2>,
}

impl Difference {
    /// How much of the picture changed, in ten-thousandths: 10_000 is
    /// every pixel and 0 is none.
    #[must_use]
    pub fn changed_q4(&self) -> u32 {
        self.changed_q4
    }

    /// Where it changed, in reading order: top to bottom, then left to
    /// right. An empty list is what "nothing moved" looks like.
    #[must_use]
    pub fn boxes(&self) -> &[Box2] {
        &self.boxes
    }
}

/// Compares two screenshots.
///
/// # Errors
/// Refuses two pictures of different sizes, a format this build cannot
/// decode, and bytes that are not the picture they claim to be.
pub fn diff(before: &Shot, after: &Shot) -> Result<Difference, AxError> {
    if before.width() != after.width() || before.height() != after.height() {
        return Err(AxError::failure(
            AxCode::InvalidArgs,
            "compare two screenshots",
            format!(
                "{}x{} against {}x{}",
                before.width(),
                before.height(),
                after.width(),
                after.height()
            ),
        )
        .with_recovery(
            "take both at one viewport; scaling one onto the other would measure the scaler",
        ));
    }
    let first = pixels(before)?;
    let second = pixels(after)?;
    if first.stride != second.stride || first.bytes.len() != second.bytes.len() {
        return Err(AxError::failure(
            AxCode::InvalidArgs,
            "compare two screenshots",
            "the two pictures are stored differently",
        )
        .with_recovery("ask for both in the same format"));
    }
    let mut grid = Grid::over(before.width(), before.height());
    let mut changed = 0u64;
    for y in 0..grid.height {
        for x in 0..grid.width {
            if same_pixel(&first, &second, x, y)? {
                continue;
            }
            changed = changed.saturating_add(1);
            grid.mark(x, y);
        }
    }
    let area = u64::from(grid.width).saturating_mul(u64::from(grid.height));
    Ok(Difference {
        changed_q4: share(changed, area),
        boxes: grid.boxes(),
    })
}

/// The picture divided into tiles, with the tiles something changed in
/// marked. The four numbers describing it always travel together.
struct Grid {
    width: u32,
    height: u32,
    across: u32,
    down: u32,
    hot: Vec<bool>,
}

impl Grid {
    fn over(width: u32, height: u32) -> Grid {
        let across = tiles(width);
        let down = tiles(height);
        Grid {
            width,
            height,
            across,
            down,
            hot: vec![false; usize::try_from(across.saturating_mul(down)).unwrap_or(0)],
        }
    }

    fn mark(&mut self, x: u32, y: u32) {
        let at = tile_index(x, y, self.across);
        if let Some(cell) = self.hot.get_mut(at) {
            *cell = true;
        }
    }

    fn is_hot(&self, x: u32, y: u32) -> bool {
        let at = usize::try_from(y.saturating_mul(self.across).saturating_add(x)).unwrap_or(0);
        self.hot.get(at).copied().unwrap_or(false)
    }
}

/// The changed share, in ten-thousandths, rounded down.
fn share(changed: u64, area: u64) -> u32 {
    if area == 0 {
        return 0;
    }
    let scaled = changed.saturating_mul(10_000);
    let value = scaled.checked_div(area).unwrap_or(0);
    u32::try_from(value).unwrap_or(10_000)
}

fn tiles(side: u32) -> u32 {
    side.saturating_add(TILE.saturating_sub(1))
        .checked_div(TILE)
        .unwrap_or(0)
}

fn tile_index(x: u32, y: u32, across: u32) -> usize {
    let column = x.checked_div(TILE).unwrap_or(0);
    let row = y.checked_div(TILE).unwrap_or(0);
    usize::try_from(row.saturating_mul(across).saturating_add(column)).unwrap_or(0)
}

/// One decoded picture: the bytes, and how long one row of them is.
///
/// The two always travel together, because neither says anything on its
/// own - a byte offset needs the stride to mean a pixel.
struct Decoded {
    bytes: Vec<u8>,
    stride: usize,
}

impl Decoded {
    /// The four bytes of one pixel, or nothing when the decoded picture
    /// is smaller than its own header said.
    fn pixel(&self, x: u32, y: u32) -> Option<&[u8]> {
        let row = usize::try_from(y).ok()?.checked_mul(self.stride)?;
        let at = row.checked_add(usize::try_from(x).ok()?.checked_mul(BYTES_PER_PIXEL)?)?;
        self.bytes.get(at..at.checked_add(BYTES_PER_PIXEL)?)
    }
}

/// Whether one pixel is the same in both pictures.
///
/// # Errors
/// Reports bytes shorter than the size they declared, which is a decoded
/// picture that lied about itself rather than a difference.
fn same_pixel(first: &Decoded, second: &Decoded, x: u32, y: u32) -> Result<bool, AxError> {
    match (first.pixel(x, y), second.pixel(x, y)) {
        (Some(left), Some(right)) => Ok(left == right),
        _ => Err(AxError::failure(
            AxCode::WireMismatch,
            "compare two screenshots",
            "the decoded picture is smaller than its own header",
        )
        .with_recovery("take the screenshot again")),
    }
}

/// Every picture is decoded to eight-bit RGBA, so one number describes
/// both of them however they were stored.
const BYTES_PER_PIXEL: usize = 4;

/// Decodes a screenshot to eight-bit RGBA.
///
/// # Errors
/// Refuses a format this build cannot decode, and bytes that are not the
/// picture they claim to be.
fn pixels(shot: &Shot) -> Result<Decoded, AxError> {
    if shot.media() != ImageType::Png {
        return Err(AxError::failure(
            AxCode::InvalidArgs,
            "compare two screenshots",
            format!("{} is not decoded by this build", shot.media().mime()),
        )
        .with_recovery("ask for png, which is what a screenshot is asked for by default"));
    }
    let mut decoder = png::Decoder::new(std::io::Cursor::new(shot.bytes()));
    decoder.set_transformations(
        png::Transformations::normalize_to_color8() | png::Transformations::ALPHA,
    );
    let mut reader = decoder.read_info().map_err(decode_failed)?;
    let mut buffer = vec![0u8; reader.output_buffer_size().unwrap_or(0)];
    let info = reader.next_frame(&mut buffer).map_err(decode_failed)?;
    buffer.truncate(info.buffer_size());
    let stride = usize::try_from(info.width)
        .unwrap_or(0)
        .saturating_mul(BYTES_PER_PIXEL);
    if info.line_size != stride {
        return Err(AxError::failure(
            AxCode::InvalidArgs,
            "compare two screenshots",
            "this picture did not decode to eight-bit rgba",
        )
        .with_recovery("ask for png, which is what a screenshot is asked for by default"));
    }
    Ok(Decoded {
        bytes: buffer,
        stride,
    })
}

fn decode_failed(err: png::DecodingError) -> AxError {
    AxError::failure(
        AxCode::WireMismatch,
        "compare two screenshots",
        err.to_string(),
    )
    .with_recovery("the bytes said png and are not png")
}

impl Grid {
    /// Groups changed tiles into boxes, one box per connected group.
    ///
    /// Connectivity is up, down, left and right. Diagonal neighbours are
    /// two groups, which reads as two places on a page rather than one
    /// box covering the gap between them.
    fn boxes(&self) -> Vec<Box2> {
        let mut seen = vec![false; self.hot.len()];
        let mut boxes = Vec::new();
        for row in 0..self.down {
            for column in 0..self.across {
                if let Some(found) = self.group_from(column, row, &mut seen) {
                    boxes.push(found);
                }
            }
        }
        boxes.sort_unstable();
        boxes
    }

    /// The box covering the group this tile belongs to, or nothing when
    /// this tile did not change or was already counted.
    fn group_from(&self, column: u32, row: u32, seen: &mut [bool]) -> Option<Box2> {
        let start = usize::try_from(row.saturating_mul(self.across).saturating_add(column)).ok()?;
        if !self.is_hot(column, row) || seen.get(start).copied().unwrap_or(true) {
            return None;
        }
        let mut stack = vec![(column, row)];
        let (mut left, mut top, mut right, mut bottom) = (column, row, column, row);
        while let Some((x, y)) = stack.pop() {
            let at = usize::try_from(y.saturating_mul(self.across).saturating_add(x)).unwrap_or(0);
            if !self.is_hot(x, y) || seen.get(at).copied().unwrap_or(true) {
                continue;
            }
            if let Some(cell) = seen.get_mut(at) {
                *cell = true;
            }
            left = left.min(x);
            top = top.min(y);
            right = right.max(x);
            bottom = bottom.max(y);
            if x > 0 {
                stack.push((x.saturating_sub(1), y));
            }
            if y > 0 {
                stack.push((x, y.saturating_sub(1)));
            }
            if x.saturating_add(1) < self.across {
                stack.push((x.saturating_add(1), y));
            }
            if y.saturating_add(1) < self.down {
                stack.push((x, y.saturating_add(1)));
            }
        }
        Some(self.box_of(Tiles {
            left,
            top,
            right,
            bottom,
        }))
    }

    /// The pixel box a run of tiles covers, cut to the picture's edge.
    fn box_of(&self, tiles: Tiles) -> Box2 {
        let x = tiles.left.saturating_mul(TILE);
        let y = tiles.top.saturating_mul(TILE);
        let far = tiles
            .right
            .saturating_add(1)
            .saturating_mul(TILE)
            .min(self.width);
        let low = tiles
            .bottom
            .saturating_add(1)
            .saturating_mul(TILE)
            .min(self.height);
        Box2 {
            x,
            y,
            width: far.saturating_sub(x),
            height: low.saturating_sub(y),
        }
    }
}

/// The four tile coordinates of one group, which are one value because
/// three of them alone say nothing.
struct Tiles {
    left: u32,
    top: u32,
    right: u32,
    bottom: u32,
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    reason = "test code"
)]
mod tests;
