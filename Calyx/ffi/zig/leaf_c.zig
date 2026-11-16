// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// C export root for the console leaf static library. Shell imports
// `leaf.zig` (no C export) so linking shell beside this library does not
// duplicate `sprawling_console_draw`.

const leaf = @import("leaf.zig");

pub const panic = leaf.panic;
pub const part = leaf.part;
pub const write = leaf.write;
pub const Drawn = leaf.Drawn;

pub export fn sprawling_console_draw(
    bytes: [*]const u8,
    len: usize,
    into: [*]u8,
    capacity: usize,
    drawn: *leaf.Drawn,
) u32 {
    return leaf.sprawling_console_draw(bytes, len, into, capacity, drawn);
}
