// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// Numbers as a person reads them on a line: the time of day, a duration
// in the unit that keeps it to three or four figures, and a count with
// its thousands marked. Each writes into a buffer the caller owns and
// answers the text, so a caller can measure it before placing it.

const std = @import("std");

pub const Buffer = [32]u8;

/// `HH:MM:SS` for seconds since local midnight.
pub fn clock(buffer: *Buffer, seconds: u32) []const u8 {
    const s = seconds % 86_400;
    return std.fmt.bufPrint(buffer, "{d:0>2}:{d:0>2}:{d:0>2}", .{ s / 3600, s % 3600 / 60, s % 60 }) catch buffer[0..0];
}

/// A tool call's duration: microseconds below a millisecond, then
/// milliseconds, then seconds, then minutes and seconds.
pub fn took(buffer: *Buffer, micros: u64) []const u8 {
    const shown = if (micros < 1_000)
        std.fmt.bufPrint(buffer, "{d} µs", .{micros})
    else if (micros < 100_000)
        std.fmt.bufPrint(buffer, "{d}.{d} ms", .{ micros / 1_000, micros % 1_000 / 100 })
    else if (micros < 1_000_000)
        std.fmt.bufPrint(buffer, "{d} ms", .{micros / 1_000})
    else if (micros < 60_000_000)
        std.fmt.bufPrint(buffer, "{d}.{d} s", .{ micros / 1_000_000, micros % 1_000_000 / 100_000 })
    else
        std.fmt.bufPrint(buffer, "{d} m {d} s", .{ micros / 60_000_000, micros % 60_000_000 / 1_000_000 });
    return shown catch buffer[0..0];
}

/// A span in whole seconds, as a run's end line says it.
pub fn span(buffer: *Buffer, seconds: u64) []const u8 {
    const shown = if (seconds < 60)
        std.fmt.bufPrint(buffer, "{d} s", .{seconds})
    else if (seconds < 3_600)
        std.fmt.bufPrint(buffer, "{d} m {d} s", .{ seconds / 60, seconds % 60 })
    else
        std.fmt.bufPrint(buffer, "{d} h {d} m", .{ seconds / 3_600, seconds % 3_600 / 60 });
    return shown catch buffer[0..0];
}

/// A count with a comma between each three figures: `1,521`.
pub fn count(buffer: *Buffer, n: u64) []const u8 {
    var digits: [20]u8 = undefined;
    const plain = std.fmt.bufPrint(&digits, "{d}", .{n}) catch return buffer[0..0];
    var at: usize = 0;
    for (plain, 0..) |digit, index| {
        if (index > 0 and (plain.len - index) % 3 == 0) {
            buffer[at] = ',';
            at += 1;
        }
        buffer[at] = digit;
        at += 1;
    }
    return buffer[0..at];
}

test "each unit keeps a duration to a few figures" {
    var b: Buffer = undefined;
    try std.testing.expectEqualStrings("842 µs", took(&b, 842));
    try std.testing.expectEqualStrings("2.6 ms", took(&b, 2_606));
    try std.testing.expectEqualStrings("312 ms", took(&b, 312_400));
    try std.testing.expectEqualStrings("8.2 s", took(&b, 8_240_000));
    try std.testing.expectEqualStrings("1 m 12 s", took(&b, 72_000_000));
}

test "a clock and a count read as a person writes them" {
    var b: Buffer = undefined;
    try std.testing.expectEqualStrings("01:24:31", clock(&b, 5071));
    try std.testing.expectEqualStrings("1,521", count(&b, 1521));
    try std.testing.expectEqualStrings("592", count(&b, 592));
}
