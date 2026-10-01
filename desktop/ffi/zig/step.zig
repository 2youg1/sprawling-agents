// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// The Zig spelling of `desktop/ffi/src/step.rs`, which is the definition.
// `build.rs` compares this file with that one on every build and refuses
// when they differ, printing the text this file must hold.

pub const Step = enum(u32) {
    Finished = 0,
    Absent = 1,
    NoRoom = 2,
    Measuring = 3,
    Listing = 4,
    NoWindow = 5,
    Context = 6,
    Bitmap = 7,
    Selecting = 8,
    Drawing = 9,
    Reading = 10,
    ShortRows = 11,
    Owner = 12,
    Opening = 13,
    Fetching = 14,
    Locking = 15,
    EmptyBlock = 16,
    Allocating = 17,
    Emptying = 18,
    Handing = 19,
};
