// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

const std = @import("std");
const builtin = @import("builtin");

const GitResult = struct { term: std.process.Child.Term, stdout: []u8 };
const output_limit = 4096;

pub fn gitHead(gpa: std.mem.Allocator, io: std.Io, cwd: []const u8, path: []const u8) ?[]u8 {
    const executable = gitExecutable(gpa, io, path) orelse return null;
    defer gpa.free(executable);
    const deadline: std.Io.Clock.Timestamp = .fromNow(io, .{ .raw = .fromMilliseconds(500), .clock = .awake });
    const symbolic = git(gpa, io, cwd, &.{ executable, "symbolic-ref", "--quiet", "--short", "HEAD" }, deadline) orelse return null;
    defer gpa.free(symbolic.stdout);
    if (symbolic.term.success()) return label(gpa, symbolic.stdout);
    if (symbolic.term != .exited or symbolic.term.exited != 1) return null;
    const detached = git(gpa, io, cwd, &.{ executable, "rev-parse", "--short", "HEAD" }, deadline) orelse return null;
    defer gpa.free(detached.stdout);
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

fn git(gpa: std.mem.Allocator, io: std.Io, cwd: []const u8, argv: []const []const u8, deadline: std.Io.Clock.Timestamp) ?GitResult {
    if (builtin.os.tag == .windows) return gitWindows(gpa, io, cwd, argv, deadline);
    const result = std.process.run(gpa, io, .{
        .argv = argv,
        .cwd = .{ .path = cwd },
        .stderr_limit = .limited(output_limit),
        .stdout_limit = .limited(output_limit),
        .timeout = .{ .deadline = deadline },
    }) catch return null;
    gpa.free(result.stderr);
    return .{ .term = result.term, .stdout = result.stdout };
}

fn gitWindows(gpa: std.mem.Allocator, io: std.Io, cwd: []const u8, argv: []const []const u8, deadline: std.Io.Clock.Timestamp) ?GitResult {
    var child = std.process.spawn(io, .{
        .argv = argv,
        .cwd = .{ .path = cwd },
        .stdin = .ignore,
        .stdout = .pipe,
        .stderr = .ignore,
        .create_no_window = true,
    }) catch return null;
    defer child.kill(io);

    const windows = std.os.windows;
    const millis = @max(0, deadline.durationFromNow(io).raw.toMilliseconds());
    if (WaitForSingleObject(child.id.?, @intCast(millis)) != 0) return null;
    var bytes: [output_limit]u8 = undefined;
    var read_count: windows.DWORD = 0;
    var available: windows.DWORD = 0;
    const ok = PeekNamedPipe(child.stdout.?.handle, &bytes, bytes.len, &read_count, &available, null);
    if (available > bytes.len) return null;
    const term = child.wait(io) catch return null;
    const count = if (ok != 0) read_count else 0;
    return .{ .term = term, .stdout = gpa.dupe(u8, bytes[0..count]) catch return null };
}

extern "kernel32" fn WaitForSingleObject(std.os.windows.HANDLE, std.os.windows.DWORD) callconv(.winapi) std.os.windows.DWORD;
extern "kernel32" fn PeekNamedPipe(std.os.windows.HANDLE, ?*anyopaque, std.os.windows.DWORD, ?*std.os.windows.DWORD, ?*std.os.windows.DWORD, ?*std.os.windows.DWORD) callconv(.winapi) c_int;

fn label(gpa: std.mem.Allocator, value: []const u8) ?[]u8 {
    const trimmed = std.mem.trimEnd(u8, value, "\r\n");
    if (trimmed.len == 0) return null;
    return gpa.dupe(u8, trimmed) catch null;
}
