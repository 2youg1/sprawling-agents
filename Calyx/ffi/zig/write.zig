// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// Writing a scene: the bytes `console_ffi::scene` writes, little-endian,
// every string a u32 length and its bytes. Mirrors Calyx/ffi/src/scene.rs
// so a shell can build frames without Rust and round-trip with Reader.

const std = @import("std");
const part = @import("part.zig");
const scene = @import("scene.zig");

pub const Error = std.mem.Allocator.Error || std.Io.Writer.Error;

pub const Banner = struct { city: []const u8, address: []const u8, url: []const u8 };
pub const You = struct { at: ?u32, said: []const u8 };
pub const Head = struct { at: ?u32, resident: []const u8, facts: []const []const u8 };
pub const Tool = struct {
    at: ?u32,
    name: []const u8,
    subject: []const u8,
    took_us: ?u64,
    outcome: part.Outcome,
};
pub const Ended = struct { at: ?u32, ending: part.Ending, took_s: ?u64 };
pub const Resolved = struct { at: ?u32, verdict: part.Verdict, what: []const u8 };

pub const Entry = union(enum) {
    banner: Banner,
    you: You,
    head: Head,
    reasoning: u64,
    tool: Tool,
    reply: []const u8,
    note: []const u8,
    ended: Ended,
    resolved: Resolved,

    pub fn tag(self: Entry) part.Part {
        return switch (self) {
            .banner => .Banner,
            .you => .You,
            .head => .Head,
            .reasoning => .Reasoning,
            .tool => .Tool,
            .reply => .Reply,
            .note => .Note,
            .ended => .Ended,
            .resolved => .Resolved,
        };
    }
};

pub const Waiting = struct { what: []const u8, more: usize };
pub const Working = struct { resident: []const u8, since: ?u32 };
pub const Calling = struct { name: []const u8, subject: []const u8 };
pub const Composer = struct {
    typed: []const u8,
    cursor: usize,
    placeholder: []const u8,
    room: []const u8,
    offer: []const u8,
};
pub const Choice = struct { spelling: []const u8, takes: []const u8, summary: []const u8 };
pub const Menu = struct { shown: []const Choice, chosen: usize, more: usize };

pub const Live = struct {
    waiting: ?Waiting = null,
    working: ?Working = null,
    calling: []const Calling = &.{},
    asking: ?u64 = null,
    composer: Composer,
    menu: ?Menu = null,
};

pub const Inline = struct {
    columns: u16,
    rows: u16,
    erase: u16,
    previous: ?part.Part,
    entries: []const Entry,
    live: ?Live = null,
};

pub const Quiet = struct {
    columns: u16,
    rows: u16,
    url: []const u8,
    code: []const u8,
    key: ?[]const u8 = null,
    transient: ?[]const u8 = null,
};

/// Allocator-backed scene bytes the leaf's Reader consumes.
pub const Scene = struct {
    bytes: std.ArrayList(u8),

    pub fn deinit(self: *Scene, gpa: std.mem.Allocator) void {
        self.bytes.deinit(gpa);
    }

    pub fn inlineFrame(gpa: std.mem.Allocator, frame: Inline) Error!Scene {
        var self: Scene = .{ .bytes = .empty };
        errdefer self.deinit(gpa);
        try writeInlineList(gpa, &self.bytes, frame);
        return self;
    }

    pub fn quietFrame(gpa: std.mem.Allocator, frame: Quiet) Error!Scene {
        var self: Scene = .{ .bytes = .empty };
        errdefer self.deinit(gpa);
        try writeQuietList(gpa, &self.bytes, frame);
        return self;
    }
};

pub fn writeInline(out: *std.Io.Writer, frame: Inline) Error!void {
    try tag(out, .Inline);
    try short(out, frame.columns);
    try short(out, frame.rows);
    try short(out, frame.erase);
    try byte(out, if (frame.previous) |p| @intFromEnum(p) else 0);
    try count(out, frame.entries.len);
    for (frame.entries) |entry| try writeEntry(out, entry);
    if (frame.live) |live| try writeLive(out, live);
}

pub fn writeQuiet(out: *std.Io.Writer, frame: Quiet) Error!void {
    try tag(out, .Quiet);
    try short(out, frame.columns);
    try short(out, frame.rows);
    try text(out, frame.url);
    try text(out, frame.code);
    try text(out, frame.key orelse "");
    try text(out, frame.transient orelse "");
}

fn writeInlineList(gpa: std.mem.Allocator, list: *std.ArrayList(u8), frame: Inline) Error!void {
    var aw: std.Io.Writer.Allocating = .fromArrayList(gpa, list);
    defer list.* = aw.toArrayList();
    try writeInline(&aw.writer, frame);
}

fn writeQuietList(gpa: std.mem.Allocator, list: *std.ArrayList(u8), frame: Quiet) Error!void {
    var aw: std.Io.Writer.Allocating = .fromArrayList(gpa, list);
    defer list.* = aw.toArrayList();
    try writeQuiet(&aw.writer, frame);
}

fn writeEntry(out: *std.Io.Writer, entry: Entry) Error!void {
    try tag(out, entry.tag());
    switch (entry) {
        .banner => |b| {
            try text(out, b.city);
            try text(out, b.address);
            try text(out, b.url);
        },
        .you => |y| {
            try time(out, y.at);
            try text(out, y.said);
        },
        .head => |h| {
            try time(out, h.at);
            try text(out, h.resident);
            try count(out, h.facts.len);
            for (h.facts) |fact| try text(out, fact);
        },
        .reasoning => |characters| try long(out, characters),
        .tool => |t| {
            try time(out, t.at);
            try text(out, t.name);
            try text(out, t.subject);
            try longOpt(out, t.took_us);
            try byte(out, @intFromEnum(t.outcome));
        },
        .reply => |said| try text(out, said),
        .note => |said| try text(out, said),
        .ended => |e| {
            try time(out, e.at);
            try byte(out, @intFromEnum(e.ending));
            try longOpt(out, e.took_s);
        },
        .resolved => |r| {
            try time(out, r.at);
            try byte(out, @intFromEnum(r.verdict));
            try text(out, r.what);
        },
    }
}

fn writeLive(out: *std.Io.Writer, live: Live) Error!void {
    if (live.waiting) |waiting| {
        try tag(out, .Waiting);
        try text(out, waiting.what);
        try count(out, waiting.more);
    }
    if (live.working) |working| {
        try tag(out, .Working);
        try text(out, working.resident);
        try time(out, working.since);
    }
    for (live.calling) |calling| {
        try tag(out, .Calling);
        try text(out, calling.name);
        try text(out, calling.subject);
    }
    if (live.asking) |runs| {
        try tag(out, .Asking);
        try long(out, runs);
    }
    try tag(out, .Composer);
    try text(out, live.composer.typed);
    try count(out, live.composer.cursor);
    try text(out, live.composer.placeholder);
    try text(out, live.composer.room);
    try text(out, live.composer.offer);
    if (live.menu) |menu| {
        try tag(out, .Menu);
        try count(out, menu.chosen);
        try count(out, menu.more);
        try count(out, menu.shown.len);
        for (menu.shown) |choice| {
            try text(out, choice.spelling);
            try text(out, choice.takes);
            try text(out, choice.summary);
        }
    }
}

fn tag(out: *std.Io.Writer, value: part.Part) Error!void {
    try byte(out, @intFromEnum(value));
}

fn byte(out: *std.Io.Writer, value: u8) Error!void {
    try out.writeAll(&.{value});
}

fn short(out: *std.Io.Writer, value: u16) Error!void {
    var buf: [2]u8 = undefined;
    std.mem.writeInt(u16, &buf, value, .little);
    try out.writeAll(&buf);
}

fn word(out: *std.Io.Writer, value: u32) Error!void {
    var buf: [4]u8 = undefined;
    std.mem.writeInt(u32, &buf, value, .little);
    try out.writeAll(&buf);
}

fn count(out: *std.Io.Writer, value: usize) Error!void {
    try word(out, std.math.cast(u32, value) orelse std.math.maxInt(u32));
}

fn long(out: *std.Io.Writer, value: u64) Error!void {
    var buf: [8]u8 = undefined;
    std.mem.writeInt(u64, &buf, value, .little);
    try out.writeAll(&buf);
}

fn longOpt(out: *std.Io.Writer, value: ?u64) Error!void {
    try long(out, value orelse scene.no_count);
}

fn time(out: *std.Io.Writer, at: ?u32) Error!void {
    try word(out, at orelse scene.no_time);
}

fn text(out: *std.Io.Writer, value: []const u8) Error!void {
    const length = std.math.cast(u32, value.len) orelse std.math.maxInt(u32);
    const kept = value[0..@min(value.len, length)];
    try word(out, length);
    try out.writeAll(kept);
}

test "inline scene round-trips tags and fields" {
    const gpa = std.testing.allocator;
    const entries = [_]Entry{
        .{ .banner = .{ .city = "city", .address = "addr", .url = "https://x" } },
        .{ .you = .{ .at = 3600, .said = "hi" } },
        .{ .head = .{ .at = null, .resident = "mei", .facts = &.{"a"} } },
        .{ .reasoning = 12 },
        .{ .tool = .{
            .at = 2,
            .name = "run",
            .subject = "t",
            .took_us = 9,
            .outcome = .Answered,
        } },
        .{ .reply = "ok" },
        .{ .note = "n" },
        .{ .ended = .{ .at = 3, .ending = .Done, .took_s = 4 } },
        .{ .resolved = .{ .at = null, .verdict = .Approved, .what = "w" } },
    };
    const live = Live{
        .waiting = .{ .what = "approve?", .more = 1 },
        .working = .{ .resident = "mei", .since = 5 },
        .calling = &.{.{ .name = "c", .subject = "s" }},
        .asking = 7,
        .composer = .{
            .typed = "/h",
            .cursor = 2,
            .placeholder = "…",
            .room = "room",
            .offer = "tab",
        },
        .menu = .{
            .shown = &.{.{ .spelling = "/help", .takes = "", .summary = "help" }},
            .chosen = 0,
            .more = 0,
        },
    };
    var built = try Scene.inlineFrame(gpa, .{
        .columns = 80,
        .rows = 24,
        .erase = 3,
        .previous = .Reply,
        .entries = &entries,
        .live = live,
    });
    defer built.deinit(gpa);

    var r: scene.Reader = .{ .bytes = built.bytes.items };
    try std.testing.expectEqual(part.Part.Inline, try r.tag());
    try std.testing.expectEqual(@as(u16, 80), try r.short());
    try std.testing.expectEqual(@as(u16, 24), try r.short());
    try std.testing.expectEqual(@as(u16, 3), try r.short());
    try std.testing.expectEqual(@as(u8, @intFromEnum(part.Part.Reply)), try r.byte());
    try std.testing.expectEqual(@as(u32, entries.len), try r.word());

    try std.testing.expectEqual(part.Part.Banner, try r.tag());
    try std.testing.expectEqualStrings("city", try r.text());
    try std.testing.expectEqualStrings("addr", try r.text());
    try std.testing.expectEqualStrings("https://x", try r.text());

    try std.testing.expectEqual(part.Part.You, try r.tag());
    try std.testing.expectEqual(@as(u32, 3600), try r.word());
    try std.testing.expectEqualStrings("hi", try r.text());

    try std.testing.expectEqual(part.Part.Head, try r.tag());
    try std.testing.expectEqual(scene.no_time, try r.word());
    try std.testing.expectEqualStrings("mei", try r.text());
    try std.testing.expectEqual(@as(u32, 1), try r.word());
    try std.testing.expectEqualStrings("a", try r.text());

    try std.testing.expectEqual(part.Part.Reasoning, try r.tag());
    try std.testing.expectEqual(@as(u64, 12), try r.long());

    try std.testing.expectEqual(part.Part.Tool, try r.tag());
    try std.testing.expectEqual(@as(u32, 2), try r.word());
    try std.testing.expectEqualStrings("run", try r.text());
    try std.testing.expectEqualStrings("t", try r.text());
    try std.testing.expectEqual(@as(u64, 9), try r.long());
    try std.testing.expectEqual(part.Outcome.Answered, try r.outcome());

    try std.testing.expectEqual(part.Part.Reply, try r.tag());
    try std.testing.expectEqualStrings("ok", try r.text());
    try std.testing.expectEqual(part.Part.Note, try r.tag());
    try std.testing.expectEqualStrings("n", try r.text());

    try std.testing.expectEqual(part.Part.Ended, try r.tag());
    try std.testing.expectEqual(@as(u32, 3), try r.word());
    try std.testing.expectEqual(part.Ending.Done, try r.ending());
    try std.testing.expectEqual(@as(u64, 4), try r.long());

    try std.testing.expectEqual(part.Part.Resolved, try r.tag());
    try std.testing.expectEqual(scene.no_time, try r.word());
    try std.testing.expectEqual(part.Verdict.Approved, try r.verdict());
    try std.testing.expectEqualStrings("w", try r.text());

    try std.testing.expectEqual(part.Part.Waiting, try r.tag());
    try std.testing.expectEqualStrings("approve?", try r.text());
    try std.testing.expectEqual(@as(u32, 1), try r.word());

    try std.testing.expectEqual(part.Part.Working, try r.tag());
    try std.testing.expectEqualStrings("mei", try r.text());
    try std.testing.expectEqual(@as(u32, 5), try r.word());

    try std.testing.expectEqual(part.Part.Calling, try r.tag());
    try std.testing.expectEqualStrings("c", try r.text());
    try std.testing.expectEqualStrings("s", try r.text());

    try std.testing.expectEqual(part.Part.Asking, try r.tag());
    try std.testing.expectEqual(@as(u64, 7), try r.long());

    try std.testing.expectEqual(part.Part.Composer, try r.tag());
    try std.testing.expectEqualStrings("/h", try r.text());
    try std.testing.expectEqual(@as(u32, 2), try r.word());
    try std.testing.expectEqualStrings("…", try r.text());
    try std.testing.expectEqualStrings("room", try r.text());
    try std.testing.expectEqualStrings("tab", try r.text());

    try std.testing.expectEqual(part.Part.Menu, try r.tag());
    try std.testing.expectEqual(@as(u32, 0), try r.word());
    try std.testing.expectEqual(@as(u32, 0), try r.word());
    try std.testing.expectEqual(@as(u32, 1), try r.word());
    try std.testing.expectEqualStrings("/help", try r.text());
    try std.testing.expectEqualStrings("", try r.text());
    try std.testing.expectEqualStrings("help", try r.text());
    try std.testing.expect(r.done());
}

test "quiet scene round-trips tags and fields" {
    const gpa = std.testing.allocator;
    var built = try Scene.quietFrame(gpa, .{
        .columns = 40,
        .rows = 12,
        .url = "http://127.0.0.1",
        .code = "ABCD",
        .key = "k",
        .transient = "wait",
    });
    defer built.deinit(gpa);

    var r: scene.Reader = .{ .bytes = built.bytes.items };
    try std.testing.expectEqual(part.Part.Quiet, try r.tag());
    try std.testing.expectEqual(@as(u16, 40), try r.short());
    try std.testing.expectEqual(@as(u16, 12), try r.short());
    try std.testing.expectEqualStrings("http://127.0.0.1", try r.text());
    try std.testing.expectEqualStrings("ABCD", try r.text());
    try std.testing.expectEqualStrings("k", try r.text());
    try std.testing.expectEqualStrings("wait", try r.text());
    try std.testing.expect(r.done());
}

test "caller-owned writer matches allocator scene" {
    const gpa = std.testing.allocator;
    const frame = Quiet{
        .columns = 8,
        .rows = 4,
        .url = "u",
        .code = "c",
    };
    var list_scene = try Scene.quietFrame(gpa, frame);
    defer list_scene.deinit(gpa);

    var aw: std.Io.Writer.Allocating = .init(gpa);
    defer aw.deinit();
    try writeQuiet(&aw.writer, frame);
    try std.testing.expectEqualSlices(u8, list_scene.bytes.items, aw.writer.buffered());
}
