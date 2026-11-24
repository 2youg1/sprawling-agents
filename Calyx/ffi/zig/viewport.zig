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
pub const Point = paint.Point;
pub const Selection = paint.Selection;

const Error = transcript.Error || std.mem.Allocator.Error;

const Anchor = struct { entry: usize, line: usize };

pub const Completion = struct { first: u16, last: u16, tone: u8 };
pub const settled_tool_tone = paint.settled_tool_tone;

pub const View = struct {
    anchor: ?Anchor = null,
    movement: i32 = 0,
    body_top: usize = 0,
    body_rows: usize = 0,
    working_tone: u8 = 117,
    working_mark: ?Point = null,
    completion: ?Completion = null,
    selection: ?Selection = null,
    copy: ?*std.Io.Writer = null,

    pub fn scroll(self: *View, lines: i32) void {
        self.movement +|= lines;
    }

    pub fn follow(self: *View) void {
        self.anchor = null;
        self.movement = 0;
    }

    fn first(self: *View, offsets: []const usize, budget: usize) usize {
        const last = offsets[offsets.len - 1] -| budget;
        const previous = if (self.anchor) |anchor| blk: {
            if (anchor.entry + 1 >= offsets.len) break :blk last;
            const lo = offsets[anchor.entry];
            const hi = offsets[anchor.entry + 1];
            break :blk @min(last, lo + @min(anchor.line, hi -| (lo + 1)));
        } else last;
        const moved = if (self.movement < 0)
            previous -| @as(usize, @abs(self.movement))
        else
            @min(last, previous +| @as(usize, @intCast(self.movement)));
        self.movement = 0;
        self.anchor = null;
        if (moved < last) {
            var entry: usize = 0;
            while (entry + 1 < offsets.len and offsets[entry + 1] <= moved) : (entry += 1) {}
            self.anchor = .{ .entry = entry, .line = moved - offsets[entry] };
        }
        return moved;
    }
};

const Clip = struct {
    inner: *std.Io.Writer,
    skip: usize,
    take: usize,
    seen: usize = 0,
    writer: std.Io.Writer,

    fn init(inner: *std.Io.Writer, skip: usize, take: usize) Clip {
        return .{
            .inner = inner,
            .skip = skip,
            .take = take,
            .writer = .{
                .buffer = &.{},
                .vtable = &.{ .drain = drain },
            },
        };
    }

    fn feed(self: *Clip, bytes: []const u8) std.Io.Writer.Error!void {
        var rest = bytes;
        while (rest.len > 0 and self.seen < self.skip + self.take) {
            const newline = std.mem.indexOfScalar(u8, rest, '\n');
            const end = if (newline) |at| at + 1 else rest.len;
            if (self.seen >= self.skip) try self.inner.writeAll(rest[0..end]);
            rest = rest[end..];
            if (newline != null) self.seen += 1;
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
    view: *View,
    hits: *std.ArrayList(Hit),
) Error![]u8 {
    hits.clearRetainingCapacity();
    view.working_mark = null;
    view.body_top = 0;
    view.body_rows = 0;
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
    const offsets = try gpa.alloc(usize, body + 1);
    defer gpa.free(offsets);
    const content_rows = try countBody(&r, body, if (banner) .Banner else null, cols, offsets);
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
        var p = paint.Painter.meiFrame(&aw.writer, cols);
        p.working_tone = view.working_tone;
        const cursor = try live.regionShown(&p, &live_reader, Grid.of(&p), .{ .skip = text_skip, .rows = text_rows });
        if (p.working_mark) |mark| if (mark.row < window) {
            view.working_mark = mark;
        };
        try place(&aw.writer, &p, cursor);
        try aw.writer.writeAll("\x1b[?25h\x1b[?2026l");
        return try aw.toOwnedSlice();
    }
    const budget = window - live_h;
    const banner_rows: usize = if (banner and budget >= 1) 1 else 0;
    const content_budget = budget - banner_rows;
    const skip = view.first(offsets, content_budget);
    const visible = @min(content_rows - skip, content_budget);
    view.body_top = banner_rows;
    view.body_rows = visible;
    r = saved;
    if (banner) {
        if (banner_rows == 1) {
            var p = paint.Painter.meiFrame(&aw.writer, cols);
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
    var clip = Clip.init(&aw.writer, skip, visible);
    var body_painter = paint.Painter.meiFrame(&clip.writer, cols);
    if (view.selection) |selection| {
        if (visible > 0) {
            const first = selectedPoint(selection.first, skip, banner_rows, visible, cols);
            const last = selectedPoint(selection.last, skip, banner_rows, visible, cols);
            body_painter.selection = Selection.between(first, last);
            body_painter.copy = view.copy;
        }
    }
    try paintBody(&body_painter, &r, body, if (banner) .Banner else null, &.{}, .{
        .marks = marks[@min(marks.len, @intFromBool(banner))..],
        .hits = hits,
        .gpa = gpa,
        .skip = skip,
        .take = visible,
        .origin = banner_rows,
        .completion = view.completion,
    });
    const used = banner_rows + visible;
    const top = @max(used, window - live_h);
    if (top > used) try aw.writer.print("\x1b[{d};1H", .{top + 1});
    var lp = paint.Painter.meiFrame(&aw.writer, cols);
    lp.working_tone = view.working_tone;
    lp.row = top;
    const cursor = try live.region(&lp, &r, Grid.of(&lp));
    view.working_mark = lp.working_mark;
    try place(&aw.writer, &lp, cursor);
    try aw.writer.writeAll("\x1b[?25h\x1b[?2026l");
    return try aw.toOwnedSlice();
}

pub fn workingPatch(out: *std.Io.Writer, mark: Point, tone: u8) paint.Error!void {
    try out.print("\x1b[?2026h\x1b7\x1b[{d};{d}H", .{ mark.row + 1, mark.column + 1 });
    try paint.writeGray(out, tone);
    try out.writeAll(paint.working_glyph ++ "\x1b[39m\x1b8\x1b[?2026l");
}

test "updating the working point changes just one cell and restores the cursor" {
    var bytes: [128]u8 = undefined;
    var writer: std.Io.Writer = .fixed(&bytes);
    try workingPatch(&writer, .{ .row = 19, .column = 10 }, 142);
    try std.testing.expectEqualStrings("\x1b[?2026h\x1b7\x1b[20;11H\x1b[38;2;142;142;142m●\x1b[39m\x1b8\x1b[?2026l", writer.buffered());
}

fn selectedPoint(point: Point, skip: usize, origin: usize, visible: usize, columns: usize) Point {
    if (point.row < origin) return .{ .row = skip, .column = 0 };
    if (point.row >= origin + visible) return .{ .row = skip + visible - 1, .column = columns };
    return .{ .row = skip + point.row - origin, .column = point.column };
}

fn countBody(r: *scene.Reader, n: usize, previous: ?part.Part, columns: usize, offsets: []usize) Error!usize {
    var hole: [1]u8 = undefined;
    var sink: std.Io.Writer.Discarding = .init(&hole);
    var p = paint.Painter.meiFrame(&sink.writer, columns);
    try paintBody(&p, r, n, previous, offsets, null);
    return p.row;
}

fn liveHeight(r: scene.Reader, columns: usize) Error!usize {
    var reader = r;
    if (reader.done()) return 0;
    var hole: [1]u8 = undefined;
    var sink: std.Io.Writer.Discarding = .init(&hole);
    var p = paint.Painter.meiFrame(&sink.writer, columns);
    _ = try live.region(&p, &reader, Grid.of(&p));
    return p.row + 1;
}

const BodyWindow = struct {
    marks: []const u16,
    hits: *std.ArrayList(Hit),
    gpa: std.mem.Allocator,
    skip: usize,
    take: usize,
    origin: usize,
    completion: ?Completion,
};

fn paintBody(
    p: *paint.Painter,
    r: *scene.Reader,
    n: usize,
    previous: ?part.Part,
    offsets: []usize,
    window: ?BodyWindow,
) Error!void {
    var prev = previous;
    var i: usize = 0;
    while (i < n) : (i += 1) {
        if (offsets.len != 0) offsets[i] = p.row;
        const tag = try r.tag();
        if (transcript.spaced(prev, tag)) try p.end();
        const before = p.row;
        p.completion_tone = if (tag == .Tool and window != null) blk: {
            const vis = window.?;
            const arriving = vis.completion orelse break :blk null;
            if (i >= vis.marks.len or vis.marks[i] < arriving.first or vis.marks[i] > arriving.last) break :blk null;
            break :blk arriving.tone;
        } else null;
        try transcript.entry(p, r, tag, Grid.of(p));
        const after = p.row;
        if (window) |vis| {
            const lower = @max(before, vis.skip);
            const upper = @min(after, vis.skip + vis.take);
            if (i < vis.marks.len and vis.marks[i] != no_card and lower < upper) {
                try vis.hits.append(vis.gpa, .{
                    .row = @intCast(vis.origin + lower - vis.skip),
                    .end = @intCast(vis.origin + upper - vis.skip),
                    .card = vis.marks[i],
                });
            }
        }
        prev = tag;
    }
    if (offsets.len != 0) offsets[n] = p.row;
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
    var view: View = .{};
    return draw(gpa, entries, &.{}, live_region, columns, rows, web, &view, &hits);
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

test "wide Mei bodies extend beyond the City measure" {
    const gpa = std.testing.allocator;
    var body: [330]u8 = undefined;
    for (0..110) |i| @memcpy(body[i * 3 ..][0..3], "测");
    const entries = [_]write.Entry{.{ .reply = &body }};
    const before = try frameOf(gpa, &entries, 120, 24, false);
    defer gpa.free(before);
    const after = try frameOf(gpa, &entries, 160, 24, false);
    defer gpa.free(after);
    try std.testing.expect(std.mem.indexOf(u8, before, body[0..180]) == null);
    try std.testing.expect(std.mem.indexOf(u8, after, body[0..180]) != null);
}

test "the reading anchor holds when new entries arrive and can follow again" {
    var view: View = .{};
    const before = [_]usize{ 0, 4, 8, 12, 16 };
    try std.testing.expectEqual(@as(usize, 10), view.first(&before, 6));
    view.scroll(-5);
    try std.testing.expectEqual(@as(usize, 5), view.first(&before, 6));
    try std.testing.expectEqual(@as(usize, 5), view.first(&.{ 0, 4, 8, 12, 16, 20 }, 6));
    try std.testing.expectEqual(@as(usize, 4), view.first(&.{ 0, 3, 7, 10, 13, 16 }, 6));
    view.follow();
    try std.testing.expectEqual(@as(usize, 10), view.first(&.{ 0, 3, 7, 10, 13, 16 }, 6));
}

test "copying from the visible transcript rejoins wrapped words and retains hard breaks" {
    const gpa = std.testing.allocator;
    const entries = [_]write.Entry{.{ .reply = "one two three four\nhard break" }};
    const live_region: write.Live = .{ .composer = .{ .typed = "", .cursor = 0, .placeholder = "Mei", .room = "Mei", .offer = "" } };
    var hits: std.ArrayList(Hit) = .empty;
    defer hits.deinit(gpa);
    var copied: std.Io.Writer.Allocating = .init(gpa);
    defer copied.deinit();
    var view: View = .{
        .selection = Selection.between(.{ .row = 0, .column = 0 }, .{ .row = 20, .column = 0 }),
        .copy = &copied.writer,
    };
    const frame = try draw(gpa, &entries, &.{}, live_region, 13, 24, false, &view, &hits);
    defer gpa.free(frame);
    try std.testing.expectEqualStrings("one two three four\nhard break", copied.written());
    try std.testing.expect(std.mem.indexOf(u8, frame, "\x1b[7m") != null);
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
        var view: View = .{};
        const bytes = try draw(gpa, &.{}, &.{}, region, 80, 24, false, &view, &hits);
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
    var view: View = .{};
    const bytes = try draw(gpa, &entries, &marks, region, 80, 24, false, &view, &hits);
    defer gpa.free(bytes);
    try std.testing.expect(hits.items.len >= 1);
    try std.testing.expectEqual(@as(u16, 3), hits.items[0].card);
    try std.testing.expect(hits.items[0].end > hits.items[0].row);
}
