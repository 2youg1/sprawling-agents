// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// The Rust reference for the leaf's four buffer rules: the rules of
// `crates/desktop/ffi/Spec.lean` written in safe Rust, so the equivalence test
// in `boundary` and the fuzz target in `crates/desktop/ffi/fuzz/` compare the
// Zig leaf with one definition of what it must do. It is compiled into
// those two and into nothing that ships; both reach `boundary` and
// `step` as siblings of this module.

use super::boundary::{Copied, Filled, Kept};
use super::step::Step;

/// The first `capacity` items of `stream` kept, and all of it counted.
pub(crate) fn keep(stream: &[usize], capacity: usize) -> Kept {
    let mut held = vec![0; capacity];
    for (slot, item) in held.iter_mut().zip(stream) {
        *slot = *item;
    }
    let step = if stream.len() > capacity {
        Step::NoRoom
    } else {
        Step::Finished
    };
    Kept {
        step: Some(step),
        count: stream.len(),
        held,
    }
}

/// The text of `block`, up to its first zero unit, copied when it fits.
pub(crate) fn text_copy(block: &[u16], capacity: usize) -> Copied {
    let end = block
        .iter()
        .position(|unit| *unit == 0)
        .unwrap_or(block.len());
    let mut into = vec![0; capacity];
    if end > capacity {
        return Copied {
            step: Some(Step::NoRoom),
            count: end,
            into,
        };
    }
    for (slot, unit) in into.iter_mut().zip(block.iter().take(end)) {
        *slot = *unit;
    }
    Copied {
        step: Some(Step::Finished),
        count: end,
        into,
    }
}

/// `units` and one terminator, into a block of exactly that length.
pub(crate) fn text_fill(units: &[u16], block_len: usize) -> Filled {
    let mut block = vec![0; block_len];
    if units.len().checked_add(1) != Some(block_len) {
        return Filled {
            step: Some(Step::Measuring),
            block,
        };
    }
    for (slot, unit) in block.iter_mut().zip(units) {
        *slot = *unit;
    }
    Filled {
        step: Some(Step::Finished),
        block,
    }
}

/// Width times height times four, for two positive sides.
pub(crate) fn bitmap_bytes(width: i32, height: i32) -> Option<usize> {
    let width = usize::try_from(width).ok().filter(|side| *side > 0)?;
    let height = usize::try_from(height).ok().filter(|side| *side > 0)?;
    width.checked_mul(height)?.checked_mul(4)
}
