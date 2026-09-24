// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

#![no_main]

//! V4 fuzz target: hostile bytes at the envelope span scanner, through
//! the Rust face. The Zig face is driven directly by the kernel's own
//! `std.testing.Smith` target (mem-SPEC.md section 2), so both sides of
//! the boundary see hostile input.

use libfuzzer_sys::fuzz_target;
use mem::EnvelopeKey;

fuzz_target!(|bytes: &[u8]| {
    // The FFI contract the adapter may never lose: whatever the kernel
    // answers, a span it names lies inside the input, and the borrow
    // points at exactly the value it reported.
    if let Ok(spans) = mem::scan(bytes) {
        for key in EnvelopeKey::ALL {
            if let Some(text) = spans.get(key) {
                let at = bytes
                    .windows(text.len().max(1))
                    .position(|window| window == text && !text.is_empty());
                if text.is_empty() != at.is_none() {
                    std::process::abort();
                }
            }
        }
    }
});
