// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

const std = @import("std");

pub const Replay = union(enum) {
    none,
    path: []const u8,
    missing_value,
};

/// Pure argv scan for `--replay` / `--replay=`. Skips `args[0]`.
pub fn replayFlag(args: []const []const u8) Replay {
    var i: usize = 1;
    while (i < args.len) : (i += 1) {
        const arg = args[i];
        if (std.mem.eql(u8, arg, "--replay")) {
            if (i + 1 >= args.len) return .missing_value;
            return .{ .path = args[i + 1] };
        }
        if (std.mem.startsWith(u8, arg, "--replay=")) {
            return .{ .path = arg["--replay=".len..] };
        }
    }
    return .none;
}

test "replay flag reads the next argument" {
    const args = [_][]const u8{ "calyx", "--replay", "run.jsonl" };
    switch (replayFlag(&args)) {
        .path => |path| try std.testing.expectEqualStrings("run.jsonl", path),
        else => try std.testing.expect(false),
    }
}

test "replay flag reads an equals form" {
    const args = [_][]const u8{ "calyx", "--replay=trace.jsonl" };
    switch (replayFlag(&args)) {
        .path => |path| try std.testing.expectEqualStrings("trace.jsonl", path),
        else => try std.testing.expect(false),
    }
}

test "replay flag is absent" {
    const args = [_][]const u8{ "calyx", "--help" };
    try std.testing.expect(replayFlag(&args) == .none);
}

test "replay flag without a value is missing_value" {
    const args = [_][]const u8{ "calyx", "--replay" };
    try std.testing.expect(replayFlag(&args) == .missing_value);
}

test "seeded argv fuzz never traps" {
    var prng = std.Random.DefaultPrng.init(0xa267_f1a9);
    const random = prng.random();
    var scratch: [256]u8 = undefined;
    var round: usize = 0;
    while (round < 2048) : (round += 1) {
        var args_buf: [8][]const u8 = undefined;
        const argc = random.intRangeAtMost(usize, 1, args_buf.len);
        var i: usize = 0;
        while (i < argc) : (i += 1) {
            const n = random.intRangeAtMost(usize, 0, @min(32, scratch.len));
            random.bytes(scratch[0..n]);
            // Keep each arg a stable slice into a per-slot buffer.
            args_buf[i] = scratch[0..n];
        }
        // Re-materialize independent args so overlapping scratch is not an issue.
        var owned: [8][32]u8 = undefined;
        var slices: [8][]const u8 = undefined;
        i = 0;
        while (i < argc) : (i += 1) {
            const n = random.intRangeAtMost(usize, 0, 32);
            random.bytes(owned[i][0..n]);
            if (n >= 8 and random.boolean()) {
                @memcpy(owned[i][0..8], "--replay");
            }
            if (n >= 9 and random.boolean()) {
                @memcpy(owned[i][0..9], "--replay=");
            }
            slices[i] = owned[i][0..n];
        }
        const result = replayFlag(slices[0..argc]);
        switch (result) {
            .none, .missing_value, .path => {},
        }
    }
}
