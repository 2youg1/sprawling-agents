// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// The console's renderer: one export that reads a scene Rust lent and
// writes the bytes of one frame into a buffer Rust lent, and nothing
// else. It reads no clock, opens no file and keeps no state between
// calls: the frame is a function of the scene and the width in it.
//
// Every pointer is lent for the call alone; the leaf writes nothing past
// `capacity`, and a frame that does not fit answers `full` so the caller
// can lend a longer buffer and ask again.

const std = @import("std");
const part = @import("part.zig");
const scene = @import("scene.zig");
const frame = @import("frame.zig");
const quiet = @import("quiet.zig");

/// The frame's length and what it left on the screen.
pub const Drawn = extern struct {
    written: usize,
    cursor_row: usize,
};

/// A failed safety check traps: no stack trace machinery is linked, so
/// the leaf carries no symbol lookup and builds the same on every target.
pub const panic = std.debug.no_panic;

export fn sprawling_console_draw(bytes: [*]const u8, len: usize, into: [*]u8, capacity: usize, drawn: *Drawn) u32 {
    var out: std.Io.Writer = .fixed(into[0..capacity]);
    var r: scene.Reader = .{ .bytes = bytes[0..len] };
    const left = draw(&out, &r) catch |err| return @backingInt(switch (err) {
        error.WriteFailed => part.Status.Full,
        error.Malformed => part.Status.Malformed,
    });
    drawn.* = .{ .written = out.end, .cursor_row = left.cursor_row };
    return @backingInt(part.Status.Drawn);
}

fn draw(out: *std.Io.Writer, r: *scene.Reader) frame.Error!frame.Left {
    return switch (try r.tag()) {
        .Inline => frame.inline_frame(out, r),
        .Quiet => blk: {
            try quiet.quiet(out, r);
            break :blk .{ .cursor_row = 0 };
        },
        .Banner, .You, .Head, .Reasoning, .Tool, .Reply, .Note, .Ended, .Resolved, .Waiting, .Working, .Calling, .Asking, .Composer, .Menu => error.Malformed,
    };
}

test {
    _ = @import("scene.zig");
    _ = @import("text.zig");
    _ = @import("paint.zig");
    _ = @import("flow.zig");
    _ = @import("format.zig");
    _ = @import("transcript.zig");
    _ = @import("live.zig");
}
