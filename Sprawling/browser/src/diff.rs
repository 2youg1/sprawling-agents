// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use kernel::{AxCode, AxError, ImageType};

use crate::shot::{Rect, Shot};

const TILE: u32 = 32;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Difference {
    changed_q4: u32,
    boxes: Vec<Rect>,
}

impl Difference {
    #[must_use]
    pub fn changed_q4(&self) -> u32 {
        self.changed_q4
    }

    #[must_use]
    pub fn boxes(&self) -> &[Rect] {
        &self.boxes
    }
}

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

struct Decoded {
    bytes: Vec<u8>,
    stride: usize,
}

impl Decoded {
    fn pixel(&self, x: u32, y: u32) -> Option<&[u8]> {
        let row = usize::try_from(y).ok()?.checked_mul(self.stride)?;
        let at = row.checked_add(usize::try_from(x).ok()?.checked_mul(BYTES_PER_PIXEL)?)?;
        self.bytes.get(at..at.checked_add(BYTES_PER_PIXEL)?)
    }
}

fn same_pixel(first: &Decoded, second: &Decoded, x: u32, y: u32) -> Result<bool, AxError> {
    match (first.pixel(x, y), second.pixel(x, y)) {
        (Some(left), Some(right)) => Ok(left == right),

        (None, Some(_)) => Err(ended_early(Short::Before)),
        (Some(_), None) => Err(ended_early(Short::After)),

        (None, None) => Err(ended_early(Short::Both)),
    }
}

enum Short {
    Before,
    After,
    Both,
}

fn ended_early(side: Short) -> AxError {
    let (subject, recovery) = match side {
        Short::Before => (
            "the earlier picture is smaller than its own header",
            "take the earlier screenshot again",
        ),
        Short::After => (
            "the later picture is smaller than its own header",
            "take the later screenshot again",
        ),
        Short::Both => (
            "both pictures are smaller than their own headers",
            "take both screenshots again",
        ),
    };
    AxError::failure(AxCode::WireMismatch, "compare two screenshots", subject)
        .with_recovery(recovery)
}

const BYTES_PER_PIXEL: usize = 4;

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
    fn boxes(&self) -> Vec<Rect> {
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

    fn group_from(&self, column: u32, row: u32, seen: &mut [bool]) -> Option<Rect> {
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

    fn box_of(&self, tiles: Tiles) -> Rect {
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
        Rect {
            x,
            y,
            width: far.saturating_sub(x),
            height: low.saturating_sub(y),
        }
    }
}

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
