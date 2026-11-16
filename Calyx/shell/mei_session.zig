// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

const std = @import("std");
const builtin = @import("builtin");

var next_session: std.atomic.Value(u64) = .init(1);

pub const Outcome = union(enum) {
    ok: void,
    err: []u8,
};

pub const CreateResult = union(enum) {
    ok: Session,
    err: []u8,
};

pub const Session = struct {
    gpa: std.mem.Allocator,
    io: std.Io,
    path: []u8,
    history: std.ArrayList([]u8) = .empty,
    resume_flag: bool = false,

    pub fn deinit(self: *Session) void {
        for (self.history.items) |line| self.gpa.free(line);
        self.history.deinit(self.gpa);
        self.gpa.free(self.path);
        self.* = undefined;
    }

    pub fn pathSlice(self: *const Session) []const u8 {
        return self.path;
    }

    pub fn historyLines(self: *const Session) []const []u8 {
        return self.history.items;
    }

    pub fn resumePath(self: *const Session) ?[]const u8 {
        return if (self.resume_flag) self.path else null;
    }

    pub fn createIn(gpa: std.mem.Allocator, io: std.Io, directory: []const u8, at_ms: u64) CreateResult {
        const counter = next_session.fetchAdd(1, .monotonic);
        const pid: u32 = switch (builtin.os.tag) {
            .linux => @intCast(std.os.linux.getpid()),
            .windows => 0,
            else => @intCast(std.c.getpid()),
        };
        const name = std.fmt.allocPrint(gpa, "mei-{d}-{d}-{d}.jsonl", .{ at_ms, pid, counter }) catch {
            return .{ .err = dupeOrStatic(gpa, "allocate session name") };
        };
        defer gpa.free(name);
        const path = std.fs.path.join(gpa, &.{ directory, name }) catch {
            return .{ .err = dupeOrStatic(gpa, "join session path") };
        };

        const file = std.Io.Dir.cwd().createFile(io, path, .{
            .exclusive = true,
            .permissions = .fromMode(0o600),
        }) catch |err| {
            const diag = formatFailure(gpa, "create a new Mei session", @errorName(err), path) catch dupeOrStatic(gpa, "create a new Mei session");
            gpa.free(path);
            return .{ .err = diag };
        };
        file.close(io);

        return .{ .ok = .{
            .gpa = gpa,
            .io = io,
            .path = path,
        } };
    }

    pub fn import(gpa: std.mem.Allocator, io: std.Io, path_in: []const u8) CreateResult {
        const path = gpa.dupe(u8, path_in) catch {
            return .{ .err = dupeOrStatic(gpa, "duplicate resume path") };
        };

        const text = std.Io.Dir.cwd().readFileAlloc(io, path, gpa, .limited(64 * 1024 * 1024)) catch |err| {
            const diag = formatFailure(gpa, "read the Mei resume log", @errorName(err), path) catch dupeOrStatic(gpa, "read the Mei resume log");
            gpa.free(path);
            return .{ .err = diag };
        };
        defer gpa.free(text);

        var history: std.ArrayList([]u8) = .empty;
        var it = std.mem.splitScalar(u8, text, '\n');
        while (it.next()) |line| {
            if (line.len == 0 and it.peek() == null) break;
            const owned = gpa.dupe(u8, line) catch {
                freeHistory(gpa, &history);
                gpa.free(path);
                return .{ .err = dupeOrStatic(gpa, "duplicate history line") };
            };
            history.append(gpa, owned) catch {
                gpa.free(owned);
                freeHistory(gpa, &history);
                gpa.free(path);
                return .{ .err = dupeOrStatic(gpa, "store history line") };
            };
        }

        return .{ .ok = .{
            .gpa = gpa,
            .io = io,
            .path = path,
            .history = history,
            .resume_flag = true,
        } };
    }

    pub fn append(self: *Session, lines: []const []const u8) Outcome {
        if (lines.len == 0) return .ok;
        const file = std.Io.Dir.cwd().openFile(self.io, self.path, .{ .mode = .read_write }) catch |err| {
            return .{ .err = formatFailure(self.gpa, "open the current Mei session for append", @errorName(err), self.path) catch dupeOrStatic(self.gpa, "open session") };
        };
        defer file.close(self.io);

        appendContents(self.io, file, lines) catch {
            const failure = last_append_fail;
            if (failure.rollback) |roll| {
                const subject = std.fmt.allocPrint(self.gpa, "{s}; restoring the original log also failed: {s}", .{
                    @errorName(failure.write_err),
                    @errorName(roll),
                }) catch null;
                defer if (subject) |s| self.gpa.free(s);
                return .{ .err = formatFailureWithRecovery(
                    self.gpa,
                    "roll back a failed Mei segment append",
                    subject orelse @errorName(failure.write_err),
                    self.path,
                    "keep the damaged file for manual recovery, use /new for further work, and resume only a complete backup",
                ) catch dupeOrStatic(self.gpa, "rollback failed") };
            }
            return .{ .err = formatFailure(self.gpa, "append a successful Mei segment", @errorName(failure.write_err), self.path) catch dupeOrStatic(self.gpa, "append failed") };
        };

        for (lines) |line| {
            const owned = self.gpa.dupe(u8, line) catch {
                return .{ .err = dupeOrStatic(self.gpa, "duplicate appended line") };
            };
            self.history.append(self.gpa, owned) catch {
                self.gpa.free(owned);
                return .{ .err = dupeOrStatic(self.gpa, "store appended line") };
            };
        }
        self.resume_flag = true;
        return .ok;
    }
};

const AppendFail = struct {
    write_err: anyerror,
    rollback: ?anyerror,
};

threadlocal var last_append_fail: AppendFail = .{ .write_err = error.OutOfMemory, .rollback = null };

fn appendContents(io: std.Io, file: std.Io.File, lines: []const []const u8) error{AppendFailed}!void {
    const original_len = file.length(io) catch |err| {
        last_append_fail = .{ .write_err = err, .rollback = null };
        return error.AppendFailed;
    };

    writeSegment(io, file, original_len, lines) catch |err| {
        const rollback = file.setLength(io, original_len);
        last_append_fail = .{
            .write_err = err,
            .rollback = if (rollback) |_| null else |roll| roll,
        };
        return error.AppendFailed;
    };
}

fn writeSegment(io: std.Io, file: std.Io.File, original_len: u64, lines: []const []const u8) !void {
    var end = original_len;
    if (original_len > 0) {
        var last: [1]u8 = undefined;
        const n = try file.readPositionalAll(io, &last, original_len - 1);
        if (n == 1 and last[0] != '\n') {
            try file.writeStreamingAll(io, "\n");
            end += 1;
        }
    }
    var buf: [512]u8 = undefined;
    var writer = file.writer(io, &buf);
    try writer.seekTo(end);
    for (lines) |line| {
        try writer.interface.writeAll(line);
        try writer.interface.writeAll("\n");
    }
    try writer.interface.flush();
}

fn freeHistory(gpa: std.mem.Allocator, history: *std.ArrayList([]u8)) void {
    for (history.items) |line| gpa.free(line);
    history.deinit(gpa);
}

fn dupeOrStatic(gpa: std.mem.Allocator, text: []const u8) []u8 {
    return gpa.dupe(u8, text) catch @panic("out of memory for Mei session diagnostic");
}

fn formatFailure(gpa: std.mem.Allocator, action: []const u8, subject: []const u8, path: []const u8) ![]u8 {
    return formatFailureWithRecovery(
        gpa,
        action,
        subject,
        path,
        "use /resume <jsonl> with a complete log, or /new to create a different file",
    );
}

fn formatFailureWithRecovery(
    gpa: std.mem.Allocator,
    action: []const u8,
    subject: []const u8,
    path: []const u8,
    recovery: []const u8,
) ![]u8 {
    return try std.fmt.allocPrint(gpa,
        \\```toml
        \\code = "E_MEI_SESSION"
        \\description = "{s}: {s}"
        \\cause = "a Mei session is one explicitly named, complete JSONL message log\npath = {s}\nvalue = {s}"
        \\resolution = "{s}"
        \\```
    , .{ action, subject, path, path, recovery });
}

test "create import append round-trip" {
    const io = std.testing.io;
    const gpa = std.testing.allocator;
    var tmp = std.testing.tmpDir(.{});
    defer tmp.cleanup();

    const dir_rel = try std.fmt.allocPrint(gpa, ".zig-cache/tmp/{s}", .{tmp.sub_path});
    defer gpa.free(dir_rel);

    var session = switch (Session.createIn(gpa, io, dir_rel, 100)) {
        .ok => |s| s,
        .err => |e| {
            defer freeDiag(gpa, e);
            return error.TestUnexpectedResult;
        },
    };
    defer session.deinit();

    switch (session.append(&.{
        "{\"role\":\"user\",\"content\":\"first\"}",
        "{\"role\":\"assistant\",\"content\":\"done\",\"stop_reason\":\"end_turn\"}",
    })) {
        .ok => {},
        .err => |e| {
            defer freeDiag(gpa, e);
            return error.TestUnexpectedResult;
        },
    }
    const first_path = try gpa.dupe(u8, session.pathSlice());
    defer gpa.free(first_path);

    var second = switch (Session.createIn(gpa, io, dir_rel, 100)) {
        .ok => |s| s,
        .err => |e| {
            defer freeDiag(gpa, e);
            return error.TestUnexpectedResult;
        },
    };
    defer second.deinit();
    try std.testing.expect(!std.mem.eql(u8, first_path, second.pathSlice()));

    var resumed = switch (Session.import(gpa, io, first_path)) {
        .ok => |s| s,
        .err => |e| {
            defer freeDiag(gpa, e);
            return error.TestUnexpectedResult;
        },
    };
    defer resumed.deinit();
    try std.testing.expectEqual(@as(usize, 2), resumed.historyLines().len);
    try std.testing.expect(resumed.resumePath() != null);
    switch (resumed.append(&.{"{\"role\":\"user\",\"content\":\"next\"}"})) {
        .ok => {},
        .err => |e| {
            defer freeDiag(gpa, e);
            return error.TestUnexpectedResult;
        },
    }
    try std.testing.expectEqual(@as(usize, 3), resumed.historyLines().len);
}

test "session append stress keeps length and frees cleanly" {
    const io = std.testing.io;
    var gpa_state: std.heap.DebugAllocator(.{}) = .init;
    defer std.testing.expect(gpa_state.deinit() == .ok) catch @panic("session GPA leak");
    const gpa = gpa_state.allocator();
    var tmp = std.testing.tmpDir(.{});
    defer tmp.cleanup();
    const dir_rel = try std.fmt.allocPrint(gpa, ".zig-cache/tmp/{s}", .{tmp.sub_path});
    defer gpa.free(dir_rel);
    var session = switch (Session.createIn(gpa, io, dir_rel, 42)) {
        .ok => |s| s,
        .err => |e| {
            defer freeDiag(gpa, e);
            return error.TestUnexpectedResult;
        },
    };
    defer session.deinit();
    var round: usize = 0;
    while (round < 128) : (round += 1) {
        var line_buf: [64]u8 = undefined;
        const line = try std.fmt.bufPrint(
            &line_buf,
            "{{\"role\":\"user\",\"content\":\"n-{d}\"}}",
            .{round},
        );
        switch (session.append(&.{line})) {
            .ok => {},
            .err => |e| {
                defer freeDiag(gpa, e);
                return error.TestUnexpectedResult;
            },
        }
    }
    try std.testing.expectEqual(@as(usize, 128), session.historyLines().len);
}

test "append after missing final newline stays parseable" {
    const io = std.testing.io;
    const gpa = std.testing.allocator;
    var tmp = std.testing.tmpDir(.{});
    defer tmp.cleanup();

    try tmp.dir.writeFile(io, .{
        .sub_path = "session.jsonl",
        .data = "{\"role\":\"user\",\"content\":\"first\"}\n{\"role\":\"assistant\",\"content\":\"done\",\"stop_reason\":\"end_turn\"}",
    });
    const full = try std.fmt.allocPrint(gpa, ".zig-cache/tmp/{s}/session.jsonl", .{tmp.sub_path});
    defer gpa.free(full);

    var session = switch (Session.import(gpa, io, full)) {
        .ok => |s| s,
        .err => |e| {
            defer freeDiag(gpa, e);
            return error.TestUnexpectedResult;
        },
    };
    defer session.deinit();
    switch (session.append(&.{
        "{\"role\":\"user\",\"content\":\"next\"}",
        "{\"role\":\"assistant\",\"content\":\"again\",\"stop_reason\":\"end_turn\"}",
    })) {
        .ok => {},
        .err => |e| {
            defer freeDiag(gpa, e);
            return error.TestUnexpectedResult;
        },
    }
    try std.testing.expectEqual(@as(usize, 4), session.historyLines().len);
}

fn freeDiag(gpa: std.mem.Allocator, text: []u8) void {
    gpa.free(text);
}
