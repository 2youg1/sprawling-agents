// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// What the leaf does with the buffers it is lent, and nothing else.
//
// Every operation in `leaf.zig` writes into memory Rust owns, and these
// four rules are the whole of what it writes there: a stream of handles
// kept up to the buffer's length and counted in full, a clipboard text
// copied up to its first zero unit and never past the block, a new text
// written with exactly one terminator into a block of exactly that size,
// and a bitmap measured in bytes without overflow. They touch no Win32
// call, so they are judged three ways without a desktop: the theorems of
// `crates/desktop/ffi/Spec.lean`, the tests below, and the Rust reference the
// equivalence check and the fuzz target compare them with.

const std = @import("std");
const Step = @import("step.zig").Step;

/// A stream of items kept in a buffer as long as it lasts, and counted
/// past it, so a caller whose buffer was short learns how long to make
/// the next one instead of reading a list that silently stopped.
pub fn Kept(comptime T: type) type {
    return struct {
        into: []T,
        count: usize = 0,

        const Self = @This();

        pub fn keep(self: *Self, item: T) void {
            if (self.count < self.into.len) self.into[self.count] = item;
            self.count +|= 1;
        }

        pub fn step(self: Self) Step {
            return if (self.count > self.into.len) .NoRoom else .Finished;
        }
    };
}

/// Where a text another program wrote ends: at its first zero unit, or
/// at the end of the block when it has none. The block's own size is the
/// bound; its terminator is a claim this rule does not rely on.
pub fn textEnd(block: []const u16) usize {
    return std.mem.indexOfScalar(u16, block, 0) orelse block.len;
}

/// The outcome of a copy: finished with `count` units written, or no
/// room, with `count` the units the caller must make room for.
pub const Copied = struct {
    step: Step,
    count: usize,
};

/// The text of `block` copied into `into`, or how much room it needs.
/// Nothing is written when the room is short.
pub fn textCopy(block: []const u16, into: []u16) Copied {
    const end = textEnd(block);
    if (end > into.len) return .{ .step = .NoRoom, .count = end };
    @memcpy(into[0..end], block[0..end]);
    return .{ .step = .Finished, .count = end };
}

/// How many units a block for `len` units of text holds: the text and
/// its one terminator, or nothing when that count overflows.
pub fn blockUnits(len: usize) ?usize {
    return std.math.add(usize, len, 1) catch null;
}

/// The same block in bytes, which is what `GlobalAlloc` is asked for.
pub fn blockBytes(len: usize) ?usize {
    const units = blockUnits(len) orelse return null;
    return std.math.mul(usize, units, @sizeOf(u16)) catch null;
}

/// `units` and one terminator written into a block that holds exactly
/// them. A block of any other length is refused before anything is
/// written, because a size that does not describe its block is a defect
/// in the caller, not a text to shorten.
pub fn textFill(units: []const u16, block: []u16) Step {
    const want = blockUnits(units.len) orelse return .Measuring;
    if (block.len != want) return .Measuring;
    @memcpy(block[0..units.len], units);
    block[units.len] = 0;
    return .Finished;
}

/// The bytes of a top-down 32-bit bitmap `width` by `height`, or nothing
/// for a side that is not positive or a product that overflows.
pub fn bitmapBytes(width: i32, height: i32) ?usize {
    if (width <= 0 or height <= 0) return null;
    const pixels = std.math.mul(usize, @intCast(width), @intCast(height)) catch return null;
    return std.math.mul(usize, pixels, 4) catch null;
}

const testing = std.testing;

test "a stream longer than its buffer is kept to the buffer and counted in full" {
    var into: [2]usize = .{ 0, 0 };
    var kept: Kept(usize) = .{ .into = &into };
    for ([_]usize{ 7, 8, 9 }) |item| kept.keep(item);
    try testing.expectEqual(Step.NoRoom, kept.step());
    try testing.expectEqual(@as(usize, 3), kept.count);
    try testing.expectEqualSlices(usize, &.{ 7, 8 }, &into);
}

test "a text ends at its first zero unit or at the end of its block" {
    try testing.expectEqual(@as(usize, 2), textEnd(&.{ 'a', 'b', 0, 'c' }));
    try testing.expectEqual(@as(usize, 3), textEnd(&.{ 'a', 'b', 'c' }));
    try testing.expectEqual(@as(usize, 0), textEnd(&.{}));
}

test "a copy with too little room writes nothing and names the room it needs" {
    var into: [1]u16 = .{0x55};
    const copied = textCopy(&.{ 'a', 'b', 0 }, &into);
    try testing.expectEqual(Copied{ .step = .NoRoom, .count = 2 }, copied);
    try testing.expectEqual(@as(u16, 0x55), into[0]);
}

test "a filled block holds the text and exactly one terminator" {
    var block: [3]u16 = undefined;
    try testing.expectEqual(Step.Finished, textFill(&.{ 'h', 'i' }, &block));
    try testing.expectEqualSlices(u16, &.{ 'h', 'i', 0 }, &block);
    var short: [2]u16 = .{ 1, 1 };
    try testing.expectEqual(Step.Measuring, textFill(&.{ 'h', 'i' }, &short));
    try testing.expectEqualSlices(u16, &.{ 1, 1 }, &short);
}

test "a bitmap's bytes are refused for a side that is not positive or a product that overflows" {
    try testing.expectEqual(@as(?usize, 64 * 32 * 4), bitmapBytes(64, 32));
    try testing.expectEqual(@as(?usize, null), bitmapBytes(0, 32));
    try testing.expectEqual(@as(?usize, null), bitmapBytes(-1, 32));
    try testing.expectEqual(@as(?usize, null), blockBytes(std.math.maxInt(usize)));
}

/// The properties `crates/desktop/ffi/Spec.lean` proves of the copy, asked of
/// one input: what is written is the block's text and fits the room,
/// and a room as long as the answer always suffices.
fn copyHolds(block: []const u16, capacity: usize) !void {
    var room: [64]u16 = undefined;
    const into = room[0..@min(capacity, room.len)];
    const copied = textCopy(block, into);
    try testing.expect(copied.count <= block.len);
    switch (copied.step) {
        .Finished => {
            try testing.expect(copied.count <= into.len);
            try testing.expectEqualSlices(u16, block[0..copied.count], into[0..copied.count]);
            try testing.expect(std.mem.indexOfScalar(u16, into[0..copied.count], 0) == null);
        },
        .NoRoom => {
            try testing.expect(copied.count > into.len);
            var enough: [64]u16 = undefined;
            const again = textCopy(block, enough[0..copied.count]);
            try testing.expectEqual(Step.Finished, again.step);
        },
        else => return error.UnexpectedStep,
    }
}

/// The fill and the copy are each other's inverse on a text with no zero
/// in it, which is what a clipboard round trip relies on.
fn fillHolds(units: []const u16) !void {
    var block: [65]u16 = undefined;
    const used = block[0 .. units.len + 1];
    try testing.expectEqual(Step.Finished, textFill(units, used));
    if (std.mem.indexOfScalar(u16, units, 0) == null) {
        try testing.expectEqual(units.len, textEnd(used));
    }
}

/// The keep rule asked of one stream: the buffer holds the stream's
/// prefix, and the count is the stream's length.
fn keepHolds(stream: []const usize, capacity: usize) !void {
    var room: [64]usize = undefined;
    var kept: Kept(usize) = .{ .into = room[0..@min(capacity, room.len)] };
    for (stream) |item| kept.keep(item);
    try testing.expectEqual(stream.len, kept.count);
    const held = @min(stream.len, kept.into.len);
    try testing.expectEqualSlices(usize, stream[0..held], kept.into[0..held]);
}

// Coverage-guided where Zig's fuzzer runs (`zig test --fuzz` is not
// implemented for Windows today), and one trivial input elsewhere.
test "fuzz: the copy, the fill and the keep hold their properties" {
    try testing.fuzz({}, fuzzOne, .{});
}

fn fuzzOne(_: void, smith: *testing.Smith) !void {
    var bytes: [128]u8 = undefined;
    const n = smith.slice(&bytes);
    var units: [64]u16 = undefined;
    const len = n / 2;
    for (0..len) |at| units[at] = std.mem.readInt(u16, bytes[at * 2 ..][0..2], .little);
    const capacity: usize = smith.valueRangeAtMost(u8, 0, 64);
    try copyHolds(units[0..len], capacity);
    try fillHolds(units[0..len]);
    var stream: [64]usize = undefined;
    for (0..len) |at| stream[at] = units[at];
    try keepHolds(stream[0..len], capacity);
}

// Hand-rolled and seeded, so it runs on every platform and replays the
// same inputs on every run: zero units are drawn often on purpose,
// because where the first one falls is what the copy is about.
test "seeded: the copy, the fill and the keep hold their properties" {
    var prng = std.Random.DefaultPrng.init(0x5eed_de5c_70b0);
    const random = prng.random();
    for (0..20_000) |_| {
        var units: [64]u16 = undefined;
        const len = random.uintLessThan(usize, units.len + 1);
        for (units[0..len]) |*unit| {
            unit.* = if (random.uintLessThan(u8, 8) == 0) 0 else random.int(u16);
        }
        const capacity = random.uintLessThan(usize, units.len + 1);
        try copyHolds(units[0..len], capacity);
        try fillHolds(units[0..len]);
        var stream: [64]usize = undefined;
        for (0..len) |at| stream[at] = units[at];
        try keepHolds(stream[0..len], capacity);
    }
}
