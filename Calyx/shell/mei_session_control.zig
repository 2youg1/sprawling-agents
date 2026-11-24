// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

const std = @import("std");
const leaf = @import("leaf");
const write = leaf.write;
const command = @import("mei_command.zig");
const effort = @import("mei_effort.zig");
const observation = @import("mei_observation.zig");
const mei_session = @import("mei_session.zig");

pub const Action = enum { task, steer, effort_menu, handled, quit };

const ResumeArg = enum { typed, resolved };

pub fn dispatch(ui: anytype, typed: []const u8) Action {
    const parsed = command.parse(typed);
    if (parsed == .quit) return .quit;
    if (ui.process != null) {
        if (parsed == .task) return .steer;
        ui.printNote("one Mei task is already running; Esc stops it");
        return .handled;
    }
    if (ui.replay_path != null) {
        ui.printNote("replay is read-only");
        return .handled;
    }
    switch (parsed) {
        .task => return .task,
        .new_session => _ = createSession(ui),
        .resume_session => |path| {
            if (path) |named| {
                resumeSession(ui, named, .typed);
            } else if (ui.previous_session) |previous| {
                resumeSession(ui, previous, .resolved);
            } else {
                ui.printNote("no previous Mei session; use /resume <jsonl>");
            }
        },
        .change_effort => return .effort_menu,
        .unknown => ui.printNote("unknown Mei command; available: /new, /resume [jsonl], /effect, /quit"),
        .quit => unreachable,
    }
    return .handled;
}

fn rememberCurrent(ui: anytype) void {
    if (ui.session) |*old| {
        if (old.historyLines().len > 0) {
            const copy = ui.gpa.dupe(u8, old.pathSlice()) catch null;
            if (copy) |previous| {
                if (ui.previous_session) |path| ui.gpa.free(path);
                ui.previous_session = previous;
            }
        }
        old.deinit();
        ui.session = null;
    }
}

pub fn createSession(ui: anytype) bool {
    const at = ui.now();
    switch (mei_session.Session.createIn(ui.gpa, ui.io, ui.session_dir, at)) {
        .ok => |session| {
            rememberCurrent(ui);
            for (ui.pending.items) |line| ui.gpa.free(line);
            ui.pending.clearRetainingCapacity();
            ui.session = session;
            ui.seen.reset();
            ui.clearConversation();
            return true;
        },
        .err => |diag| {
            defer ui.gpa.free(diag);
            ui.printNote(diag);
            return false;
        },
    }
}

fn resumeSession(ui: anytype, path_in: []const u8, arg: ResumeArg) void {
    const path = switch (arg) {
        .resolved => ui.gpa.dupe(u8, path_in) catch {
            ui.printNote("copy resume path");
            return;
        },
        .typed => if (std.fs.path.isAbsolute(path_in))
            ui.gpa.dupe(u8, path_in) catch {
                ui.printNote("copy resume path");
                return;
            }
        else
            std.fs.path.join(ui.gpa, &.{ ui.session_dir, path_in }) catch {
                ui.printNote("join resume path");
                return;
            },
    };
    defer ui.gpa.free(path);
    switch (mei_session.Session.import(ui.gpa, ui.io, path)) {
        .ok => |session| {
            ui.touch(true);
            const at = ui.now();
            for (ui.pending.items) |line| ui.gpa.free(line);
            ui.pending.clearRetainingCapacity();
            ui.seen.reset();
            ui.clearConversation();
            _ = ui.frame_arena.reset(.retain_capacity);
            const arena = ui.frame_arena.allocator();
            var entries: std.ArrayList(write.Entry) = .empty;
            for (session.historyLines()) |line| {
                const more = ui.seen.line(arena, line, at) catch continue;
                entries.appendSlice(arena, more) catch {};
            }
            rememberCurrent(ui);
            ui.session = session;
            ui.print(entries.items);
            ui.touch(true);
        },
        .err => |diag| {
            defer ui.gpa.free(diag);
            ui.printNote(diag);
        },
    }
}

pub fn chooseEffort(ui: anytype, selected: []const u8) void {
    const encoded = effort.updatedExtra(ui.gpa, ui.environ.get("HARNESS_EXTRA"), selected) catch {
        ui.printNote("could not update HARNESS_EXTRA; keep it as one JSON object");
        return;
    };
    defer ui.gpa.free(encoded);
    ui.environ.put("HARNESS_EXTRA", encoded) catch {
        ui.printNote("could not store the requested effort");
        return;
    };
    ui.seen.requestEffort(selected);
}

const Fake = struct {
    gpa: std.mem.Allocator,
    io: std.Io,
    environ: std.process.Environ.Map,
    seen: observation.Seen,
    session: ?mei_session.Session = null,
    previous_session: ?[]u8 = null,
    pending: std.ArrayList([]u8) = .empty,
    session_dir: []const u8,
    frame_arena: std.heap.ArenaAllocator,
    process: ?u8 = null,
    replay_path: ?[]const u8 = null,
    printed: usize = 0,

    fn release(self: *Fake) void {
        if (self.session) |*session| session.deinit();
        if (self.previous_session) |path| self.gpa.free(path);
        for (self.pending.items) |line| self.gpa.free(line);
        self.pending.deinit(self.gpa);
        self.seen.deinit();
        self.environ.deinit();
        self.frame_arena.deinit();
    }

    fn printNote(self: *Fake, said: []const u8) void {
        _ = self;
        _ = said;
    }

    fn print(self: *Fake, entries: []const write.Entry) void {
        self.printed += entries.len;
    }

    fn touch(self: *Fake, force: bool) void {
        _ = self;
        _ = force;
    }

    fn clearConversation(self: *Fake) void {
        _ = self;
    }

    fn now(self: *Fake) u64 {
        _ = self;
        return 0;
    }
};

fn commit(ui: *Fake, line: []const u8) !void {
    const session = if (ui.session) |*session| session else return error.TestUnexpectedResult;
    switch (session.append(&.{line})) {
        .ok => {},
        else => return error.TestUnexpectedResult,
    }
}

test "a running task turns ordinary text into steer" {
    var ui = Fake{
        .gpa = std.testing.allocator,
        .io = std.testing.io,
        .environ = std.process.Environ.Map.init(std.testing.allocator),
        .seen = try observation.Seen.init(std.testing.allocator, "auto"),
        .session_dir = ".",
        .frame_arena = .init(std.testing.allocator),
        .process = 1,
    };
    defer ui.release();
    try std.testing.expect(dispatch(&ui, "narrow the search") == .steer);
    try std.testing.expect(dispatch(&ui, "/new") == .handled);
}

fn resumeSwitch(gpa: std.mem.Allocator, io: std.Io, dir: []const u8) !void {
    var ui = Fake{
        .gpa = gpa,
        .io = io,
        .environ = std.process.Environ.Map.init(gpa),
        .seen = try observation.Seen.init(gpa, "auto"),
        .session_dir = dir,
        .frame_arena = .init(gpa),
    };
    defer ui.release();

    try std.testing.expectEqual(Action.handled, dispatch(&ui, "/new"));
    const first = try gpa.dupe(u8, ui.session.?.pathSlice());
    defer gpa.free(first);
    try commit(&ui, "{\"role\":\"assistant\",\"content\":\"one\"}");

    try std.testing.expectEqual(Action.handled, dispatch(&ui, "/new"));
    const second = try gpa.dupe(u8, ui.session.?.pathSlice());
    defer gpa.free(second);
    try commit(&ui, "{\"role\":\"assistant\",\"content\":\"two\"}");

    try std.testing.expectEqual(Action.handled, dispatch(&ui, "/resume"));
    try std.testing.expectEqualStrings(first, ui.session.?.pathSlice());
    try std.testing.expectEqualStrings(second, ui.previous_session.?);
}

test "a resume to the previous session owns its path" {
    const gpa = std.testing.allocator;
    const io = std.testing.io;
    var tmp = std.testing.tmpDir(.{});
    defer tmp.cleanup();
    const relative = try std.fmt.allocPrint(gpa, ".zig-cache/tmp/{s}", .{tmp.sub_path});
    defer gpa.free(relative);
    var cwd_buf: [std.fs.max_path_bytes]u8 = undefined;
    const cwd_len = try std.process.currentPath(io, &cwd_buf);
    const absolute = try std.fs.path.join(gpa, &.{ cwd_buf[0..cwd_len], relative });
    defer gpa.free(absolute);
    try resumeSwitch(gpa, io, absolute);
}

test "a resume into a relative session directory joins its path once" {
    const gpa = std.testing.allocator;
    const io = std.testing.io;
    var tmp = std.testing.tmpDir(.{});
    defer tmp.cleanup();
    const relative = try std.fmt.allocPrint(gpa, ".zig-cache/tmp/{s}", .{tmp.sub_path});
    defer gpa.free(relative);
    try resumeSwitch(gpa, io, relative);
}
