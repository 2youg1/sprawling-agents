// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

const std = @import("std");
pub const part = @import("part.zig");
pub const write = @import("write.zig");
pub const scene = @import("scene.zig");
const frame = @import("frame.zig");
const quiet = @import("quiet.zig");

pub const Drawn = extern struct {
    written: usize,
    cursor_row: usize,
};

pub const panic = std.debug.no_panic;

pub fn sprawling_console_draw(bytes: [*]const u8, len: usize, into: [*]u8, capacity: usize, drawn: *Drawn) u32 {
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
        .Banner, .You, .Head, .Reasoning, .Tool, .Reply, .Note, .Ended, .Resolved, .Waiting, .Working, .Calling, .Asking, .Composer, .Menu, .Replying => error.Malformed,
    };
}

test {
    _ = @import("scene.zig");
    _ = write;
    _ = @import("text.zig");
    _ = @import("paint.zig");
    _ = @import("flow.zig");
    _ = @import("format.zig");
    _ = @import("transcript.zig");
    _ = @import("live.zig");
}

test "hostile scene bytes never trap the draw export" {
    var prng = std.Random.DefaultPrng.init(0x5eed_c01a);
    const random = prng.random();
    var buf: [4096]u8 = undefined;
    var scratch: [512]u8 = undefined;
    var drawn: Drawn = undefined;
    var round: usize = 0;
    while (round < 4096) : (round += 1) {
        const n = random.intRangeAtMost(usize, 0, scratch.len);
        random.bytes(scratch[0..n]);
        const status = sprawling_console_draw(scratch[0..n].ptr, n, &buf, buf.len, &drawn);
        try std.testing.expect(status == @backingInt(part.Status.Drawn) or status == @backingInt(part.Status.Full) or status == @backingInt(part.Status.Malformed));
    }
}
