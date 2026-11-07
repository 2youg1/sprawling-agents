// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The leaf's one export as Rust sees it, and the safe function around
//! it (`crates/console_ffi/Spec.lean` D1).
//!
//! The leaf reads the scene and writes the frame into a buffer this
//! module lends; a frame longer than the buffer answers `Full`, and the
//! buffer grows at most twice as long, up to [`LONGEST`]. Larger caller
//! capacities are used as supplied; `Malformed` never retries.

use std::fmt;

use crate::part::Status;
use crate::scene::Scene;

/// What the leaf writes back beside the frame, laid out as the leaf's
/// `Drawn` is.
#[repr(C)]
#[derive(Debug, Default)]
struct Drawn {
    written: usize,
    cursor_row: usize,
}

#[expect(
    unsafe_code,
    reason = "the one declaration of the leaf's export; the call into it is an `unsafe` block of its own"
)]
unsafe extern "C" {
    fn sprawling_console_draw(
        bytes: *const u8,
        len: usize,
        into: *mut u8,
        capacity: usize,
        drawn: *mut Drawn,
    ) -> u32;
}

/// The buffer a frame is first written into.
const FIRST: usize = 16 * 1024;

/// The automatic buffer growth limit. Larger caller-provided capacities
/// are used as supplied; user text and its transcript can exceed this limit.
const LONGEST: usize = 64 * 1024 * 1024;

/// What a frame left on the screen that the next frame needs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Frame {
    /// Rows between the live region's first row and the cursor.
    pub cursor_row: u16,
}

/// Why a frame was not drawn. Report a malformed scene; reduce the text
/// rendered in one frame when the output buffer is full at the growth limit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Refused {
    /// The leaf could not read the scene.
    Malformed,
    /// The leaf returned `Full` with a buffer of at least [`LONGEST`] bytes.
    TooLong,
}

impl fmt::Display for Refused {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let what = match self {
            Refused::Malformed => "the console's renderer could not read the scene it was given",
            Refused::TooLong => "the console's frame did not fit in the output buffer",
        };
        let recovery = match self {
            Refused::Malformed => "report this against crates/console_ffi",
            Refused::TooLong => "reduce the text rendered in one frame",
        };
        write!(f, "{what}; the city keeps serving; {recovery}")
    }
}

impl std::error::Error for Refused {}

/// Draws `scene` into `into`, which holds exactly the frame's bytes
/// afterwards. `into` keeps its capacity between frames, so a console
/// that reuses it allocates only when a frame is longer than any before.
/// Only `Full` retries, at most doubling the buffer up to [`LONGEST`].
/// This limits automatic growth; larger caller capacities are used unchanged.
/// `Malformed` returns immediately without retrying.
///
/// # Errors
/// [`Refused::Malformed`] for a scene the leaf cannot read, and
/// [`Refused::TooLong`] when the leaf returns `Full` with a buffer of at
/// least [`LONGEST`] bytes, including for large user text and transcripts.
pub fn draw(scene: &Scene, into: &mut Vec<u8>) -> Result<Frame, Refused> {
    let bytes = scene.bytes();
    let mut capacity = into.capacity().max(FIRST);
    loop {
        into.clear();
        into.resize(capacity, 0);
        let mut drawn = Drawn::default();
        // SAFETY: `bytes` and `into` are live slices whose lengths are
        // passed beside their pointers, and `drawn` is a local of the
        // leaf's `Drawn` layout; the leaf reads at most `bytes.len()`
        // bytes, writes at most `into.len()` bytes and one `Drawn`, and
        // keeps none of the pointers past the call.
        #[expect(unsafe_code, reason = "the one call into the console's Zig leaf")]
        let status = unsafe {
            sprawling_console_draw(
                bytes.as_ptr(),
                bytes.len(),
                into.as_mut_ptr(),
                into.len(),
                &raw mut drawn,
            )
        };
        if status == Status::Drawn.number() {
            into.truncate(drawn.written);
            return Ok(Frame {
                cursor_row: u16::try_from(drawn.cursor_row).unwrap_or(u16::MAX),
            });
        }
        if status == Status::Full.number() && capacity < LONGEST {
            capacity = capacity.saturating_mul(2).min(LONGEST);
            continue;
        }
        into.clear();
        return Err(if status == Status::Full.number() {
            Refused::TooLong
        } else {
            Refused::Malformed
        });
    }
}
