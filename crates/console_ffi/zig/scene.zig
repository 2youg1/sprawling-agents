// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// Reading a scene: the bytes `console_ffi::scene` writes, little-endian,
// every string a u32 length and its bytes. Every read is bounded by the
// scene's own length, so a short or malformed scene is `error.Malformed`
// and never a read past the buffer Rust lent.

const std = @import("std");
const part = @import("part.zig");

pub const Error = error{Malformed};

/// The time of day a line carries, or none: `u32` seconds since local
/// midnight, with `none` for a line that has no moment.
pub const no_time: u32 = std.math.maxInt(u32);

/// A count or duration that may be absent.
pub const no_count: u64 = std.math.maxInt(u64);

pub const Reader = struct {
    bytes: []const u8,
    at: usize = 0,

    pub fn done(self: *const Reader) bool {
        return self.at >= self.bytes.len;
    }

    fn take(self: *Reader, n: usize) Error![]const u8 {
        const end = std.math.add(usize, self.at, n) catch return error.Malformed;
        if (end > self.bytes.len) return error.Malformed;
        const slice = self.bytes[self.at..end];
        self.at = end;
        return slice;
    }

    pub fn byte(self: *Reader) Error!u8 {
        return (try self.take(1))[0];
    }

    pub fn short(self: *Reader) Error!u16 {
        return std.mem.readInt(u16, (try self.take(2))[0..2], .little);
    }

    pub fn word(self: *Reader) Error!u32 {
        return std.mem.readInt(u32, (try self.take(4))[0..4], .little);
    }

    pub fn long(self: *Reader) Error!u64 {
        return std.mem.readInt(u64, (try self.take(8))[0..8], .little);
    }

    pub fn text(self: *Reader) Error![]const u8 {
        const n = try self.word();
        return self.take(n);
    }

    pub fn tag(self: *Reader) Error!part.Part {
        return std.enums.fromInt(part.Part, try self.byte()) orelse error.Malformed;
    }

    pub fn outcome(self: *Reader) Error!part.Outcome {
        return std.enums.fromInt(part.Outcome, try self.byte()) orelse error.Malformed;
    }

    pub fn ending(self: *Reader) Error!part.Ending {
        return std.enums.fromInt(part.Ending, try self.byte()) orelse error.Malformed;
    }

    pub fn verdict(self: *Reader) Error!part.Verdict {
        return std.enums.fromInt(part.Verdict, try self.byte()) orelse error.Malformed;
    }
};

test "a short scene is malformed, never a read past its end" {
    var reader: Reader = .{ .bytes = &.{ 5, 0, 0, 0, 'a', 'b' } };
    try std.testing.expectError(error.Malformed, reader.text());
}

test "an unknown tag is malformed" {
    var reader: Reader = .{ .bytes = &.{200} };
    try std.testing.expectError(error.Malformed, reader.tag());
}
