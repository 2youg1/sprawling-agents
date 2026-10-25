// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! What a read of a file costs, the handle read against the judged path
//! it replaced, interleaved in one process so both sides see the same
//! machine (`crates/runtime/spec/Tools/Read.lean` D113).
//!
//! The judged side re-resolves the city root on every call, as the
//! judged path did before the root was anchored, and cuts with the cut
//! the one-pass cut replaced; the handle side also takes the two stamps
//! `read` records for `edit`. The ratio test holds the handle
//! read within a bound of one resolution of the path followed by a bare
//! buffered read of the same bytes, which is the design's claim and a
//! ratio every machine agrees on: going back to four resolutions and a
//! second open per read crosses it.

#![allow(
    clippy::disallowed_methods,
    clippy::arithmetic_side_effects,
    reason = "test code: an instrument reads the wall clock"
)]

use std::time::{Duration, Instant};

use super::same_bytes::reference_cut;
use super::*;
use crate::tools::chosen_path::CityRoot;

/// One file of `size` bytes of ordinary source-shaped lines under
/// `lab/`, and its address.
fn fixture(size: usize) -> (tempfile::TempDir, kernel::Address) {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("lab")).unwrap();
    let line = "    let value = compute(left, right).unwrap_or_default();\n";
    let text: String = line.repeat(size / line.len() + 1);
    std::fs::write(
        dir.path().join("lab").join("file.rs"),
        text.get(..size).unwrap(),
    )
    .unwrap();
    (dir, kernel::Address::parse("lab/file.rs").unwrap())
}

fn open_everywhere() -> ReadBound {
    Arc::new(|_: &kernel::Address| kernel::ReadVerdict::Open)
}

/// The judged path as it stood: the root resolved again, the file
/// resolved, opened and judged again, every line collected for the cut.
fn judged(root: &Path, addr: &kernel::Address, bound: &ReadBound) -> usize {
    let city = CityRoot::anchor(root, "read").unwrap();
    let (text, _, _) = super::engine::judged_read(&city, addr, addr.as_str(), &**bound).unwrap();
    let (cut, _, _) = reference_cut(0, LINE_CAP, &text);
    cut.len()
}

/// The handle read with the one-pass cut.
fn by_handle(city: &CityRoot, addr: &kernel::Address, bound: &ReadBound) -> usize {
    let (text, _, _) = super::engine::text_of(city, addr, addr.as_str(), &**bound).unwrap();
    let cut = Interval {
        offset: 0,
        limit: LINE_CAP,
    }
    .cut(&text);
    cut.text.len()
}

/// A bare buffered read of the same bytes, cut: the floor no
/// judgement can go under.
fn bare(root: &Path, addr: &kernel::Address) -> usize {
    read_and_cut(&root.join(addr.as_str()))
}

/// One resolution of the path, then a bare read of where it lies: what
/// the handle read claims to cost, one judgement of the real location
/// and one open.
fn resolved(root: &Path, addr: &kernel::Address) -> usize {
    read_and_cut(&std::fs::canonicalize(root.join(addr.as_str())).unwrap())
}

fn read_and_cut(path: &Path) -> usize {
    let text = std::fs::read_to_string(path).unwrap();
    let cut = Interval {
        offset: 0,
        limit: LINE_CAP,
    }
    .cut(&text);
    cut.text.len()
}

fn timed(work: &mut dyn FnMut() -> usize) -> Duration {
    let started = Instant::now();
    std::hint::black_box(work());
    started.elapsed()
}

/// The `rank`-th per-mille sample, nearest rank.
fn at(samples: &mut [Duration], per_mille: usize) -> Duration {
    samples.sort_unstable();
    let index = (samples.len() * per_mille).div_ceil(1000).saturating_sub(1);
    samples[index.min(samples.len() - 1)]
}

/// Interleaves the sides round by round, so a machine that slows down
/// mid-run slows every side alike; one warm-up round per side first.
fn interleaved(rounds: usize, sides: &mut [&mut dyn FnMut() -> usize]) -> Vec<Vec<Duration>> {
    for side in sides.iter_mut() {
        timed(*side);
    }
    let mut samples = vec![Vec::with_capacity(rounds); sides.len()];
    for _ in 0..rounds {
        for (side, out) in sides.iter_mut().zip(samples.iter_mut()) {
            out.push(timed(*side));
        }
    }
    samples
}

fn micros(duration: Duration) -> f64 {
    duration.as_secs_f64() * 1e6
}

#[test]
#[ignore = "an instrument: run it by name with --run-ignored only --no-capture"]
fn instrument_read_ab() {
    let bound = open_everywhere();
    for size in [20 << 10, 100 << 10, 1 << 20, 8 << 20] {
        let (dir, addr) = fixture(size);
        let city = CityRoot::anchor(dir.path(), "read").unwrap();
        let rounds = if size >= 8 << 20 { 60 } else { 400 };
        let samples = interleaved(
            rounds,
            &mut [
                &mut || judged(dir.path(), &addr, &bound),
                &mut || by_handle(&city, &addr, &bound),
                &mut || resolved(dir.path(), &addr),
                &mut || bare(dir.path(), &addr),
            ],
        );
        let [mut judged, mut handle, mut resolved, mut bare]: [Vec<Duration>; 4] =
            samples.try_into().unwrap();
        println!(
            "read_ab size={size} rounds={rounds} judged_p50_us={:.1} judged_p95_us={:.1} \
             handle_p50_us={:.1} handle_p95_us={:.1} resolved_p50_us={:.1} \
             resolved_p95_us={:.1} bare_p50_us={:.1} bare_p95_us={:.1}",
            micros(at(&mut judged, 500)),
            micros(at(&mut judged, 950)),
            micros(at(&mut handle, 500)),
            micros(at(&mut handle, 950)),
            micros(at(&mut resolved, 500)),
            micros(at(&mut resolved, 950)),
            micros(at(&mut bare, 500)),
            micros(at(&mut bare, 950)),
        );
    }
}

/// The edit-then-read sequence: one byte written through the page cache
/// just before each read, so the read meets a dirty page.
#[test]
#[ignore = "an instrument: run it by name with --run-ignored only --no-capture"]
fn instrument_read_after_write_ab() {
    let bound = open_everywhere();
    let size = 100 << 10;
    let (dir, addr) = fixture(size);
    let city = CityRoot::anchor(dir.path(), "read").unwrap();
    let path = dir.path().join("lab").join("file.rs");
    let touch = || {
        use std::io::{Seek as _, Write as _};
        let mut file = std::fs::OpenOptions::new().write(true).open(&path).unwrap();
        file.seek(std::io::SeekFrom::Start(0)).unwrap();
        file.write_all(b" ").unwrap();
    };
    // The write sits outside the timed span: it is the edit, and its
    // close is what an on-access scanner waits on.
    let (mut judged_after, mut handle_after) = (Vec::new(), Vec::new());
    for _ in 0..100 {
        touch();
        judged_after.push(timed(&mut || judged(dir.path(), &addr, &bound)));
        touch();
        handle_after.push(timed(&mut || by_handle(&city, &addr, &bound)));
    }
    println!(
        "read_after_write_ab size={size} judged_p50_us={:.1} judged_p95_us={:.1} \
         handle_p50_us={:.1} handle_p95_us={:.1}",
        micros(at(&mut judged_after, 500)),
        micros(at(&mut judged_after, 950)),
        micros(at(&mut handle_after, 500)),
        micros(at(&mut handle_after, 950)),
    );
}

/// The ceiling CI holds (`tools/xtask/budgets.toml` `[read_handle_ratio]`):
/// the handle read of a 20 KiB file, where the fixed cost of resolving
/// and opening dominates, stays within this many resolved reads of it.
#[cfg(any(windows, target_os = "linux"))]
const RATIO_CEILING: f64 = 1.5;

/// Only where the engine reads through the handle: elsewhere the judged
/// path is the read, and it is meant to cost more than a bare read.
#[cfg(any(windows, target_os = "linux"))]
#[test]
fn a_read_costs_within_a_bound_of_a_resolved_read() {
    let bound = open_everywhere();
    let (dir, addr) = fixture(20 << 10);
    let city = CityRoot::anchor(dir.path(), "read").unwrap();
    let samples = interleaved(
        200,
        &mut [&mut || by_handle(&city, &addr, &bound), &mut || {
            resolved(dir.path(), &addr)
        }],
    );
    let [mut handle, mut resolved]: [Vec<Duration>; 2] = samples.try_into().unwrap();
    let ratio = micros(at(&mut handle, 500)) / micros(at(&mut resolved, 500));
    assert!(
        ratio <= RATIO_CEILING,
        "a 20 KiB read through the handle cost {ratio:.2} resolved reads, above \
         {RATIO_CEILING}: a resolution or an open came back onto the read path"
    );
}
