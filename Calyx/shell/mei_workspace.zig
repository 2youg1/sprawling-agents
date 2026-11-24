// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

const std = @import("std");
const builtin = @import("builtin");

pub fn gitHead(gpa: std.mem.Allocator, io: std.Io, cwd: []const u8, path: []const u8) ?[]u8 {
    const executable = gitExecutable(gpa, io, path) orelse return null;
    defer gpa.free(executable);
    const symbolic = git(gpa, io, cwd, &.{ executable, "symbolic-ref", "--quiet", "--short", "HEAD" }) orelse return null;
    defer gpa.free(symbolic.stdout);
    defer gpa.free(symbolic.stderr);
    if (symbolic.term.success()) return label(gpa, symbolic.stdout);
    if (symbolic.term != .exited or symbolic.term.exited != 1) return null;
    const detached = git(gpa, io, cwd, &.{ executable, "rev-parse", "--short", "HEAD" }) orelse return null;
    defer gpa.free(detached.stdout);
    defer gpa.free(detached.stderr);
    return if (detached.term.success()) label(gpa, detached.stdout) else null;
}

fn gitExecutable(gpa: std.mem.Allocator, io: std.Io, path: []const u8) ?[]u8 {
    var directories = std.mem.tokenizeScalar(u8, path, std.fs.path.delimiter);
    while (directories.next()) |directory| {
        if (!std.fs.path.isAbsolute(directory)) continue;
        const name = if (builtin.os.tag == .windows) "git.exe" else "git";
        const candidate = std.fs.path.join(gpa, &.{ directory, name }) catch return null;
        std.Io.Dir.cwd().access(io, candidate, .{}) catch {
            gpa.free(candidate);
            continue;
        };
        return candidate;
    }
    return null;
}

fn git(gpa: std.mem.Allocator, io: std.Io, cwd: []const u8, argv: []const []const u8) ?std.process.RunResult {
    return std.process.run(gpa, io, .{
        .argv = argv,
        .cwd = .{ .path = cwd },
        .stderr_limit = .limited(4096),
        .stdout_limit = .limited(4096),
    }) catch null;
}

fn label(gpa: std.mem.Allocator, value: []const u8) ?[]u8 {
    const trimmed = std.mem.trimEnd(u8, value, "\r\n");
    if (trimmed.len == 0) return null;
    return gpa.dupe(u8, trimmed) catch null;
}
