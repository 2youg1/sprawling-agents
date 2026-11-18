// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

const std = @import("std");
const builtin = @import("builtin");
const mei_line = @import("mei_line.zig");

var next_session: std.atomic.Value(u64) = .init(1);
var next_run: std.atomic.Value(u64) = .init(1);

/// What one append left on disk.
///
/// `err` means the segment was not written and the original length was
/// restored, so nothing refers to the run's captures. `uncertain` means the
/// write failed and the rollback did too: the log may already name those files,
/// so the caller must keep them.
pub const Diagnostic = struct {
    text: []const u8,
    owned: bool,

    pub fn deinit(self: Diagnostic, gpa: std.mem.Allocator) void {
        if (self.owned) gpa.free(self.text);
    }
};

pub const Outcome = union(enum) {
    ok: void,
    err: Diagnostic,
    uncertain: Diagnostic,
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
    log_len: u64 = 0,

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

    pub fn captureDirectory(self: *Session, gpa: std.mem.Allocator, io: std.Io, at_ms: u64) ![]u8 {
        const parent = try std.fmt.allocPrint(gpa, "{s}.captures", .{self.path});
        defer gpa.free(parent);
        try std.Io.Dir.cwd().createDirPath(io, parent);
        while (true) {
            const run = next_run.fetchAdd(1, .monotonic);
            const path = try std.fmt.allocPrint(gpa, "{s}/run-{d}-{d}-{d}", .{ parent, at_ms, processId(), run });
            std.Io.Dir.cwd().createDir(io, path, .default_dir) catch |err| {
                gpa.free(path);
                if (err == error.PathAlreadyExists) continue;
                return err;
            };
            return path;
        }
    }

    fn processId() u32 {
        return switch (builtin.os.tag) {
            .linux => @intCast(std.os.linux.getpid()),
            .windows => @intCast(std.os.windows.GetCurrentProcessId()),
            else => @intCast(std.c.getpid()),
        };
    }

    pub fn createIn(gpa: std.mem.Allocator, io: std.Io, directory: []const u8, at_ms: u64) CreateResult {
        const counter = next_session.fetchAdd(1, .monotonic);
        const name = std.fmt.allocPrint(gpa, "mei-{d}-{d}-{d}.jsonl", .{ at_ms, processId(), counter }) catch {
            return .{ .err = dupeOrStatic(gpa, "allocate session name") };
        };
        defer gpa.free(name);
        const path = std.fs.path.join(gpa, &.{ directory, name }) catch {
            return .{ .err = dupeOrStatic(gpa, "join session path") };
        };

        const file = std.Io.Dir.cwd().createFile(io, path, .{
            .exclusive = true,
            // POSIX: owner read/write only. Windows: default attributes (no Unix mode).
            .permissions = if (comptime builtin.os.tag == .windows)
                .default_file
            else
                .fromMode(0o600),
        }) catch |err| {
            const diag = formatFailure(gpa, "create a new Mei session", @errorName(err), path) catch dupeOrStatic(gpa, "create a new Mei session");
            gpa.free(path);
            return .{ .err = diag };
        };
        if (comptime builtin.os.tag != .windows) {
            file.setPermissions(io, .fromMode(0o600)) catch {
                file.close(io);
                std.Io.Dir.cwd().deleteFile(io, path) catch {};
                const diag = dupeOrStatic(gpa, "restrict session file permissions");
                gpa.free(path);
                return .{ .err = diag };
            };
        }
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

        var file = std.Io.Dir.cwd().openFile(io, path, .{}) catch |err| {
            const diag = formatFailure(gpa, "read the Mei resume log", @errorName(err), path) catch dupeOrStatic(gpa, "read the Mei resume log");
            gpa.free(path);
            return .{ .err = diag };
        };
        defer file.close(io);
        const log_len = file.length(io) catch |err| {
            const diag = formatFailure(gpa, "measure the Mei resume log", @errorName(err), path) catch dupeOrStatic(gpa, "measure the Mei resume log");
            gpa.free(path);
            return .{ .err = diag };
        };

        var history: std.ArrayList([]u8) = .empty;
        var buf: [8192]u8 = undefined;
        var reader = file.readerStreaming(io, &buf);
        while (true) {
            const line = (mei_line.read(gpa, &reader.interface, mei_line.MAX_BYTES) catch |err| {
                freeHistory(gpa, &history);
                const diag = switch (err) {
                    error.MessageLineTooLong => mei_line.tooLong(gpa, mei_line.MAX_BYTES, path) catch dupeOrStatic(gpa, "read one Mei message line"),
                    else => formatFailure(gpa, "read the Mei resume log", @errorName(err), path) catch dupeOrStatic(gpa, "read the Mei resume log"),
                };
                gpa.free(path);
                return .{ .err = diag };
            }) orelse break;
            history.append(gpa, line) catch {
                gpa.free(line);
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
            .log_len = log_len,
        } };
    }

    pub fn append(self: *Session, lines: []const []const u8) Outcome {
        if (lines.len == 0) return .ok;

        const original_len = self.history.items.len;
        self.history.ensureUnusedCapacity(self.gpa, lines.len) catch {
            return .{ .err = outOfMemory() };
        };
        for (lines) |line| {
            const owned = self.gpa.dupe(u8, line) catch {
                shrinkHistory(self.gpa, &self.history, original_len);
                return .{ .err = outOfMemory() };
            };
            self.history.appendAssumeCapacity(owned);
        }

        const file = std.Io.Dir.cwd().openFile(self.io, self.path, .{ .mode = .read_write }) catch |err| {
            shrinkHistory(self.gpa, &self.history, original_len);
            const diag = formatFailure(self.gpa, "open the current Mei session for append", @errorName(err), self.path) catch return .{ .err = outOfMemory() };
            return .{ .err = ownedDiagnostic(diag) };
        };
        defer file.close(self.io);
        file.lock(self.io, .exclusive) catch |err| {
            shrinkHistory(self.gpa, &self.history, original_len);
            const diag = formatFailure(self.gpa, "lock the current Mei session for append", @errorName(err), self.path) catch return .{ .err = outOfMemory() };
            return .{ .err = ownedDiagnostic(diag) };
        };
        defer file.unlock(self.io);

        switch (appendContents(self.io, file, lines, self.log_len)) {
            .ok => |new_len| self.log_len = new_len,
            .stale => {
                shrinkHistory(self.gpa, &self.history, original_len);
                const diag = formatFailureWithRecovery(
                    self.gpa,
                    "append to a stale Mei session",
                    "another process committed to this log after it was resumed",
                    self.path,
                    "use /resume <jsonl> on the latest complete log, then submit this run again",
                ) catch return .{ .err = outOfMemory() };
                return .{ .err = ownedDiagnostic(diag) };
            },
            .failed => |failure| {
                shrinkHistory(self.gpa, &self.history, original_len);
                if (failure.rollback) |roll| {
                    const subject = std.fmt.allocPrint(self.gpa, "{s}; restoring the original log also failed: {s}", .{
                        @errorName(failure.write),
                        @errorName(roll),
                    }) catch null;
                    defer if (subject) |owned| self.gpa.free(owned);
                    const diag = formatFailureWithRecovery(
                        self.gpa,
                        "roll back a failed Mei segment append",
                        subject orelse @errorName(failure.write),
                        self.path,
                        "keep the log and the run's captured output for manual recovery; use /new for further work and resume only a complete backup",
                    ) catch return .{ .uncertain = uncertainOutOfMemory() };
                    return .{ .uncertain = ownedDiagnostic(diag) };
                }
                const diag = formatFailure(self.gpa, "append a successful Mei segment", @errorName(failure.write), self.path) catch return .{ .err = outOfMemory() };
                return .{ .err = ownedDiagnostic(diag) };
            },
        }
        self.resume_flag = true;
        return .ok;
    }
};

fn shrinkHistory(gpa: std.mem.Allocator, history: *std.ArrayList([]u8), keep: usize) void {
    while (history.items.len > keep) {
        const line = history.pop().?;
        gpa.free(line);
    }
}

const AppendFailure = struct {
    write: anyerror,
    rollback: ?anyerror,
};

const AppendResult = union(enum) {
    ok: u64,
    stale: void,
    failed: AppendFailure,
};

fn appendContents(io: std.Io, file: std.Io.File, lines: []const []const u8, expected_len: u64) AppendResult {
    const original_len = file.length(io) catch |err| {
        return .{ .failed = .{ .write = err, .rollback = null } };
    };
    if (original_len != expected_len) return .stale;

    const new_len = writeSegment(io, file, original_len, lines) catch |err| {
        file.setLength(io, original_len) catch |roll| {
            return .{ .failed = .{ .write = err, .rollback = roll } };
        };
        return .{ .failed = .{ .write = err, .rollback = null } };
    };
    return .{ .ok = new_len };
}

fn writeSegment(io: std.Io, file: std.Io.File, original_len: u64, lines: []const []const u8) !u64 {
    var end = original_len;
    if (original_len > 0) {
        var last: [1]u8 = undefined;
        const n = try file.readPositionalAll(io, &last, original_len - 1);
        if (n == 1 and last[0] != '\n') {
            try file.writePositionalAll(io, "\n", original_len);
            end += 1;
        }
    }
    var buf: [512]u8 = undefined;
    var writer = file.writer(io, &buf);
    try writer.seekTo(end);
    for (lines) |line| {
        try writer.interface.writeAll(line);
        try writer.interface.writeAll("\n");
        end += line.len + 1;
    }
    try writer.interface.flush();
    return end;
}

fn freeHistory(gpa: std.mem.Allocator, history: *std.ArrayList([]u8)) void {
    for (history.items) |line| gpa.free(line);
    history.deinit(gpa);
}

fn outOfMemory() Diagnostic {
    return .{ .text =
    \\```toml
    \\code = "E_MEI_SESSION"
    \\description = "remember a Mei session append: the allocator is exhausted"
    \\cause = "no history or diagnostic allocation succeeded; the log was not changed"
    \\resolution = "close other processes to free memory, then resume the complete log and retry"
    \\```
    , .owned = false };
}

fn uncertainOutOfMemory() Diagnostic {
    return .{ .text =
    \\```toml
    \\code = "E_MEI_SESSION"
    \\description = "report a Mei session rollback failure: the allocator is exhausted"
    \\cause = "the append failed, rollback failed, and the log may already name the run's captures"
    \\resolution = "keep the log and captured output for manual recovery; free memory, then resume only a complete backup"
    \\```
    , .owned = false };
}

fn ownedDiagnostic(text: []u8) Diagnostic {
    return .{ .text = text, .owned = true };
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

test "createIn session file is owner-only on POSIX" {
    if (comptime builtin.os.tag == .windows) return;
    const io = std.testing.io;
    const gpa = std.testing.allocator;
    var tmp = std.testing.tmpDir(.{});
    defer tmp.cleanup();
    const dir_rel = try std.fmt.allocPrint(gpa, ".zig-cache/tmp/{s}", .{tmp.sub_path});
    defer gpa.free(dir_rel);
    var session = switch (Session.createIn(gpa, io, dir_rel, 7)) {
        .ok => |s| s,
        .err => |e| {
            defer freeDiag(gpa, e);
            return error.TestUnexpectedResult;
        },
    };
    defer session.deinit();
    const st = try std.Io.Dir.cwd().statFile(io, session.pathSlice(), .{});
    try std.testing.expect((st.permissions.toMode() & 0o077) == 0);
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
        .err, .uncertain => |e| {
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
        .err, .uncertain => |e| {
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
            .err, .uncertain => |e| {
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
        .err, .uncertain => |e| {
            defer freeDiag(gpa, e);
            return error.TestUnexpectedResult;
        },
    }
    try std.testing.expectEqual(@as(usize, 4), session.historyLines().len);
    const contents = try std.Io.Dir.cwd().readFileAlloc(io, full, gpa, .limited(4096));
    defer gpa.free(contents);
    try std.testing.expectEqualStrings(
        "{\"role\":\"user\",\"content\":\"first\"}\n{\"role\":\"assistant\",\"content\":\"done\",\"stop_reason\":\"end_turn\"}\n{\"role\":\"user\",\"content\":\"next\"}\n{\"role\":\"assistant\",\"content\":\"again\",\"stop_reason\":\"end_turn\"}\n",
        contents,
    );
}

test "a stale session cannot append over another instance's commit" {
    const io = std.testing.io;
    const gpa = std.testing.allocator;
    var tmp = std.testing.tmpDir(.{});
    defer tmp.cleanup();
    const root = try std.fmt.allocPrint(gpa, ".zig-cache/tmp/{s}", .{tmp.sub_path});
    defer gpa.free(root);
    const path = try std.fs.path.join(gpa, &.{ root, "session.jsonl" });
    defer gpa.free(path);
    try std.Io.Dir.cwd().writeFile(io, .{ .sub_path = path, .data = "{\"role\":\"user\",\"content\":\"first\"}\n" });
    var first = switch (Session.import(gpa, io, path)) {
        .ok => |s| s,
        .err => |e| {
            defer freeDiag(gpa, e);
            return error.TestUnexpectedResult;
        },
    };
    defer first.deinit();
    var second = switch (Session.import(gpa, io, path)) {
        .ok => |s| s,
        .err => |e| {
            defer freeDiag(gpa, e);
            return error.TestUnexpectedResult;
        },
    };
    defer second.deinit();
    switch (first.append(&.{"{\"role\":\"assistant\",\"content\":\"first commit\"}"})) {
        .ok => {},
        .err, .uncertain => |e| {
            defer freeDiag(gpa, e);
            return error.TestUnexpectedResult;
        },
    }
    const committed = try std.Io.Dir.cwd().readFileAlloc(io, path, gpa, .limited(4096));
    defer gpa.free(committed);
    switch (second.append(&.{"{\"role\":\"assistant\",\"content\":\"stale commit\"}"})) {
        .ok => return error.TestUnexpectedResult,
        .err => |e| {
            defer freeDiag(gpa, e);
            try std.testing.expect(std.mem.indexOf(u8, e.text, "stale Mei session") != null);
        },
        .uncertain => |e| {
            defer freeDiag(gpa, e);
            return error.TestUnexpectedResult;
        },
    }
    const after = try std.Io.Dir.cwd().readFileAlloc(io, path, gpa, .limited(4096));
    defer gpa.free(after);
    try std.testing.expectEqualStrings(committed, after);
}

test "capture directories are unique across instances of one session path" {
    const io = std.testing.io;
    const gpa = std.testing.allocator;
    var tmp = std.testing.tmpDir(.{});
    defer tmp.cleanup();
    const root = try std.fmt.allocPrint(gpa, ".zig-cache/tmp/{s}", .{tmp.sub_path});
    defer gpa.free(root);
    const path = try std.fs.path.join(gpa, &.{ root, "session.jsonl" });
    defer gpa.free(path);
    try std.Io.Dir.cwd().writeFile(io, .{ .sub_path = path, .data = "log\n" });
    var first = switch (Session.import(gpa, io, path)) {
        .ok => |s| s,
        .err => |e| {
            defer freeDiag(gpa, e);
            return error.TestUnexpectedResult;
        },
    };
    defer first.deinit();
    var second = switch (Session.import(gpa, io, path)) {
        .ok => |s| s,
        .err => |e| {
            defer freeDiag(gpa, e);
            return error.TestUnexpectedResult;
        },
    };
    defer second.deinit();
    const first_capture = try first.captureDirectory(gpa, io, 100);
    defer gpa.free(first_capture);
    const second_capture = try second.captureDirectory(gpa, io, 100);
    defer gpa.free(second_capture);
    try std.testing.expect(!std.mem.eql(u8, first_capture, second_capture));
    try std.Io.Dir.cwd().access(io, first_capture, .{});
    try std.Io.Dir.cwd().access(io, second_capture, .{});

    const collision = try std.fmt.allocPrint(gpa, "{s}.captures/run-100-{d}-{d}", .{ path, Session.processId(), next_run.load(.monotonic) });
    defer gpa.free(collision);
    try std.Io.Dir.cwd().createDirPath(io, collision);
    const marker = try std.fs.path.join(gpa, &.{ collision, "keep" });
    defer gpa.free(marker);
    try std.Io.Dir.cwd().writeFile(io, .{ .sub_path = marker, .data = "keep" });
    const replacement = try first.captureDirectory(gpa, io, 100);
    defer gpa.free(replacement);
    try std.testing.expect(!std.mem.eql(u8, collision, replacement));
    const kept = try std.Io.Dir.cwd().readFileAlloc(io, marker, gpa, .limited(5));
    defer gpa.free(kept);
    try std.testing.expectEqualStrings("keep", kept);
}

test "a fully exhausted allocator still returns an append error" {
    const io = std.testing.io;
    const gpa = std.testing.allocator;
    var tmp = std.testing.tmpDir(.{});
    defer tmp.cleanup();
    const root = try std.fmt.allocPrint(gpa, ".zig-cache/tmp/{s}", .{tmp.sub_path});
    defer gpa.free(root);
    const path = try std.fs.path.join(gpa, &.{ root, "session.jsonl" });
    defer gpa.free(path);
    try std.Io.Dir.cwd().writeFile(io, .{ .sub_path = path, .data = "log\n" });
    var storage: [0]u8 = .{};
    var fixed: std.heap.FixedBufferAllocator = .init(&storage);
    var session: Session = .{ .gpa = fixed.allocator(), .io = io, .path = @constCast(path) };
    switch (session.append(&.{"line"})) {
        .ok => return error.TestUnexpectedResult,
        .err => |e| {
            defer freeDiag(fixed.allocator(), e);
            try std.testing.expect(std.mem.indexOf(u8, e.text, "E_MEI_SESSION") != null);
        },
        .uncertain => |e| {
            defer freeDiag(fixed.allocator(), e);
            return error.TestUnexpectedResult;
        },
    }
    const contents = try std.Io.Dir.cwd().readFileAlloc(io, path, gpa, .limited(4096));
    defer gpa.free(contents);
    try std.testing.expectEqualStrings("log\n", contents);
}

test "an append allocation failure cannot leave unreported lines in the log" {
    const io = std.testing.io;
    const gpa = std.testing.allocator;
    var tmp = std.testing.tmpDir(.{});
    defer tmp.cleanup();
    const root = try std.fmt.allocPrint(gpa, ".zig-cache/tmp/{s}", .{tmp.sub_path});
    defer gpa.free(root);
    const path = try std.fs.path.join(gpa, &.{ root, "session.jsonl" });
    defer gpa.free(path);
    try std.Io.Dir.cwd().writeFile(io, .{ .sub_path = path, .data = "log\n" });
    var storage: [256]u8 = undefined;
    var fixed: std.heap.FixedBufferAllocator = .init(&storage);
    const limited = fixed.allocator();
    var session: Session = .{ .gpa = limited, .io = io, .path = @constCast(path) };
    const line = try gpa.alloc(u8, 1024);
    defer gpa.free(line);
    @memset(line, 'x');
    switch (session.append(&.{line})) {
        .ok => return error.TestUnexpectedResult,
        .err => |e| e.deinit(limited),
        .uncertain => |e| {
            e.deinit(limited);
            return error.TestUnexpectedResult;
        },
    }
    const contents = try std.Io.Dir.cwd().readFileAlloc(io, path, gpa, .limited(4096));
    defer gpa.free(contents);
    try std.testing.expectEqualStrings("log\n", contents);
}

/// Removes one run's capture directory. A directory that is already gone is
/// the same result as one just removed, so a failed spawn reports nothing.
pub fn discardCaptureDirectory(io: std.Io, path: []const u8) !void {
    try std.Io.Dir.cwd().deleteTree(io, path);
}

fn freeDiag(gpa: std.mem.Allocator, value: anytype) void {
    if (comptime @TypeOf(value) == Diagnostic) {
        value.deinit(gpa);
    } else {
        gpa.free(value);
    }
}

test "uncommitted capture directory is removed without touching the session" {
    const io = std.testing.io;
    const gpa = std.testing.allocator;
    var tmp = std.testing.tmpDir(.{});
    defer tmp.cleanup();
    const root = try std.fmt.allocPrint(gpa, ".zig-cache/tmp/{s}", .{tmp.sub_path});
    defer gpa.free(root);
    const session_path = try std.fs.path.join(gpa, &.{ root, "session.jsonl" });
    defer gpa.free(session_path);
    try std.Io.Dir.cwd().writeFile(io, .{ .sub_path = session_path, .data = "log" });
    const captures = try std.fmt.allocPrint(gpa, "{s}.captures/run-1", .{session_path});
    defer gpa.free(captures);
    try std.Io.Dir.cwd().createDirPath(io, captures);
    const output = try std.fs.path.join(gpa, &.{ captures, "output.txt" });
    defer gpa.free(output);
    try std.Io.Dir.cwd().writeFile(io, .{ .sub_path = output, .data = "complete" });
    try discardCaptureDirectory(io, captures);
    try std.testing.expectError(error.FileNotFound, std.Io.Dir.cwd().access(io, captures, .{}));
    try std.Io.Dir.cwd().access(io, session_path, .{});
}
