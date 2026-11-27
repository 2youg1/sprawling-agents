// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

const std = @import("std");
pub const part = @import("part.zig");
pub const write = @import("write.zig");
pub const scene = @import("scene.zig");
pub const text = @import("text.zig");
pub const viewport = @import("viewport.zig");
const frame = @import("frame.zig");
pub const quiet = @import("quiet.zig");

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
    _ = quiet;
    _ = @import("text.zig");
    _ = @import("paint.zig");
    _ = @import("flow.zig");
    _ = @import("format.zig");
    _ = @import("transcript.zig");
    _ = @import("live.zig");
    _ = @import("viewport.zig");
    _ = @import("prose.zig");
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

test "fuzz: the draw export respects the output capacity" {
    try std.testing.fuzz({}, fuzzDraw, .{});
}

fn fuzzDraw(_: void, smith: *std.testing.Smith) !void {
    var bytes: [128]u8 = undefined;
    const input = bytes[0..smith.slice(&bytes)];
    const capacity = smith.valueRangeAtMost(u16, 0, 2048);
    var into: [2049]u8 = undefined;
    _ = try checkDraw(input, &into, capacity);

    const columns = smith.valueRangeAtMost(u16, 1, 64);
    const rows = smith.valueRangeAtMost(u16, 1, 12);
    var encoded: [512]u8 = undefined;
    var writer: std.Io.Writer = .fixed(&encoded);
    if (smith.value(bool)) {
        try write.writeInline(&writer, .{
            .columns = columns,
            .rows = rows,
            .erase = 0,
            .previous = null,
            .entries = &.{.{ .note = input }},
        });
    } else {
        try write.writeQuiet(&writer, .{ .columns = columns, .rows = rows, .url = "", .code = input });
    }
    const frame_bytes = writer.buffered();
    var ample: [8193]u8 = undefined;
    const rendered = switch (try checkDraw(frame_bytes, &ample, ample.len - 1)) {
        .drawn => |output| output,
        else => return error.ValidFrameRefused,
    };
    const actual = try checkDraw(frame_bytes, &into, capacity);
    if (rendered.len <= capacity) {
        switch (actual) {
            .drawn => |output| try std.testing.expectEqualSlices(u8, rendered, output),
            else => return error.FrameRefused,
        }
    } else {
        try std.testing.expect(actual == .full);
    }
}

const DrawResult = union(enum) { drawn: []const u8, full, malformed };

fn checkDraw(input: []const u8, into: []u8, capacity: usize) !DrawResult {
    const sentinel: u8 = 0xa5;
    @memset(into[capacity..], sentinel);
    const initial = Drawn{ .written = std.math.maxInt(usize), .cursor_row = std.math.maxInt(usize) };
    var drawn = initial;
    const status = sprawling_console_draw(input.ptr, input.len, into.ptr, capacity, &drawn);
    try std.testing.expect(std.mem.allEqual(u8, into[capacity..], sentinel));
    switch (status) {
        @backingInt(part.Status.Drawn) => {
            try std.testing.expect(drawn.written <= capacity);
            return .{ .drawn = into[0..drawn.written] };
        },
        @backingInt(part.Status.Full), @backingInt(part.Status.Malformed) => {
            try std.testing.expectEqual(initial, drawn);
            return if (status == @backingInt(part.Status.Full)) .full else .malformed;
        },
        else => return error.UnknownDrawStatus,
    }
}
