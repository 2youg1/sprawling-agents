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
    const steer_path = std.fmt.allocPrint(ui.gpa, "{s}{c}steer.jsonl", .{ ui.capture_dir.?, std.fs.path.sep }) catch {
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
        }
    }
}

fn onLine(ui: anytype, line: []u8) void {
    const is_message = blk: {
        var parsed = std.json.parseFromSlice(std.json.Value, ui.gpa, line, .{}) catch break :blk false;
        defer parsed.deinit();
        if (parsed.value != .object) break :blk false;
        const role = parsed.value.object.get("role") orelse break :blk false;
        if (role != .string) break :blk false;
        if (!std.mem.eql(u8, role.string, "user") and !std.mem.eql(u8, role.string, "assistant")) break :blk false;
        if (parsed.value.object.get("content") == null) break :blk false;
        break :blk true;
    };
    if (is_message) {
        ui.pending.append(ui.gpa, line) catch {
            ui.gpa.free(line);
            return;
        };
    }
    defer if (!is_message) ui.gpa.free(line);
    const arena = ui.frame_arena.allocator();
    const entries = ui.seen.line(arena, line, ui.now()) catch return;
    ui.print(entries);
}

fn onFinished(ui: anytype, finished: mei_process.Finished) void {
    if (ui.process) |*process| {
        process.finish();
        ui.process = null;
    }
    const successful = !ui.lost and !finished.cancelled and finished.result == .ok;
    var capture = CaptureFate.discard;
    if (successful) {
        if (ui.session) |*session| {
            switch (session.append(ui.pending.items)) {
                .ok => capture = .keep,
                .err => |diag| {
                    defer diag.deinit(ui.gpa);
                    session.deinit();
                    ui.session = null;
                    ui.printNote(diag.text);
                },
                .uncertain => |diag| {
                    defer diag.deinit(ui.gpa);
                    session.deinit();
                    ui.session = null;
                    capture = .keep;
                    ui.printNote(diag.text);
                },
            }
        }
    }
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
    ui.print(entries);
    ui.touch(true);
    ui.ended();
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
