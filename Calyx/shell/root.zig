// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

const std = @import("std");
const argv = @import("argv.zig");
const mei_line = @import("mei_line.zig");
const mei_path = @import("mei_path.zig");
const lifecycle = @import("lifecycle.zig");
const editor = @import("editor.zig");
const local_time = @import("local_time.zig");
const language = @import("language.zig");
const program_status = @import("program_status.zig");
const menu = @import("menu.zig");

pub const panic = std.debug.no_panic;

pub const ReplayStatus = enum(u32) {
    none = 0,
    found = 1,
    missing_value = 2,
    buffer_too_small = 3,
};

pub const MeiStatus = enum(u32) {
    ok = 0,
    buffer_too_small = 1,
};

export fn calyx_shell_replay_flag(
    argc: usize,
    argv_ptr: [*]const [*]const u8,
    argv_lens: [*]const usize,
    into: [*]u8,
    capacity: usize,
    written: *usize,
) u32 {
    var stack: [64][]const u8 = undefined;
    const n = @min(argc, stack.len);
    var i: usize = 0;
    while (i < n) : (i += 1) {
        stack[i] = argv_ptr[i][0..argv_lens[i]];
    }
    return switch (argv.replayFlag(stack[0..n])) {
        .none => @backingInt(ReplayStatus.none),
        .missing_value => @backingInt(ReplayStatus.missing_value),
        .path => |path| blk: {
            if (capacity < path.len) break :blk @backingInt(ReplayStatus.buffer_too_small);
            @memcpy(into[0..path.len], path);
            written.* = path.len;
            break :blk @backingInt(ReplayStatus.found);
        },
    };
}

export fn calyx_shell_mei_program(
    mei_bin_ptr: [*]const u8,
    mei_bin_len: usize,
    sibling_ptr: [*]const u8,
    sibling_len: usize,
    sibling_exists: u8,
    into: [*]u8,
    capacity: usize,
    written: *usize,
) u32 {
    const mei_bin: ?[]const u8 = if (mei_bin_len == 0) null else mei_bin_ptr[0..mei_bin_len];
    const sibling: ?[]const u8 = if (sibling_len == 0) null else sibling_ptr[0..sibling_len];
    const path = mei_path.resolve(mei_bin, sibling, sibling_exists != 0, into[0..capacity]) catch {
        return @backingInt(MeiStatus.buffer_too_small);
    };
    written.* = path.len;
    return @backingInt(MeiStatus.ok);
}

export fn calyx_shell_mei_sibling_name(into: [*]u8, capacity: usize, written: *usize) u32 {
    const name = mei_path.siblingName();
    if (capacity < name.len) return @backingInt(MeiStatus.buffer_too_small);
    @memcpy(into[0..name.len], name);
    written.* = name.len;
    return @backingInt(MeiStatus.ok);
}

// Keep city UI exports in the installed shell archive (not test-only).
comptime {
    _ = @import("city_ui.zig");
}

test {
    _ = argv;
    _ = mei_path;
    _ = lifecycle;
    _ = editor;
    _ = local_time;
    _ = language;
    _ = program_status;
    _ = menu;
    _ = @import("termio.zig");
    _ = @import("screen.zig");
    _ = @import("mei_command.zig");
    _ = @import("mei_effort.zig");
    _ = @import("mei_observation.zig");
    _ = @import("mei_session_control.zig");
    _ = @import("mei_runtime.zig");
    _ = @import("mei_process.zig");
    _ = @import("mei_session.zig");
    _ = @import("mei_ui.zig");
    _ = @import("host.zig");
    _ = @import("city_ui.zig");
}
