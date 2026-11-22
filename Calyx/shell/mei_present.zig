// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

const std = @import("std");

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
