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
    ui.capture_dir = ui.session.?.captureDirectory(ui.gpa, ui.io, at) catch {
        ui.printNote("create the run capture directory");
        return;
    };
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
