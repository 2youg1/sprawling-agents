// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

const std = @import("std");
const paint = @import("paint.zig");
const part = @import("part.zig");
const scene = @import("scene.zig");
const transcript = @import("transcript.zig");
const live = @import("live.zig");
const write = @import("write.zig");
const Grid = @import("grid.zig").Grid;

pub const Hit = struct { row: u16, end: u16, card: u16 };

pub const no_card: u16 = std.math.maxInt(u16);

const Error = transcript.Error || std.mem.Allocator.Error;

const Clip = struct {
    inner: *std.Io.Writer,
    skip: usize,
    seen: usize = 0,
    writer: std.Io.Writer,

    fn init(inner: *std.Io.Writer, skip: usize) Clip {
        return .{
            .inner = inner,
            .skip = skip,
            .writer = .{
                .buffer = &.{},
                .vtable = &.{ .drain = drain },
            },
        };
    }

    fn feed(self: *Clip, bytes: []const u8) std.Io.Writer.Error!void {
        if (self.seen >= self.skip) {
            try self.inner.writeAll(bytes);
            return;
        }
        var i: usize = 0;
        while (i < bytes.len) : (i += 1) {
            if (bytes[i] != '\n') continue;
            self.seen += 1;
            if (self.seen == self.skip) {
                try self.inner.writeAll(bytes[i + 1 ..]);
                return;
            }
        }
    }

    fn drain(w: *std.Io.Writer, data: []const []const u8, splat: usize) std.Io.Writer.Error!usize {
        const self: *Clip = @alignCast(@fieldParentPtr("writer", w));
        try self.feed(w.buffer[0..w.end]);
        w.end = 0;
        var total: usize = 0;
        if (data.len == 0) return 0;
        for (data[0 .. data.len - 1]) |bytes| {
            try self.feed(bytes);
            total += bytes.len;
        }
        const pattern = data[data.len - 1];
        var n: usize = 0;
        while (n < splat) : (n += 1) try self.feed(pattern);
        total += pattern.len * splat;
        return total;
    }
};

pub fn draw(
    gpa: std.mem.Allocator,
    entries: []const write.Entry,
    marks: []const u16,
    region: write.Live,
    columns: u16,
    rows: u16,
    show_web: bool,
    dim: bool,
    hits: *std.ArrayList(Hit),
) Error![]u8 {
    hits.clearRetainingCapacity();
    const cols: usize = @max(@as(usize, columns), 2);
    const window: usize = @max(@as(usize, rows), 1);
    var scene_bytes = try write.Scene.inlineFrame(gpa, .{
        .columns = columns,
        .rows = rows,
        .erase = 0,
        .previous = null,
        .entries = entries,
        .live = region,
    });
    defer scene_bytes.deinit(gpa);
    var r: scene.Reader = .{ .bytes = scene_bytes.bytes.items };
    if (try r.tag() != .Inline) return error.Malformed;
    _ = try r.short();
    _ = try r.short();
    _ = try r.short();
    _ = try r.byte();
    const n: usize = try r.word();
    const saved = r;
    const banner = n > 0 and (try r.tag()) == .Banner;
    r = saved;
    var aw: std.Io.Writer.Allocating = .init(gpa);
    errdefer aw.deinit();
    try aw.writer.writeAll("\x1b[?2026h\x1b[?25l\x1b[2J\x1b[H");
    if (banner) {
        var hole: [1]u8 = undefined;
        var sink: std.Io.Writer.Discarding = .init(&hole);
        var drop = paint.Painter.init(&sink.writer, cols);
        _ = try r.tag();
        try transcript.bannerChoice(&drop, &r, show_web);
        try drop.end();
    }
    const body = n - @intFromBool(banner);
    const content_rows = try countBody(&r, body, if (banner) .Banner else null, cols);
    const live_at = r;
    const live_h = try liveHeight(live_at, cols);
    if (live_h >= window) {
        const shape = live.typedShape(region.composer.typed, region.composer.cursor, cols);
        const non_text = live_h -| shape.rows;
        var text_rows = window -| non_text;
        if (text_rows == 0) text_rows = 1;
        var text_skip: usize = 0;
        if (shape.rows > text_rows) {
            if (shape.cursor + 1 > text_rows) text_skip = shape.cursor + 1 - text_rows;
            const max_skip = shape.rows - text_rows;
            if (text_skip > max_skip) text_skip = max_skip;
        }
        var live_reader = live_at;
        var p = paint.Painter.init(&aw.writer, cols);
        const cursor = try live.regionShown(&p, &live_reader, Grid.of(&p), .{ .skip = text_skip, .rows = text_rows });
        try place(&aw.writer, &p, cursor);
        try aw.writer.writeAll("\x1b[?25h\x1b[?2026l");
        return try aw.toOwnedSlice();
    }
    const budget = window - live_h;
    const banner_rows: usize = if (banner and budget >= 1) 1 else 0;
    const content_budget = budget - banner_rows;
    const visible = if (dim) 0 else @min(content_rows, content_budget);
    const skip = content_rows - visible;
    r = saved;
    if (banner) {
        if (banner_rows == 1) {
            var p = paint.Painter.init(&aw.writer, cols);
            _ = try r.tag();
            try transcript.bannerChoice(&p, &r, show_web);
            try p.end();
        } else {
            var hole: [1]u8 = undefined;
            var sink: std.Io.Writer.Discarding = .init(&hole);
            var drop = paint.Painter.init(&sink.writer, cols);
            _ = try r.tag();
            try transcript.bannerChoice(&drop, &r, show_web);
            try drop.end();
        }
    }
    var clip = Clip.init(&aw.writer, skip);
    var body_painter = paint.Painter.init(&clip.writer, cols);
    try paintBody(&body_painter, &r, body, if (banner) .Banner else null, marks, @intFromBool(banner), hits, gpa, skip, banner_rows);
    const used = banner_rows + visible;
    const top = @max(used, window - live_h);
    if (top > used) try aw.writer.print("\x1b[{d};1H", .{top + 1});
    var lp = paint.Painter.init(&aw.writer, cols);
    lp.row = top;
    const cursor = try live.region(&lp, &r, Grid.of(&lp));
    try place(&aw.writer, &lp, cursor);
    try aw.writer.writeAll("\x1b[?25h\x1b[?2026l");
    return try aw.toOwnedSlice();
}

fn countBody(r: *scene.Reader, n: usize, previous: ?part.Part, columns: usize) Error!usize {
    var hole: [1]u8 = undefined;
    var sink: std.Io.Writer.Discarding = .init(&hole);
    var p = paint.Painter.init(&sink.writer, columns);
    try paintBody(&p, r, n, previous, &.{}, 0, null, undefined, 0, 0);
    return p.row;
}

fn liveHeight(r: scene.Reader, columns: usize) Error!usize {
    var reader = r;
    if (reader.done()) return 0;
    var hole: [1]u8 = undefined;
    var sink: std.Io.Writer.Discarding = .init(&hole);
    var p = paint.Painter.init(&sink.writer, columns);
    _ = try live.region(&p, &reader, Grid.of(&p));
    return p.row + 1;
}

fn paintBody(
    p: *paint.Painter,
    r: *scene.Reader,
    n: usize,
    previous: ?part.Part,
    marks: []const u16,
    mark_at: usize,
    hits: ?*std.ArrayList(Hit),
    gpa: std.mem.Allocator,
    skip: usize,
    origin: usize,
) Error!void {
    var prev = previous;
    var i: usize = 0;
    while (i < n) : (i += 1) {
        const tag = try r.tag();
        if (transcript.spaced(prev, tag)) try p.end();
        const before = p.row;
        try transcript.entry(p, r, tag, Grid.of(p));
        const after = p.row;
        const mark_i = mark_at + i;
        if (hits) |list| {
            if (mark_i < marks.len and marks[mark_i] != no_card and after > skip) {
                const vis_lo = if (before > skip) before - skip else 0;
                const vis_hi = after - skip;
                list.append(gpa, .{
                    .row = std.math.cast(u16, origin + vis_lo) orelse std.math.maxInt(u16),
                    .end = std.math.cast(u16, origin + vis_hi) orelse std.math.maxInt(u16),
                    .card = marks[mark_i],
                }) catch {};
            }
        }
        prev = tag;
    }
}

fn place(out: *std.Io.Writer, p: *const paint.Painter, cursor: live.Cursor) paint.Error!void {
    const up = p.row -| cursor.row;
    if (up > 0) try out.print("\x1b[{d}A", .{up});
    try out.print("\x1b[{d}G", .{cursor.column + 1});
}

fn frameOf(
    gpa: std.mem.Allocator,
    entries: []const write.Entry,
    columns: u16,
    rows: u16,
    web: bool,
) ![]u8 {
    var hits: std.ArrayList(Hit) = .empty;
    defer hits.deinit(gpa);
    const live_region: write.Live = .{
        .composer = .{
            .typed = "",
            .cursor = 0,
            .placeholder = "to Mei…",
            .room = "Mei",
            .offer = "model",
        },
    };
    return draw(gpa, entries, &.{}, live_region, columns, rows, web, false, &hits);
}

test "mei viewport clears, anchors the composer, and omits the web hint" {
    const gpa = std.testing.allocator;
    const entries = [_]write.Entry{
        .{ .banner = .{ .city = "~/work", .address = "127.0.0.1:3425", .url = "http://127.0.0.1:3425" } },
        .{ .you = .{ .at = 100, .said = "你好" } },
    };
    const bytes = try frameOf(gpa, &entries, 80, 24, false);
    defer gpa.free(bytes);
    try std.testing.expect(std.mem.indexOf(u8, bytes, "\x1b[2J") != null);
    try std.testing.expect(std.mem.indexOf(u8, bytes, "你好") != null);
    try std.testing.expect(std.mem.indexOf(u8, bytes, "to Mei…") != null);
    try std.testing.expect(std.mem.indexOf(u8, bytes, "/web") == null);
    try std.testing.expect(std.mem.indexOf(u8, bytes, "\x1b[21;1H") != null);
}

test "the city hint stays available to a viewport that asks for it" {
    const gpa = std.testing.allocator;
    const entries = [_]write.Entry{
        .{ .banner = .{ .city = "~/work", .address = "127.0.0.1:3425", .url = "http://127.0.0.1:3425" } },
    };
    const bytes = try frameOf(gpa, &entries, 80, 24, true);
    defer gpa.free(bytes);
    try std.testing.expect(std.mem.indexOf(u8, bytes, "/web") != null);
    try std.testing.expect(std.mem.indexOf(u8, bytes, "to open the WebUI") != null);
}

test "a long transcript keeps its tail and the composer" {
    const gpa = std.testing.allocator;
    var storage: [30][8]u8 = undefined;
    var entries: [30]write.Entry = undefined;
    for (0..30) |i| {
        const text = std.fmt.bufPrint(&storage[i], "L{d:0>2}!", .{i}) catch unreachable;
        entries[i] = .{ .note = text };
    }
    const bytes = try frameOf(gpa, &entries, 80, 12, false);
    defer gpa.free(bytes);
    try std.testing.expect(std.mem.indexOf(u8, bytes, "L00!") == null);
    try std.testing.expect(std.mem.indexOf(u8, bytes, "L21!") == null);
    try std.testing.expect(std.mem.indexOf(u8, bytes, "L22!") != null);
    try std.testing.expect(std.mem.indexOf(u8, bytes, "L29!") != null);
    try std.testing.expect(std.mem.indexOf(u8, bytes, "to Mei…") != null);
}

test "narrow and wide windows wrap CJK on different rows" {
    const gpa = std.testing.allocator;
    const entries = [_]write.Entry{
        .{ .reply = "中文中文中文中文中文中文中文中文中文中文" },
    };
    const narrow = try frameOf(gpa, &entries, 12, 24, false);
    defer gpa.free(narrow);
    const wide = try frameOf(gpa, &entries, 80, 24, false);
    defer gpa.free(wide);
    try std.testing.expect(std.mem.count(u8, narrow, "\n") > std.mem.count(u8, wide, "\n"));
    try std.testing.expect(std.mem.indexOf(u8, narrow, "中文") != null);
}

test "a tall composer keeps the cursor line and the status row" {
    const gpa = std.testing.allocator;
    var storage: [40 * 4]u8 = undefined;
    var at: usize = 0;
    for (0..40) |i| {
        if (i > 0) {
            storage[at] = '\n';
            at += 1;
        }
        storage[at] = 'L';
        storage[at + 1] = '0' + @as(u8, @intCast(i / 10));
        storage[at + 2] = '0' + @as(u8, @intCast(i % 10));
        at += 3;
    }
    const typed = storage[0..at];
    const cases = [_]usize{ 0, 20, 39 };
    for (cases) |line| {
        var seen_lines: usize = 0;
        var cursor: usize = 0;
        while (cursor < typed.len and seen_lines < line) : (cursor += 1) {
            if (typed[cursor] == '\n') seen_lines += 1;
        }
        if (cursor < typed.len and typed[cursor] == '\n') cursor += 1;
        var hits: std.ArrayList(Hit) = .empty;
        defer hits.deinit(gpa);
        const region: write.Live = .{
            .composer = .{
                .typed = typed,
                .cursor = cursor,
                .placeholder = "to Mei…",
                .room = "Mei",
                .offer = "model",
            },
        };
        const bytes = try draw(gpa, &.{}, &.{}, region, 80, 24, false, false, &hits);
        defer gpa.free(bytes);
        var label = [_]u8{ 'L', '0' + @as(u8, @intCast(line / 10)), '0' + @as(u8, @intCast(line % 10)) };
        try std.testing.expect(std.mem.indexOf(u8, bytes, &label) != null);
        try std.testing.expect(std.mem.indexOf(u8, bytes, "Mei") != null);
        if (line == 0) try std.testing.expect(std.mem.indexOf(u8, bytes, "L39") == null);
        if (line == 39) try std.testing.expect(std.mem.indexOf(u8, bytes, "L00") == null);
    }
}

test "a click hit names the visible python card" {
    const gpa = std.testing.allocator;
    const entries = [_]write.Entry{
        .{ .you = .{ .at = null, .said = "ask" } },
        .{ .tool = .{ .at = null, .name = "python", .subject = "time", .took_us = 1_500_000, .outcome = .Answered } },
    };
    const marks = [_]u16{ no_card, 3 };
    var hits: std.ArrayList(Hit) = .empty;
    defer hits.deinit(gpa);
    const region: write.Live = .{
        .composer = .{
            .typed = "",
            .cursor = 0,
            .placeholder = "to Mei…",
            .room = "Mei",
            .offer = "",
        },
    };
    const bytes = try draw(gpa, &entries, &marks, region, 80, 24, false, false, &hits);
    defer gpa.free(bytes);
    try std.testing.expect(hits.items.len >= 1);
    try std.testing.expectEqual(@as(u16, 3), hits.items[0].card);
    try std.testing.expect(hits.items[0].end > hits.items[0].row);
}
