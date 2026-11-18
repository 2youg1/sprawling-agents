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

pub const Action = enum { task, handled, quit };

pub fn dispatch(ui: anytype, typed: []const u8) Action {
    const parsed = command.parse(typed);
    if (parsed == .quit) return .quit;
    if (ui.process != null) {
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
                resumeSession(ui, named);
            } else if (ui.previous_session) |previous| {
                resumeSession(ui, previous);
            } else {
                ui.printNote("no previous Mei session; use /resume <jsonl>");
            }
        },
        .change_effort => switchEffort(ui),
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
            var buf: [512]u8 = undefined;
            const msg = std.fmt.bufPrint(&buf, "new Mei session: {s}", .{session.pathSlice()}) catch "new Mei session";
            ui.printNote(msg);
            return true;
        },
        .err => |diag| {
            defer ui.gpa.free(diag);
            ui.printNote(diag);
            return false;
        },
    }
}

fn resumeSession(ui: anytype, path_in: []const u8) void {
    const path = if (std.fs.path.isAbsolute(path_in))
        path_in
    else
        std.fs.path.join(ui.gpa, &.{ ui.session_dir, path_in }) catch {
            ui.printNote("join resume path");
            return;
        };
    defer if (!std.fs.path.isAbsolute(path_in)) ui.gpa.free(path);
    switch (mei_session.Session.import(ui.gpa, ui.io, path)) {
        .ok => |session| {
            const at = ui.now();
            for (ui.pending.items) |line| ui.gpa.free(line);
            ui.pending.clearRetainingCapacity();
            ui.seen.reset();
            _ = ui.frame_arena.reset(.retain_capacity);
            const arena = ui.frame_arena.allocator();
            var entries: std.ArrayList(write.Entry) = .empty;
            for (session.historyLines()) |line| {
                const more = ui.seen.line(arena, line, at) catch continue;
                entries.appendSlice(arena, more) catch {};
            }
            var buf: [512]u8 = undefined;
            const msg = std.fmt.bufPrint(&buf, "resumed Mei session: {s}", .{session.pathSlice()}) catch "resumed Mei session";
            entries.append(arena, .{ .note = arena.dupe(u8, msg) catch msg }) catch {};
            rememberCurrent(ui);
            ui.session = session;
            ui.print(entries.items);
        },
        .err => |diag| {
            defer ui.gpa.free(diag);
            ui.printNote(diag);
        },
    }
}

fn switchEffort(ui: anytype) void {
    const selected = effort.next(ui.seen.currentEffort(), ui.seen.effortLevels()) orelse {
        ui.printNote("Magpie effort levels arrive with the first model selection");
        return;
    };
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
    var buf: [128]u8 = undefined;
    const msg = std.fmt.bufPrint(&buf, "requested effort for the next run: {s}", .{selected}) catch "requested effort updated";
    ui.printNote(msg);
}
