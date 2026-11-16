// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The leaf's buffer rules, called on their own: the very functions of
//! `zig/boundary.zig` the operations write Rust's memory with.
//!
//! They are public so the judge below reaches them through the door
//! production uses: the equivalence check compares each with the Rust
//! reference on seeded inputs, twenty thousand per rule on every test
//! run, and as many as `just fuzz-desktop` asks for from a seed it
//! names. [`bitmap_bytes`] is also the size the capture lends its buffer
//! at. Every buffer here starts zeroed, so what a rule left untouched is
//! part of what the two sides are compared on.

use crate::ended;
use crate::leaf;
use crate::step::Step;

/// A stream of handles kept in a buffer of `capacity`: the step, the
/// stream's full count, and the whole buffer as the rule left it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Kept {
    pub step: Option<Step>,
    pub count: usize,
    pub held: Vec<usize>,
}

/// A text copied into a buffer of `capacity`: the step, the units
/// written or needed, and the whole buffer as the rule left it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Copied {
    pub step: Option<Step>,
    pub count: usize,
    pub into: Vec<u16>,
}

/// A text written into a block of `block_len`: the step, and the whole
/// block as the rule left it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Filled {
    pub step: Option<Step>,
    pub block: Vec<u16>,
}

/// `stream` through the leaf's keep rule.
#[must_use]
pub fn keep(stream: &[usize], capacity: usize) -> Kept {
    let mut held = vec![0; capacity];
    let mut count = 0;
    // SAFETY: `stream` is a live slice the leaf only reads, `held` is
    // `capacity` initialised slots lent to this call alone, and both
    // lengths travel beside their pointers.
    #[expect(unsafe_code, reason = "the leaf's keep rule, judged on its own")]
    let raw = unsafe {
        leaf::sprawling_desktop_keep(
            stream.as_ptr(),
            stream.len(),
            held.as_mut_ptr(),
            held.len(),
            &raw mut count,
        )
    };
    Kept {
        step: ended::read(raw),
        count,
        held,
    }
}

/// `block` through the leaf's copy rule.
#[must_use]
pub fn text_copy(block: &[u16], capacity: usize) -> Copied {
    let mut into = vec![0; capacity];
    let mut count = 0;
    // SAFETY: `block` is a live slice the leaf only reads, `into` is
    // `capacity` initialised units lent to this call alone, and both
    // lengths travel beside their pointers.
    #[expect(unsafe_code, reason = "the leaf's copy rule, judged on its own")]
    let raw = unsafe {
        leaf::sprawling_desktop_text_copy(
            block.as_ptr(),
            block.len(),
            into.as_mut_ptr(),
            into.len(),
            &raw mut count,
        )
    };
    Copied {
        step: ended::read(raw),
        count,
        into,
    }
}

/// `units` through the leaf's fill rule, into a block of `block_len`.
#[must_use]
pub fn text_fill(units: &[u16], block_len: usize) -> Filled {
    let mut block = vec![0; block_len];
    // SAFETY: `units` is a live slice the leaf only reads, `block` is
    // `block_len` initialised units lent to this call alone, and both
    // lengths travel beside their pointers.
    #[expect(unsafe_code, reason = "the leaf's fill rule, judged on its own")]
    let raw = unsafe {
        leaf::sprawling_desktop_text_fill(
            units.as_ptr(),
            units.len(),
            block.as_mut_ptr(),
            block.len(),
        )
    };
    Filled {
        step: ended::read(raw),
        block,
    }
}

/// The bytes the leaf's rule gives a `width` by `height` bitmap, or
/// `None` for a side that is not positive or a size that overflows.
#[must_use]
pub fn bitmap_bytes(width: i32, height: i32) -> Option<usize> {
    let mut bytes = 0;
    // SAFETY: two integers by value, and `bytes` is a live local the
    // leaf writes one `usize` into.
    #[expect(
        unsafe_code,
        reason = "the leaf's bitmap rule, which the capture lends at"
    )]
    let raw = unsafe { leaf::sprawling_desktop_bitmap_bytes(width, height, &raw mut bytes) };
    (ended::read(raw) == Some(Step::Finished)).then_some(bytes)
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    clippy::as_conversions,
    reason = "test code"
)]
mod tests {
    use super::*;
    use crate::reference;

    /// A seeded generator, so a disagreement replays from the seed in
    /// its message on every machine.
    struct Draws(u64);

    impl Draws {
        fn next(&mut self) -> u64 {
            self.0 ^= self.0 << 13;
            self.0 ^= self.0 >> 7;
            self.0 ^= self.0 << 17;
            self.0
        }

        fn below(&mut self, bound: u64) -> usize {
            (self.next() % bound) as usize
        }

        /// A text in which zero units are frequent, because where the
        /// first one falls is what the copy rule is about.
        fn units(&mut self) -> Vec<u16> {
            let len = self.below(48);
            (0..len)
                .map(|_| match self.below(6) {
                    0 => 0,
                    _ => (self.next() & 0xffff) as u16,
                })
                .collect()
        }
    }

    /// `rounds` drawn inputs from `seed`, each through the leaf and the
    /// reference, for each of the four rules.
    fn agree(seed: u64, rounds: u64) {
        let mut draws = Draws(seed);
        for round in 0..rounds {
            let units = draws.units();
            let capacity = draws.below(52);
            assert_eq!(
                text_copy(&units, capacity),
                reference::text_copy(&units, capacity),
                "seed {seed:#x} round {round}: copy {units:?} into {capacity}"
            );
            let block_len = units.len() + draws.below(3);
            assert_eq!(
                text_fill(&units, block_len),
                reference::text_fill(&units, block_len),
                "seed {seed:#x} round {round}: fill {units:?} into {block_len}"
            );
            let stream: Vec<usize> = units.iter().map(|unit| usize::from(*unit)).collect();
            assert_eq!(
                keep(&stream, capacity),
                reference::keep(&stream, capacity),
                "seed {seed:#x} round {round}: keep {stream:?} in {capacity}"
            );
            let (width, height) = (
                draws.next() as i32 >> draws.below(31),
                draws.next() as i32 >> draws.below(31),
            );
            assert_eq!(
                bitmap_bytes(width, height),
                reference::bitmap_bytes(width, height),
                "seed {seed:#x} round {round}: bitmap {width}x{height}"
            );
        }
    }

    /// The leaf and the Rust reference give the same answer, buffers
    /// included, on every drawn input, for each of the four rules.
    #[test]
    fn the_leaf_and_the_rust_reference_agree_on_every_drawn_input() {
        agree(0x5eed_de5c_70b0_0001, 20_000);
    }

    /// The same comparison for as long as a person asks: the rounds and
    /// the seed come from `DESKTOP_FFI_FUZZ_ROUNDS` and
    /// `DESKTOP_FFI_FUZZ_SEED`, and a disagreement prints the seed and
    /// the round it replays from. libFuzzer has no platform for this
    /// leaf today (`crates/desktop/Spec.lean` D12), so the Rust side is
    /// fuzzed by drawing rather than by coverage.
    #[test]
    #[ignore = "runs for as long as DESKTOP_FFI_FUZZ_ROUNDS asks; `just fuzz-desktop` runs it"]
    fn the_leaf_and_the_rust_reference_agree_for_as_long_as_asked() {
        let asked = |name: &str| {
            std::env::var(name)
                .ok()
                .and_then(|value| u64::from_str_radix(value.trim_start_matches("0x"), 16).ok())
        };
        let rounds = std::env::var("DESKTOP_FFI_FUZZ_ROUNDS")
            .ok()
            .and_then(|value| value.parse().ok())
            .unwrap_or(1_000_000);
        // Xorshift never leaves zero, so a zero seed is taken as one.
        let seed = asked("DESKTOP_FFI_FUZZ_SEED").unwrap_or(0x5eed_f022).max(1);
        agree(seed, rounds);
    }

    /// The edges a drawn input reaches rarely, written down.
    #[test]
    fn the_leaf_and_the_rust_reference_agree_at_the_edges() {
        for (width, height) in [(0, 1), (1, 0), (-1, 1), (i32::MAX, i32::MAX), (1, 1)] {
            assert_eq!(
                bitmap_bytes(width, height),
                reference::bitmap_bytes(width, height)
            );
        }
        for capacity in 0..4 {
            assert_eq!(
                text_copy(&[], capacity),
                reference::text_copy(&[], capacity)
            );
            assert_eq!(keep(&[], capacity), reference::keep(&[], capacity));
        }
        assert_eq!(text_fill(&[], 0), reference::text_fill(&[], 0));
        assert_eq!(text_fill(&[], 1), reference::text_fill(&[], 1));
    }
}
