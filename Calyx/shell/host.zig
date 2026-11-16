// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// Permanent city host C ABI: the shell draws and handles keys; the host
// owns wire/accounting types and projects Committed → scene Entry bytes.

const std = @import("std");
const leaf = @import("leaf");
const write = leaf.write;
const part = leaf.part;
const reader_mod = leaf.scene;

pub const FaceTag = enum(u32) {
    opening = 0,
    cli = 1,
    quiet_host = 2,
    headless = 3,
    stopping = 4,
    gone = 5,
};

pub const CloseMode = enum(u32) {
    drain = 0,
    interrupt = 1,
};

pub const Sinks = enum(u32) {
    held = 0,
    cut = 1,
};

pub const Handoff = enum(u32) {
    written = 0,
    skipped = 1,
};

pub const LifecycleEvent = enum(u32) {
    web = 0,
    back = 1,
    quit_drain = 2,
    quit_interrupt = 3,
    terminal_lost = 4,
};

pub const HostEventKind = enum(u32) {
    empty = 0,
    face = 1,
    notice = 2,
    pairing = 3,
    entries = 4,
    live_meta = 5,
    lost = 6,
    asking = 7,
};

pub const Status = enum(i32) {
    ok = 0,
    refused = 1,
    buffer_too_small = 2,
};

/// Packed face payload written into the poll buffer for `.face`.
pub const FaceBytes = extern struct {
    tag: u32,
    mode: u32,
    sinks: u32,
    handoff: u32,
};

/// Packed live-meta payload for `.live_meta` (strings length-prefixed after header).
pub const LiveMetaHeader = extern struct {
    waiting_more: u32,
    working_since: u64, // maxInt = absent
    calling_count: u32,
    room_len: u32,
    offer_len: u32,
    waiting_len: u32,
};

pub const CityConfig = extern struct {
    url_ptr: [*]const u8,
    url_len: usize,
    city_ptr: [*]const u8,
    city_len: usize,
    token_ptr: ?[*]const u8,
    token_len: usize,
    surface: u32, // 0=cli, 1=quiet_host
};

/// In/out buffer pair for host callbacks that copy text into a caller buffer.
pub const BufIo = extern struct {
    in_ptr: [*]const u8,
    in_len: usize,
    out_ptr: [*]u8,
    out_cap: usize,
    out_written: *usize,
};

pub const Host = extern struct {
    ctx: *anyopaque,
    poll: *const fn (
        ctx: *anyopaque,
        into: [*]u8,
        capacity: usize,
        written: *usize,
    ) callconv(.c) u32,
    tell: *const fn (ctx: *anyopaque, event: u32, arg: u32) callconv(.c) void,
    carry_line: *const fn (ctx: *anyopaque, line_ptr: [*]const u8, line_len: usize) callconv(.c) void,
    open_paired: *const fn (ctx: *anyopaque, url_ptr: [*]const u8, url_len: usize) callconv(.c) i32,
    carry_remote: *const fn (ctx: *anyopaque, io: *BufIo) callconv(.c) i32,
    clock_ms: *const fn (ctx: *anyopaque, out: *u64) callconv(.c) i32,
    post_json: *const fn (ctx: *anyopaque, json_ptr: [*]const u8, json_len: usize) callconv(.c) i32,
    answer_json: *const fn (ctx: *anyopaque, io: *BufIo) callconv(.c) i32,
};

pub fn urlOf(config: *const CityConfig) []const u8 {
    return config.url_ptr[0..config.url_len];
}

pub fn cityOf(config: *const CityConfig) []const u8 {
    return config.city_ptr[0..config.city_len];
}

pub fn tokenOf(config: *const CityConfig) ?[]const u8 {
    if (config.token_len == 0) return null;
    const ptr = config.token_ptr orelse return null;
    return ptr[0..config.token_len];
}

/// Decode host-projected entry list (`u32 count` + write.zig entry encodings).
pub fn readEntries(arena: std.mem.Allocator, bytes: []const u8) ![]write.Entry {
    if (bytes.len < 4) return error.Malformed;
    var r: reader_mod.Reader = .{ .bytes = bytes };
    const count = try r.word();
    var list: std.ArrayList(write.Entry) = .empty;
    errdefer list.deinit(arena);
    var i: u32 = 0;
    while (i < count) : (i += 1) {
        try list.append(arena, try readOneEntry(arena, &r));
    }
    return try list.toOwnedSlice(arena);
}

fn readOneEntry(arena: std.mem.Allocator, r: *reader_mod.Reader) !write.Entry {
    const tag = try r.tag();
    return switch (tag) {
        .Banner => .{ .banner = .{
            .city = try arena.dupe(u8, try r.text()),
            .address = try arena.dupe(u8, try r.text()),
            .url = try arena.dupe(u8, try r.text()),
        } },
        .You => .{ .you = .{
            .at = absTime(try r.word()),
            .said = try arena.dupe(u8, try r.text()),
        } },
        .Head => blk: {
            const at = absTime(try r.word());
            const resident = try arena.dupe(u8, try r.text());
            const n = try r.word();
            const facts = try arena.alloc([]const u8, n);
            var i: u32 = 0;
            while (i < n) : (i += 1) {
                facts[i] = try arena.dupe(u8, try r.text());
            }
            break :blk .{ .head = .{ .at = at, .resident = resident, .facts = facts } };
        },
        .Reasoning => .{ .reasoning = try r.long() },
        .Tool => .{ .tool = .{
            .at = absTime(try r.word()),
            .name = try arena.dupe(u8, try r.text()),
            .subject = try arena.dupe(u8, try r.text()),
            .took_us = absLong(try r.long()),
            .outcome = try r.outcome(),
        } },
        .Reply => .{ .reply = try arena.dupe(u8, try r.text()) },
        .Note => .{ .note = try arena.dupe(u8, try r.text()) },
        .Ended => .{ .ended = .{
            .at = absTime(try r.word()),
            .ending = try r.ending(),
            .took_s = absLong(try r.long()),
        } },
        .Resolved => .{ .resolved = .{
            .at = absTime(try r.word()),
            .verdict = try r.verdict(),
            .what = try arena.dupe(u8, try r.text()),
        } },
        else => error.Malformed,
    };
}

fn absTime(v: u32) ?u32 {
    return if (v == reader_mod.no_time) null else v;
}

fn absLong(v: u64) ?u64 {
    return if (v == reader_mod.no_count) null else v;
}

/// Encode a note as host entry-list bytes (for stub hosts and tests).
pub fn encodeNotes(gpa: std.mem.Allocator, notes: []const []const u8) ![]u8 {
    var list: std.ArrayList(u8) = .empty;
    errdefer list.deinit(gpa);
    var count_buf: [4]u8 = undefined;
    std.mem.writeInt(u32, &count_buf, @intCast(notes.len), .little);
    try list.appendSlice(gpa, &count_buf);
    for (notes) |said| {
        try list.append(gpa, @intFromEnum(part.Part.Note));
        const length: u32 = @intCast(@min(said.len, std.math.maxInt(u32)));
        var len_buf: [4]u8 = undefined;
        std.mem.writeInt(u32, &len_buf, length, .little);
        try list.appendSlice(gpa, &len_buf);
        try list.appendSlice(gpa, said[0..length]);
    }
    return try list.toOwnedSlice(gpa);
}

test "readEntries round-trips a note list" {
    const gpa = std.testing.allocator;
    const bytes = try encodeNotes(gpa, &.{"hello city"});
    defer gpa.free(bytes);
    var arena_state = std.heap.ArenaAllocator.init(gpa);
    defer arena_state.deinit();
    const entries = try readEntries(arena_state.allocator(), bytes);
    try std.testing.expectEqual(@as(usize, 1), entries.len);
    try std.testing.expectEqualStrings("hello city", entries[0].note);
}

test "malformed entry bytes are refused" {
    const gpa = std.testing.allocator;
    var arena_state = std.heap.ArenaAllocator.init(gpa);
    defer arena_state.deinit();
    try std.testing.expectError(error.Malformed, readEntries(arena_state.allocator(), &.{1}));
}

test "seeded host entry fuzz never traps" {
    var prng = std.Random.DefaultPrng.init(0xc17a_0e11);
    const random = prng.random();
    const gpa = std.testing.allocator;
    var round: usize = 0;
    while (round < 2048) : (round += 1) {
        var buf: [128]u8 = undefined;
        const n = random.intRangeAtMost(usize, 0, buf.len);
        random.bytes(buf[0..n]);
        var arena_state = std.heap.ArenaAllocator.init(gpa);
        defer arena_state.deinit();
        _ = readEntries(arena_state.allocator(), buf[0..n]) catch {};
    }
}
