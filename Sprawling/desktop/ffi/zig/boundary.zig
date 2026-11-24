// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

const std = @import("std");
const Step = @import("step.zig").Step;

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

pub fn textEnd(block: []const u16) usize {
    return std.mem.indexOfScalar(u16, block, 0) orelse block.len;
}

pub const Copied = struct {
    step: Step,
    count: usize,
};

pub fn textCopy(block: []const u16, into: []u16) Copied {
    const end = textEnd(block);
    if (end > into.len) return .{ .step = .NoRoom, .count = end };
    @memcpy(into[0..end], block[0..end]);
    return .{ .step = .Finished, .count = end };
}

pub fn blockUnits(len: usize) ?usize {
    return std.math.add(usize, len, 1) catch null;
}

pub fn blockBytes(len: usize) ?usize {
    const units = blockUnits(len) orelse return null;
    return std.math.mul(usize, units, @sizeOf(u16)) catch null;
}

pub fn textFill(units: []const u16, block: []u16) Step {
    const want = blockUnits(units.len) orelse return .Measuring;
    if (block.len != want) return .Measuring;
    @memcpy(block[0..units.len], units);
    block[units.len] = 0;
    return .Finished;
}

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

fn copyHolds(block: []const u16, capacity: usize) !void {
    const sentinel: u16 = 0xa55a;
    var room: [65]u16 = @splat(sentinel);
    const into = room[0..capacity];
    const expected = std.mem.indexOfScalar(u16, block, 0) orelse block.len;
    const copied = textCopy(block, into);
    try testing.expectEqual(expected, copied.count);
    if (expected <= capacity) {
        try testing.expectEqual(Step.Finished, copied.step);
        try testing.expectEqualSlices(u16, block[0..expected], into[0..expected]);
        try testing.expect(std.mem.allEqual(u16, room[expected..], sentinel));
    } else {
        try testing.expectEqual(Step.NoRoom, copied.step);
        try testing.expect(std.mem.allEqual(u16, &room, sentinel));
    }
}

fn fillHolds(units: []const u16, capacity: usize) !void {
    const sentinel: u16 = 0xa55a;
    var block: [66]u16 = @splat(sentinel);
    try testing.expectEqual(Step.Finished, textFill(units, block[0 .. units.len + 1]));
    try testing.expectEqualSlices(u16, units, block[0..units.len]);
    try testing.expectEqual(@as(u16, 0), block[units.len]);
    try testing.expect(std.mem.allEqual(u16, block[units.len + 1 ..], sentinel));
    if (capacity != units.len + 1) {
        @memset(&block, sentinel);
        try testing.expectEqual(Step.Measuring, textFill(units, block[0..capacity]));
        try testing.expect(std.mem.allEqual(u16, &block, sentinel));
    }
}

fn keepHolds(stream: []const usize, capacity: usize) !void {
    var room: [64]usize = undefined;
    var kept: Kept(usize) = .{ .into = room[0..@min(capacity, room.len)] };
    for (stream) |item| kept.keep(item);
    try testing.expectEqual(if (stream.len > capacity) Step.NoRoom else Step.Finished, kept.step());
    try testing.expectEqual(stream.len, kept.count);
    const held = @min(stream.len, kept.into.len);
    try testing.expectEqualSlices(usize, stream[0..held], kept.into[0..held]);
}

test "fuzz: the four desktop buffer rules hold" {
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
    try fillHolds(units[0..len], capacity);
    var stream: [64]usize = undefined;
    for (0..len) |at| stream[at] = units[at];
    try keepHolds(stream[0..len], capacity);
    const width = smith.value(i32);
    const height = smith.value(i32);
    const bytes_wide = @as(i128, width) * height * 4;
    const expected = if (width > 0 and height > 0) std.math.cast(usize, bytes_wide) else null;
    try testing.expectEqual(expected, bitmapBytes(width, height));
}

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
        try fillHolds(units[0..len], capacity);
        var stream: [64]usize = undefined;
        for (0..len) |at| stream[at] = units[at];
        try keepHolds(stream[0..len], capacity);
    }
}
