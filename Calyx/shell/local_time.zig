// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

const std = @import("std");
const builtin = @import("builtin");

pub const TimeMs = u64;

pub const TimeOfDay = struct {
    seconds: u32,

    pub fn fromSeconds(seconds: u32) ?TimeOfDay {
        if (seconds >= 86_400) return null;
        return .{ .seconds = seconds };
    }
};

pub const Zone = *const fn (TimeMs) ?TimeOfDay;

const time_t = i64;
const Tm = extern struct {
    tm_sec: c_int,
    tm_min: c_int,
    tm_hour: c_int,
    tm_mday: c_int,
    tm_mon: c_int,
    tm_year: c_int,
    tm_wday: c_int,
    tm_yday: c_int,
    tm_isdst: c_int,
    tm_gmtoff: c_long,
    tm_zone: ?[*:0]const u8,
};

extern "c" fn localtime_r(timer: *const time_t, result: *Tm) ?*Tm;

pub fn local(at: TimeMs) ?TimeOfDay {
    if (builtin.os.tag == .windows) return utc(at);
    const millis: i64 = std.math.cast(i64, at) orelse return null;
    var epoch: time_t = @divTrunc(millis, 1000);
    var moment: Tm = undefined;
    if (localtime_r(&epoch, &moment) == null) return null;
    const hour: u32 = @intCast(moment.tm_hour);
    const min: u32 = @intCast(moment.tm_min);
    const sec: u32 = @intCast(moment.tm_sec);
    return TimeOfDay.fromSeconds(hour * 3600 + min * 60 + sec);
}

pub fn utc(at: TimeMs) ?TimeOfDay {
    const seconds = std.math.divTrunc(u64, at, 1_000) catch return null;
    const day = std.math.rem(u64, seconds, 86_400) catch return null;
    return TimeOfDay.fromSeconds(std.math.cast(u32, day) orelse return null);
}

test "utc maps milliseconds into the day" {
    const noon = utc(12 * 3600 * 1000).?;
    try std.testing.expectEqual(@as(u32, 12 * 3600), noon.seconds);
}

test "utc refuses a second past the day" {
    try std.testing.expect(TimeOfDay.fromSeconds(86_400) == null);
}

test "utc wraps across midnight" {
    const almost = utc(86_400 * 1000 - 1).?;
    try std.testing.expectEqual(@as(u32, 86_399), almost.seconds);
    const next = utc(86_400 * 1000).?;
    try std.testing.expectEqual(@as(u32, 0), next.seconds);
}
