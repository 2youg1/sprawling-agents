// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

const std = @import("std");
const builtin = @import("builtin");

pub const Error = error{BufferTooSmall};

/// Resolve the headless Mei program into a caller-owned buffer.
/// Order: non-empty `mei_bin`, else `sibling` when `sibling_exists`, else PATH name `mei`.
pub fn resolve(
    mei_bin: ?[]const u8,
    sibling: ?[]const u8,
    sibling_exists: bool,
    into: []u8,
) Error![]u8 {
    if (mei_bin) |bin| {
        if (bin.len > 0) return write(into, bin);
    }
    if (sibling_exists) {
        if (sibling) |path| return write(into, path);
    }
    return write(into, "mei");
}

pub fn siblingName() []const u8 {
    return if (builtin.os.tag == .windows) "mei.exe" else "mei";
}

fn write(into: []u8, value: []const u8) Error![]u8 {
    if (into.len < value.len) return error.BufferTooSmall;
    @memcpy(into[0..value.len], value);
    return into[0..value.len];
}

test "MEI_BIN wins" {
    var buf: [64]u8 = undefined;
    const got = try resolve("/opt/mei", "/beside/mei", true, &buf);
    try std.testing.expectEqualStrings("/opt/mei", got);
}

test "sibling wins when MEI_BIN is absent" {
    var buf: [64]u8 = undefined;
    const got = try resolve(null, "/beside/mei", true, &buf);
    try std.testing.expectEqualStrings("/beside/mei", got);
}

test "PATH name when neither is usable" {
    var buf: [16]u8 = undefined;
    const got = try resolve(null, "/beside/mei", false, &buf);
    try std.testing.expectEqualStrings("mei", got);
}

test "empty MEI_BIN falls through" {
    var buf: [64]u8 = undefined;
    const got = try resolve("", "/beside/mei", true, &buf);
    try std.testing.expectEqualStrings("/beside/mei", got);
}

test "buffer too small is refused" {
    var buf: [2]u8 = undefined;
    try std.testing.expectError(error.BufferTooSmall, resolve("/opt/mei", null, false, &buf));
}
