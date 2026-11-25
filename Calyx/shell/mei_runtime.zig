// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

const std = @import("std");
const observation = @import("mei_observation.zig");
const mei_process = @import("mei_process.zig");
const mei_session = @import("mei_session.zig");

const CaptureFate = enum { keep, discard };

pub fn submit(ui: anytype, typed: []const u8, at: u64) void {
    std.debug.assert(ui.capture_dir == null);
    std.debug.assert(ui.steer_path == null);
    ui.capture_dir = ui.session.?.captureDirectory(ui.gpa, ui.io, at) catch {
        ui.printNote("create the run capture directory");
        return;
    };
    const steer_path = mei_process.steerPath(ui.gpa, ui.capture_dir.?) catch {
        finishCapture(ui, .discard);
        ui.printNote("allocate the Mei steer path");
        return;
    };
    createEmptySteer(ui.io, steer_path) catch {
        ui.gpa.free(steer_path);
        finishCapture(ui, .discard);
        ui.printNote("create the Mei steer file");
        return;
    };
    ui.steer_path = steer_path;
    for (ui.pending.items) |line| ui.gpa.free(line);
    ui.pending.clearRetainingCapacity();
    ui.seen.begin(at);
    ui.status(.{ .counted = ui.seen.counts() });
    const resume_path = if (ui.session) |*session| session.resumePath() else null;
    switch (mei_process.Process.spawn(ui.gpa, ui.io, &ui.bus, ui.mei_program, typed, resume_path, ui.capture_dir.?, ui.environ)) {
        .ok => |process| ui.process = process,
        .err => |diag| {
            defer ui.gpa.free(diag);
            finishCapture(ui, .discard);
            const arena = ui.frame_arena.allocator();
            const entries = ui.seen.finished(arena, .{ .err = diag }, ui.now()) catch return;
            ui.print(entries);
            ui.touch(true);
            ui.ended();
        },
    }
}

pub fn steer(ui: anytype, typed: []const u8) void {
    const path = ui.steer_path orelse {
        ui.printNote("no running Mei task to steer");
        return;
    };
    appendSteer(ui.gpa, ui.io, path, typed) catch {
        ui.printNote("append the Mei steer");
    };
}

pub fn drain(ui: anytype) void {
    var drained: std.ArrayList(mei_process.Event) = .empty;
    defer drained.deinit(ui.gpa);
    ui.bus.drainInto(&drained);
    for (drained.items) |event| {
        switch (event) {
            .line => |line| onLine(ui, line),
            .finished => |finished| onFinished(ui, finished),
            .git_head => |head| ui.gitHeadArrived(head),
        }
    }
}

fn onLine(ui: anytype, line: []u8) void {
    var recorded = false;
    defer if (!recorded) ui.gpa.free(line);
    var parsed = std.json.parseFromSlice(std.json.Value, ui.gpa, line, .{}) catch null;
    defer if (parsed) |*record| record.deinit();
    const value: std.json.Value = if (parsed) |record| record.value else .null;
    if (journalLine(value)) {
        ui.pending.append(ui.gpa, line) catch return;
        recorded = true;
    }
    const arena = ui.frame_arena.allocator();
    const entries = ui.seen.projectLine(arena, line, value, ui.now()) catch return;
    ui.print(entries);
}

fn journalLine(value: std.json.Value) bool {
    if (value != .object) return false;
    if (value.object.get("type")) |kind| {
        if (kind == .string and std.mem.eql(u8, kind.string, "mei.obs")) {
            const name = value.object.get("kind") orelse return false;
            return name == .string and std.mem.eql(u8, name.string, "image");
        }
    }
    const role = value.object.get("role") orelse return false;
    return role == .string and
        (std.mem.eql(u8, role.string, "user") or std.mem.eql(u8, role.string, "assistant")) and
        value.object.get("content") != null;
}

fn onFinished(ui: anytype, finished: mei_process.Finished) void {
    if (ui.process) |*process| {
        process.finish();
        ui.process = null;
    }
    const successful = !ui.lost and !finished.cancelled and finished.result == .ok;
    const capture = storeUsage(ui, successful);
    for (ui.pending.items) |line| ui.gpa.free(line);
    ui.pending.clearRetainingCapacity();
    finishCapture(ui, capture);
    const arena = ui.frame_arena.allocator();
    const at = ui.now();
    const entries = if (finished.cancelled)
        ui.seen.cancelled(arena, at) catch return
    else
        ui.seen.finished(arena, finished.result, at) catch return;
    finished.result.deinit(ui.gpa);
    ui.refreshGit();
    ui.print(entries);
    ui.touch(true);
    ui.ended();
}

fn storeUsage(ui: anytype, successful: bool) CaptureFate {
    const session = if (ui.session) |*session| session else return .discard;
    if (!successful and (ui.lost or session.resumePath() == null)) return .discard;
    var usage_buf: [256]u8 = undefined;
    const usage = ui.seen.sessionUsage(&usage_buf) catch unreachable;
    const messages: []const []const u8 = if (successful) ui.pending.items else &.{};
    if (messages.len == 0 and successful) return .discard;
    const lines = ui.gpa.alloc([]const u8, messages.len + 1) catch {
        ui.printNote("could not save the Mei session statistics");
        return .discard;
    };
    defer ui.gpa.free(lines);
    @memcpy(lines[0..messages.len], messages);
    lines[messages.len] = usage;
    const outcome = session.append(lines);
    switch (outcome) {
        .ok => return if (successful) .keep else .discard,
        .err, .uncertain => |diag| {
            defer diag.deinit(ui.gpa);
            session.deinit();
            ui.session = null;
            ui.printNote(diag.text);
            return if (successful and outcome == .uncertain) .keep else .discard;
        },
    }
}

pub fn stop(ui: anytype) void {
    if (ui.process) |*process| {
        switch (process.requestStop()) {
            .ok => {},
            .err => |failure| {
                defer ui.gpa.free(failure);
                ui.printNote(failure);
            },
        }
    }
}

fn finishCapture(ui: anytype, fate: CaptureFate) void {
    if (ui.steer_path) |path| {
        std.Io.Dir.cwd().deleteFile(ui.io, path) catch {};
        ui.gpa.free(path);
        ui.steer_path = null;
    }
    const path = ui.capture_dir orelse return;
    if (fate == .discard) {
        mei_session.discardCaptureDirectory(ui.io, path) catch |err| {
            const message = std.fmt.allocPrint(ui.gpa, "discard the uncommitted capture directory {s}: {s}", .{ path, @errorName(err) }) catch null;
            if (message) |owned| {
                defer ui.gpa.free(owned);
                ui.printNote(owned);
            }
        };
    }
    ui.gpa.free(path);
    ui.capture_dir = null;
}

fn createEmptySteer(io: std.Io, path: []const u8) !void {
    const file = try std.Io.Dir.cwd().createFile(io, path, .{});
    file.close(io);
}

fn appendSteer(gpa: std.mem.Allocator, io: std.Io, path: []const u8, text: []const u8) !void {
    const line = try std.json.Stringify.valueAlloc(gpa, .{ .text = text }, .{});
    defer gpa.free(line);
    const file = try std.Io.Dir.cwd().openFile(io, path, .{ .mode = .read_write });
    defer file.close(io);
    const end = try file.length(io);
    var buf: [512]u8 = undefined;
    var writer = file.writer(io, &buf);
    try writer.seekTo(end);
    try writer.interface.writeAll(line);
    try writer.interface.writeAll("\n");
    try writer.interface.flush();
}

test "a steer line is one JSON object ending in a newline" {
    const gpa = std.testing.allocator;
    const io = std.testing.io;
    const file_path = "mei-steer-append-test.jsonl";
    defer std.Io.Dir.cwd().deleteFile(io, file_path) catch {};
    try createEmptySteer(io, file_path);
    try appendSteer(gpa, io, file_path, "narrow the search");
    const contents = try std.Io.Dir.cwd().readFileAlloc(io, file_path, gpa, .limited(4096));
    defer gpa.free(contents);
    try std.testing.expectEqualStrings("{\"text\":\"narrow the search\"}\n", contents);
}

test "creating the steer file leaves the capture directory empty" {
    const gpa = std.testing.allocator;
    const io = std.testing.io;
    const capture = "mei-steer-capture-empty-test";
    const path = try mei_process.steerPath(gpa, capture);
    defer gpa.free(path);
    defer std.Io.Dir.cwd().deleteTree(io, capture) catch {};
    defer std.Io.Dir.cwd().deleteFile(io, path) catch {};
    try std.Io.Dir.cwd().createDirPath(io, capture);
    try createEmptySteer(io, path);
    var dir = try std.Io.Dir.cwd().openDir(io, capture, .{ .iterate = true });
    defer dir.close(io);
    var it = dir.iterate();
    try std.testing.expect((try it.next(io)) == null);
    try std.testing.expectEqualStrings("mei-steer-capture-empty-test.steer.jsonl", path);
}
