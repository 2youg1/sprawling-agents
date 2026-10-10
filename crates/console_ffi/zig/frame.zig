// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// One frame on the main screen, as one synchronized update (mode 2026):
// the live region drawn last time erased from its first row down, the
// new transcript lines written where it stood, and the live region drawn
// again under them with the cursor put back in the composer. A terminal
// that does not know mode 2026 ignores it and draws the same bytes.

const std = @import("std");
const part = @import("part.zig");
const scene = @import("scene.zig");
const paint = @import("paint.zig");
const transcript = @import("transcript.zig");
const live = @import("live.zig");
const Grid = @import("grid.zig").Grid;

pub const Error = transcript.Error;

/// What a frame left on the screen that the next one needs: how many
/// rows the cursor stands below the live region's first row.
pub const Left = struct { cursor_row: usize };

pub fn inline_frame(out: *std.Io.Writer, r: *scene.Reader) Error!Left {
    const columns = try r.short();
    const erase = try r.short();
    const before = try r.byte();
    const entries = try r.word();
    try out.writeAll("\x1b[?2026h\x1b[?25l");
    if (erase > 0) try out.print("\x1b[{d}A", .{erase});
    try out.writeAll("\r\x1b[J");
    var p = paint.Painter.init(out, columns);
    const grid = Grid.of(&p);
    var previous: ?part.Part = std.enums.fromInt(part.Part, before);
    for (0..entries) |_| {
        const tag = try r.tag();
        if (transcript.spaced(previous, tag)) try p.end();
        try transcript.entry(&p, r, tag, grid);
        previous = tag;
    }
    var left: Left = .{ .cursor_row = 0 };
    if (!r.done()) {
        const top = p.row;
        const cursor = try live.region(&p, r, grid);
        const up = p.row - cursor.row;
        if (up > 0) try out.print("\x1b[{d}A", .{up});
        try out.print("\x1b[{d}G", .{cursor.column + 1});
        left.cursor_row = cursor.row - top;
    }
    // The cursor was hidden while the frame was drawn; it is shown again
    // whether or not a live region put it back in the composer.
    try out.writeAll("\x1b[?25h\x1b[?2026l");
    return left;
}
