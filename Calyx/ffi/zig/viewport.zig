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
const layout = @import("layout.zig");
const Grid = @import("grid.zig").Grid;

pub const Hit = struct { row: u16, end: u16, card: u16 };

pub const no_card: u16 = std.math.maxInt(u16);
pub const Item = struct { origin: layout.Origin, card: u16 = no_card };
pub const Point = paint.Point;
pub const Location = layout.Location;
pub const Selection = layout.Range;

const Error = transcript.Error || std.mem.Allocator.Error;

pub const Completion = struct { first: u16, last: u16, tone: u8 };
pub const settled_tool_tone = paint.settled_tool_tone;

pub const View = struct {
    anchor: ?Location = null,
    movement: i32 = 0,
    body_top: usize = 0,
    visible: std.ArrayList(layout.Row) = .empty,
    spans: ?std.heap.ArenaAllocator = null,
    working_tone: u8 = 117,
    working_mark: ?Point = null,
    completion: ?Completion = null,
    selection: ?Selection = null,
    copy: ?*std.Io.Writer = null,
    down: ?Location = null,
    draft_entry: ?usize = null,

    pub fn deinit(self: *View, gpa: std.mem.Allocator) void {
        self.visible.deinit(gpa);
        if (self.spans) |*spans| spans.deinit();
    }

    pub fn trackDraft(self: *View, entry: ?usize) void {
        if (self.draft_entry) |old| {
            if (entry != @as(?usize, old)) {
                self.anchor = remapDraft(self.anchor, old, entry);
                self.down = remapDraft(self.down, old, entry);
                if (self.selection) |selection| {
                    const start = remapDraft(selection.start, old, entry);
                    const finish = remapDraft(selection.finish, old, entry);
                    self.selection = if (start != null and finish != null) .{ .start = start.?, .finish = finish.? } else null;
                }
            }
        }
        self.draft_entry = entry;
    }

    pub fn commitDraft(self: *View, entry: usize) void {
        self.trackDraft(entry);
        self.draft_entry = null;
    }

    fn remapDraft(location_: ?Location, old: usize, new: ?usize) ?Location {
        var position = location_ orelse return null;
        if (position.origin.entry == old and position.origin.fragment == .main) {
            position.origin.entry = new orelse return null;
        }
        return position;
    }

    pub fn contains(self: *const View, row: usize) bool {
        return row >= self.body_top and row - self.body_top < self.visible.items.len;
    }

    pub fn location(self: *const View, point: Point) ?Location {
        const rows = self.visible.items;
        if (rows.len == 0) return null;
        if (point.row < self.body_top) return rows[0].at(0);
        const row = point.row - self.body_top;
        if (row >= rows.len) return rows[rows.len - 1].at(std.math.maxInt(usize));
        return rows[row].at(point.column);
    }

    pub fn scroll(self: *View, lines: i32) void {
        self.movement +|= lines;
    }

    pub fn follow(self: *View) void {
        self.anchor = null;
        self.movement = 0;
    }

    fn first(self: *View, rows: []const layout.Row, budget: usize) usize {
        const last = rows.len -| budget;
        const previous = if (self.anchor) |anchor| @min(last, layout.locate(rows, anchor, .forward).row) else last;
        if (self.movement == 0) return previous;
        const moved = if (self.movement < 0)
            previous -| @as(usize, @abs(self.movement))
        else
            @min(last, previous +| @as(usize, @intCast(self.movement)));
        self.movement = 0;
        self.anchor = if (moved < last) rows[moved].at(0) else null;
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
    items: []const Item,
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
    view.visible.clearRetainingCapacity();
    if (view.spans == null) view.spans = .init(gpa);
    _ = view.spans.?.reset(.retain_capacity);
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
    const body_items = items[@min(items.len, @intFromBool(banner))..];
    var index: layout.Index = .{ .gpa = gpa, .arena = view.spans.?.allocator() };
    defer index.deinit();
    try countBody(&r, body, if (banner) .Banner else null, cols, body_items, &index);
    const body_rows = index.rows.items;
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
    const skip = view.first(body_rows, content_budget);
    const visible = @min(body_rows.len - skip, content_budget);
    view.body_top = banner_rows;
    try view.visible.appendSlice(gpa, body_rows[skip..][0..visible]);
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
    if (view.selection) |range| {
        if (visible > 0 and !std.meta.eql(range.start, range.finish)) {
            const selection = range.ordered();
            if (layout.cellRange(body_rows, selection)) |cell| {
                body_painter.cell_selection = cell;
                body_painter.copy = view.copy;
            } else {
                const first = layout.locate(body_rows, selection.start, .forward);
                const last = layout.locate(body_rows, selection.finish, .backward);
                if (first.before(last) or last.before(first)) {
                    body_painter.selection = paint.Selection.between(first, last);
                    body_painter.copy = view.copy;
                }
            }
        }
    }
    try paintBody(&body_painter, &r, body, if (banner) .Banner else null, body_items, .{
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

fn countBody(r: *scene.Reader, n: usize, previous: ?part.Part, columns: usize, items: []const Item, index: *layout.Index) Error!void {
    var hole: [1]u8 = undefined;
    var sink: std.Io.Writer.Discarding = .init(&hole);
    var p = paint.Painter.meiFrame(&sink.writer, columns);
    p.layout_index = index;
    try paintBody(&p, r, n, previous, items, null);
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
    items: []const Item,
    window: ?BodyWindow,
) Error!void {
    var prev = previous;
    var i: usize = 0;
    while (i < n) : (i += 1) {
        const item = if (i < items.len) items[i] else Item{ .origin = .{ .entry = i } };
        if (p.layout_index) |index| index.begin(item.origin);
        const tag = try r.tag();
        if (transcript.spaced(prev, tag)) try p.end();
        const before = p.row;
        p.completion_tone = if (tag == .Tool and window != null) blk: {
            const vis = window.?;
            const arriving = vis.completion orelse break :blk null;
            if (item.card == no_card or item.card < arriving.first or item.card > arriving.last) break :blk null;
            break :blk arriving.tone;
        } else null;
        try transcript.entry(p, r, tag, Grid.of(p));
        const after = p.row;
        if (window) |vis| {
            const lower = @max(before, vis.skip);
            const upper = @min(after, vis.skip + vis.take);
            if (item.card != no_card and lower < upper) {
                try vis.hits.append(vis.gpa, .{
                    .row = @intCast(vis.origin + lower - vis.skip),
                    .end = @intCast(vis.origin + upper - vis.skip),
                    .card = item.card,
                });
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
    var view: View = .{};
    defer view.deinit(gpa);
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
    var rows: [20]layout.Row = undefined;
    for (&rows, 0..) |*row, n| row.* = .{ .start = .{ .origin = .{ .entry = n / 4 }, .line = n % 4 }, .indent = 0, .width = 5 };
    try std.testing.expectEqual(@as(usize, 10), view.first(rows[0..16], 6));
    view.scroll(-5);
    try std.testing.expectEqual(@as(usize, 5), view.first(rows[0..16], 6));
    try std.testing.expectEqual(@as(usize, 5), view.first(&rows, 6));
    view.follow();
    try std.testing.expectEqual(@as(usize, 14), view.first(&rows, 6));
}

test "copying from the visible transcript rejoins wrapped words and retains hard breaks" {
    const gpa = std.testing.allocator;
    const entries = [_]write.Entry{.{ .reply = "one two three four\nhard break" }};
    const live_region: write.Live = .{ .composer = .{ .typed = "", .cursor = 0, .placeholder = "Mei", .room = "Mei", .offer = "" } };
    var hits: std.ArrayList(Hit) = .empty;
    defer hits.deinit(gpa);
    var copied: std.Io.Writer.Allocating = .init(gpa);
    defer copied.deinit();
    var view: View = .{};
    defer view.deinit(gpa);
    const initial = try draw(gpa, &entries, &.{}, live_region, 13, 24, false, &view, &hits);
    defer gpa.free(initial);
    view.selection = .{
        .start = view.location(.{ .row = 0, .column = 0 }).?,
        .finish = view.location(.{ .row = 20, .column = 0 }).?,
    };
    view.copy = &copied.writer;
    const frame = try draw(gpa, &entries, &.{}, live_region, 13, 24, false, &view, &hits);
    defer gpa.free(frame);
    try std.testing.expectEqualStrings("one two three four\nhard break", copied.written());
    try std.testing.expect(std.mem.indexOf(u8, frame, "\x1b[7m") != null);
}

test "a drag keeps its original endpoint after the transcript follows new rows and rewraps" {
    const gpa = std.testing.allocator;
    const entries = [_]write.Entry{
        .{ .reply = "one two three four five six" },
        .{ .note = "new event" },
    };
    const region: write.Live = .{ .composer = .{ .typed = "", .cursor = 0, .placeholder = "Mei", .room = "Mei", .offer = "" } };
    var hits: std.ArrayList(Hit) = .empty;
    defer hits.deinit(gpa);
    var view: View = .{};
    defer view.deinit(gpa);
    const narrow = try draw(gpa, entries[0..1], &.{}, region, 13, 8, false, &view, &hits);
    defer gpa.free(narrow);
    const down = view.location(.{ .row = 1, .column = 2 }).?;
    const moved = try draw(gpa, &entries, &.{}, region, 23, 7, false, &view, &hits);
    defer gpa.free(moved);
    const up = view.location(.{ .row = 0, .column = 10 }).?;
    var copied: std.Io.Writer.Allocating = .init(gpa);
    defer copied.deinit();
    view.selection = .{ .start = down, .finish = up };
    view.copy = &copied.writer;
    const selected = try draw(gpa, &entries, &.{}, region, 23, 7, false, &view, &hits);
    defer gpa.free(selected);
    try std.testing.expectEqualStrings("three four five six", copied.written());
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
        defer view.deinit(gpa);
        const bytes = try draw(gpa, &.{}, &.{}, region, 80, 24, false, &view, &hits);
        defer gpa.free(bytes);
        var label = [_]u8{ 'L', '0' + @as(u8, @intCast(line / 10)), '0' + @as(u8, @intCast(line % 10)) };
        try std.testing.expect(std.mem.indexOf(u8, bytes, &label) != null);
        try std.testing.expect(std.mem.indexOf(u8, bytes, "Mei") != null);
        if (line == 0) try std.testing.expect(std.mem.indexOf(u8, bytes, "L39") == null);
        if (line == 39) try std.testing.expect(std.mem.indexOf(u8, bytes, "L00") == null);
    }
}

fn copiedAfterReflow(
    before: write.Entry,
    after: write.Entry,
    widths: struct { u16, u16 },
    first: Point,
    last: Point,
    expected: []const u8,
) !void {
    const gpa = std.testing.allocator;
    const region: write.Live = .{ .composer = .{ .typed = "", .cursor = 0, .placeholder = "Mei", .room = "Mei", .offer = "" } };
    var hits: std.ArrayList(Hit) = .empty;
    defer hits.deinit(gpa);
    var view: View = .{};
    defer view.deinit(gpa);
    const initial = try draw(gpa, &.{before}, &.{}, region, widths[0], 20, false, &view, &hits);
    defer gpa.free(initial);
    view.selection = .{ .start = view.location(first).?, .finish = view.location(last).? };
    var copied: std.Io.Writer.Allocating = .init(gpa);
    defer copied.deinit();
    view.copy = &copied.writer;
    const reflowed = try draw(gpa, &.{after}, &.{}, region, widths[1], 20, false, &view, &hits);
    defer gpa.free(reflowed);
    try std.testing.expectEqualStrings(expected, copied.written());
}

test "a growing reply keeps its selection when Markdown starts farther down" {
    const early = write.Entry{ .reply = "intro line is here\nalpha beta gamma" };
    const late = write.Entry{ .reply = "intro line is here\nalpha beta gamma\n# Topic" };
    try copiedAfterReflow(early, late, .{ 40, 40 }, .{ .row = 1, .column = 8 }, .{ .row = 1, .column = 12 }, "beta");
}

test "a prompt keeps its selected words when the time gutter appears" {
    for ([_]?u32{ 45, null }) |at| {
        const entry = write.Entry{ .you = .{ .at = at, .said = "alpha beta gamma" } };
        try copiedAfterReflow(entry, entry, .{ 59, 60 }, .{ .row = 0, .column = 8 }, .{ .row = 0, .column = 12 }, "beta");
    }
}

test "a right table cell keeps its selected characters when cells wrap" {
    const entry = write.Entry{ .reply = "| A | B |\n| --- | --- |\n| abcdef | uvwxyz |" };
    try copiedAfterReflow(entry, entry, .{ 19, 12 }, .{ .row = 2, .column = 11 }, .{ .row = 2, .column = 14 }, "uvw");
    try copiedAfterReflow(entry, entry, .{ 19, 12 }, .{ .row = 2, .column = 15 }, .{ .row = 2, .column = 11 }, "uvwx");
    try copiedAfterReflow(entry, entry, .{ 19, 9 }, .{ .row = 2, .column = 11 }, .{ .row = 2, .column = 14 }, "uvw");
}

test "the draft's logical positions follow the first committed reply past its heading" {
    var view: View = .{};
    view.trackDraft(4);
    const first: Location = .{ .origin = .{ .entry = 4 }, .line = 1, .offset = 8 };
    const last: Location = .{ .origin = .{ .entry = 4 }, .line = 1, .offset = 13 };
    view.anchor = first;
    view.down = first;
    view.selection = .{ .start = first, .finish = last };
    view.commitDraft(6);
    try std.testing.expectEqual(@as(usize, 6), view.anchor.?.origin.entry);
    try std.testing.expectEqual(@as(usize, 6), view.down.?.origin.entry);
    try std.testing.expectEqual(@as(usize, 6), view.selection.?.finish.origin.entry);
    try std.testing.expectEqual(@as(usize, 13), view.selection.?.finish.offset);
    view.trackDraft(null);
    try std.testing.expect(view.anchor != null);
}

test "a click hit names the visible python card" {
    const gpa = std.testing.allocator;
    const entries = [_]write.Entry{
        .{ .you = .{ .at = null, .said = "ask" } },
        .{ .tool = .{ .at = null, .name = "python", .subject = "time", .took_us = 1_500_000, .outcome = .Answered } },
    };
    const marks = [_]Item{ .{ .origin = .{ .entry = 0 } }, .{ .origin = .{ .entry = 1 }, .card = 3 } };
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
    defer view.deinit(gpa);
    const bytes = try draw(gpa, &entries, &marks, region, 80, 24, false, &view, &hits);
    defer gpa.free(bytes);
    try std.testing.expect(hits.items.len >= 1);
    try std.testing.expectEqual(@as(u16, 3), hits.items[0].card);
    try std.testing.expect(hits.items[0].end > hits.items[0].row);
}

test "a selection crossing a table column boundary copies in source order after wrapping" {
    const entry = write.Entry{ .reply = "| A | B |\n| --- | --- |\n| abcdef | uvwxyz |\n| ghijkl | stuvwx |" };
    try copiedAfterReflow(entry, entry, .{ 19, 12 }, .{ .row = 2, .column = 6 }, .{ .row = 2, .column = 13 }, "ef │ uv");
    try copiedAfterReflow(entry, entry, .{ 19, 12 }, .{ .row = 3, .column = 13 }, .{ .row = 2, .column = 6 }, "ef │ uvwxyz\nghijkl │ st");
}
