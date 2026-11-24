// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

const std = @import("std");
const leaf = @import("leaf");
const write = leaf.write;

pub const MAX_HZ: u64 = 360;
pub const MIN_INTERVAL_NS: u64 = 1_000_000_000 / MAX_HZ;

pub const Cadence = struct {
    dirty: bool = false,
    last_ns: ?u64 = null,

    pub fn mark(self: *Cadence) void {
        self.dirty = true;
    }

    pub fn due(self: *const Cadence, now_ns: u64, force: bool) bool {
        if (!self.dirty) return false;
        if (force) return true;
        const last = self.last_ns orelse return true;
        return now_ns -| last >= MIN_INTERVAL_NS;
    }

    pub fn waitMs(self: *const Cadence, now_ns: u64) i32 {
        if (!self.dirty) return -1;
        const last = self.last_ns orelse return 0;
        const elapsed = now_ns -| last;
        if (elapsed >= MIN_INTERVAL_NS) return 0;
        const left_ns = MIN_INTERVAL_NS - elapsed;
        const ms = (left_ns + 999_999) / 1_000_000;
        return @intCast(@min(ms, std.math.maxInt(i32)));
    }

    pub fn presented(self: *Cadence, now_ns: u64) void {
        self.dirty = false;
        self.last_ns = now_ns;
    }
};

pub fn cloneEntry(allocator: std.mem.Allocator, entry: write.Entry) !write.Entry {
    return switch (entry) {
        .banner => |b| .{ .banner = .{
            .city = try allocator.dupe(u8, b.city),
            .address = try allocator.dupe(u8, b.address),
            .url = try allocator.dupe(u8, b.url),
        } },
        .you => |y| .{ .you = .{
            .at = y.at,
            .said = try allocator.dupe(u8, y.said),
        } },
        .head => |h| .{ .head = .{
            .at = h.at,
            .resident = try allocator.dupe(u8, h.resident),
            .facts = try cloneFacts(allocator, h.facts),
        } },
        .reasoning => |thought| .{ .reasoning = .{
            .characters = thought.characters,
            .body = try allocator.dupe(u8, thought.body),
        } },
        .tool => |t| .{ .tool = .{
            .at = t.at,
            .name = try allocator.dupe(u8, t.name),
            .subject = try allocator.dupe(u8, t.subject),
            .took_us = t.took_us,
            .outcome = t.outcome,
        } },
        .reply => |s| .{ .reply = try allocator.dupe(u8, s) },
        .note => |s| .{ .note = try allocator.dupe(u8, s) },
        .ended => |e| .{ .ended = e },
        .resolved => |r| .{ .resolved = .{
            .at = r.at,
            .verdict = r.verdict,
            .what = try allocator.dupe(u8, r.what),
        } },
    };
}

fn cloneFacts(allocator: std.mem.Allocator, facts: []const []const u8) ![]const []const u8 {
    if (facts.len == 0) return &.{};
    const out = try allocator.alloc([]const u8, facts.len);
    for (facts, 0..) |fact, i| {
        out[i] = try allocator.dupe(u8, fact);
    }
    return out;
}

test "a dirty cadence is due immediately then waits the 360 Hz interval" {
    var cadence: Cadence = .{};
    try std.testing.expect(!cadence.due(0, false));
    cadence.mark();
    try std.testing.expect(cadence.due(0, false));
    cadence.presented(1_000);
    cadence.mark();
    try std.testing.expect(!cadence.due(1_000 + MIN_INTERVAL_NS - 1, false));
    try std.testing.expect(cadence.due(1_000 + MIN_INTERVAL_NS - 1, true));
    try std.testing.expect(cadence.due(1_000 + MIN_INTERVAL_NS, false));
    try std.testing.expectEqual(@as(i32, 0), cadence.waitMs(1_000 + MIN_INTERVAL_NS));
    cadence.presented(1_000 + MIN_INTERVAL_NS);
    cadence.mark();
    try std.testing.expectEqual(@as(i32, 1), cadence.waitMs(1_000 + MIN_INTERVAL_NS + MIN_INTERVAL_NS - 1));
}

test "force presents even when the interval has not elapsed" {
    var cadence: Cadence = .{};
    cadence.mark();
    cadence.presented(10);
    cadence.mark();
    try std.testing.expect(cadence.due(10, true));
    try std.testing.expect(!cadence.due(10, false));
}

test "cloned head facts survive freeing the source model bytes" {
    const gpa = std.testing.allocator;
    const model = try gpa.dupe(u8, "provider/old");
    const facts = try gpa.alloc([]const u8, 1);
    facts[0] = model;
    const entry: write.Entry = .{ .head = .{
        .at = 12,
        .resident = "Mei",
        .facts = facts,
    } };

    var queue_arena = std.heap.ArenaAllocator.init(gpa);
    defer queue_arena.deinit();
    const cloned = try cloneEntry(queue_arena.allocator(), entry);

    gpa.free(model);
    gpa.free(facts);

    try std.testing.expectEqual(@as(?u32, 12), cloned.head.at);
    try std.testing.expectEqualStrings("Mei", cloned.head.resident);
    try std.testing.expectEqual(@as(usize, 1), cloned.head.facts.len);
    try std.testing.expectEqualStrings("provider/old", cloned.head.facts[0]);
}

test "cloned you text survives freeing the temporary said buffer" {
    const gpa = std.testing.allocator;
    const said = try gpa.dupe(u8, "resume after you");
    const entry: write.Entry = .{ .you = .{ .at = 1, .said = said } };

    var queue_arena = std.heap.ArenaAllocator.init(gpa);
    defer queue_arena.deinit();
    const cloned = try cloneEntry(queue_arena.allocator(), entry);
    gpa.free(said);

    try std.testing.expectEqualStrings("resume after you", cloned.you.said);
}
