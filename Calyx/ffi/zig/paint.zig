// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

const std = @import("std");
const text = @import("text.zig");
const layout = @import("layout.zig");

pub const Style = enum {
    plain,
    faint,
    bold,
    accent,
    alert,
    alert_bold,

    fn sgr(self: Style) []const u8 {
        return switch (self) {
            .plain => "\x1b[0m",
            .faint => "\x1b[0;2m",
            .bold => "\x1b[0;1m",
            .accent => "\x1b[0;34m",
            .alert => "\x1b[0;33m",
            .alert_bold => "\x1b[0;1;33m",
        };
    }
};

pub const Error = std.Io.Writer.Error;
pub const working_glyph = "●";
pub const settled_tool_tone = 172;

pub fn writeGray(out: *std.Io.Writer, tone: u8) Error!void {
    try out.print("\x1b[38;2;{d};{d};{d}m", .{ tone, tone, tone });
}

pub const Point = layout.Point;

pub const Selection = struct {
    first: Point,
    last: Point,

    pub fn between(a: Point, b: Point) Selection {
        return if (b.before(a)) .{ .first = b, .last = a } else .{ .first = a, .last = b };
    }

    fn contains(self: Selection, row: usize, column: usize, width: usize) bool {
        const start: Point = .{ .row = row, .column = column };
        const end: Point = .{ .row = row, .column = column + @max(width, 1) };
        return self.first.before(end) and start.before(self.last);
    }
};

pub const Painter = struct {
    out: *std.Io.Writer,
    limit: usize,
    column: usize = 0,
    row: usize = 0,
    style: Style = .plain,
    mei: bool = false,
    working_tone: u8 = 117,
    working_mark: ?Point = null,
    completion_tone: ?u8 = null,
    selection: ?Selection = null,
    copy: ?*std.Io.Writer = null,
    reversed: bool = false,
    selected_base: bool = false,
    layout_index: ?*layout.Index = null,

    pub fn init(out: *std.Io.Writer, terminal_width: usize) Painter {
        return .{ .out = out, .limit = @max(terminal_width, 2) - 1 };
    }

    pub fn meiFrame(out: *std.Io.Writer, terminal_width: usize) Painter {
        var painter = init(out, terminal_width);
        painter.mei = true;
        return painter;
    }

    pub fn room(self: *const Painter) usize {
        return self.limit -| self.column;
    }

    pub fn raw(self: *Painter, bytes: []const u8) Error!void {
        try self.out.writeAll(bytes);
    }

    pub fn sourceLine(self: *Painter, body: []const u8, line: []const u8) void {
        if (self.layout_index) |index| {
            index.start.line = 1 + @intFromPtr(line.ptr) - @intFromPtr(body.ptr);
            index.start.offset = 0;
        }
    }

    fn styled(self: *Painter, style: Style) Error!void {
        const applied: Style = if (self.mei) switch (style) {
            .accent => .plain,
            .alert, .alert_bold => .bold,
            else => style,
        } else style;
        if (applied == self.style) return;
        try self.out.writeAll(applied.sgr());
        self.style = applied;
        self.reversed = false;
    }

    fn reverse(self: *Painter, on: bool) Error!void {
        if (on == self.reversed) return;
        try self.out.writeAll(if (on) "\x1b[7m" else "\x1b[27m");
        self.reversed = on;
    }

    pub fn char(self: *Painter, style: Style, c: text.Char) Error!bool {
        if (c.columns > self.room()) return false;
        try self.styled(style);
        const selected = if (c.columns == 0) self.selected_base else if (self.selection) |range| range.contains(self.row, self.column, c.columns) else false;
        self.selected_base = selected;
        try self.reverse(selected);
        if (selected) if (self.copy) |copy| try copy.writeAll(c.shown);
        try self.out.writeAll(c.shown);
        self.column += c.columns;
        return true;
    }

    pub fn fit(self: *Painter, style: Style, s: []const u8, max: usize) Error!void {
        const allowed = @min(max, self.room());
        if (text.width(s) <= allowed) return self.cut(style, s, allowed);
        if (allowed == 0) return;
        try self.cut(style, s, allowed - 1);
        _ = try self.char(style, text.char("…", 0));
    }

    pub fn grayMark(self: *Painter, mark: []const u8, tone: u8) Error!void {
        if (self.room() == 0) return;
        try self.styled(.plain);
        try writeGray(self.out, tone);
        try self.cut(.plain, mark, 1);
        try self.out.writeAll("\x1b[39m");
    }

    pub fn cut(self: *Painter, style: Style, s: []const u8, max: usize) Error!void {
        const stop = self.column + max;
        var at: usize = 0;
        while (at < s.len) {
            const c = text.char(s, at);
            if (self.column + c.columns > stop) break;
            _ = try self.char(style, c);
            at += c.len;
        }
    }

    pub fn to(self: *Painter, target: usize) Error!void {
        const stop = @min(target, self.limit);
        if (self.column >= stop) return;
        try self.styled(.plain);
        try self.reverse(false);
        self.selected_base = false;
        if (self.column == 0) {
            if (self.layout_index) |index| index.indent = stop;
        }
        if (self.column != 0) {
            if (self.selection) |range| {
                if (self.copy) |copy| {
                    const first = if (range.first.row == self.row) @max(self.column, range.first.column) else self.column;
                    const last = if (range.last.row == self.row) @min(stop, range.last.column) else stop;
                    if (range.first.row <= self.row and self.row <= range.last.row) try copy.splatByteAll(' ', last -| first);
                }
            }
        }
        try self.out.splatByteAll(' ', stop - self.column);
        self.column = stop;
    }

    pub fn end(self: *Painter) Error!void {
        try self.newline("\n");
    }

    pub fn wrap(self: *Painter, gap: bool) Error!void {
        try self.newline(if (gap) " " else "");
    }

    fn newline(self: *Painter, separation: []const u8) Error!void {
        if (self.layout_index) |index| index.line(self.column, separation) catch return error.WriteFailed;
        if (self.selection) |range| {
            if (range.first.row <= self.row and self.row < range.last.row) {
                if (self.copy) |copy| try copy.writeAll(separation);
            }
        }
        try self.styled(.plain);
        try self.reverse(false);
        try self.out.writeAll("\r\n");
        self.column = 0;
        self.row += 1;
        self.selected_base = false;
    }

    pub fn close(self: *Painter) Error!void {
        try self.styled(.plain);
        try self.reverse(false);
    }

    pub fn link(self: *Painter, style: Style, uri: []const u8, label: []const u8) Error!void {
        try self.out.writeAll("\x1b]8;;");
        var at: usize = 0;
        while (at < uri.len) {
            const c = text.char(uri, at);
            if (std.mem.eql(u8, c.shown, uri[at .. at + c.len])) {
                try self.out.writeAll(c.shown);
            }
            at += c.len;
        }
        try self.out.writeAll("\x1b\\");
        try self.fit(style, label, self.room());
        try self.out.writeAll("\x1b]8;;\x1b\\");
    }
};

test "selection copies combining marks with their base and includes a visible ellipsis" {
    var output: std.Io.Writer.Discarding = .init(&.{});
    var buffer: [16]u8 = undefined;
    var copy: std.Io.Writer = .fixed(&buffer);
    const cases = [_][]const u8{ "e\u{0301}", "x", "…" };
    for (cases, 0..) |wanted, col| {
        copy.end = 0;
        var p = Painter.meiFrame(&output.writer, 8);
        p.selection = Selection.between(.{ .row = 0, .column = col }, .{ .row = 0, .column = col + 1 });
        p.copy = &copy;
        try p.fit(.plain, "e\u{0301}xyz", 3);
        try std.testing.expectEqualStrings(wanted, copy.buffered());
    }
}

test "a fitted text never runs past its room" {
    var buffer: [256]u8 = undefined;
    var out: std.Io.Writer = .fixed(&buffer);
    var p = Painter.init(&out, 11);
    try p.fit(.plain, "a long subject", 6);
    try std.testing.expectEqual(@as(usize, 6), p.column);
    try std.testing.expect(std.mem.endsWith(u8, out.buffered(), "a lon…"));
}

test "link URI cannot terminate OSC or inject terminal commands" {
    var buffer: [256]u8 = undefined;
    var out: std.Io.Writer = .fixed(&buffer);
    var p = Painter.init(&out, 80);
    try p.link(.plain, "https://example/\x07\x1b\\\x1b[2J\u{009c}\x7f\xff中文", "open");
    try std.testing.expectEqualStrings("\x1b]8;;https://example/\\[2J中文\x1b\\open\x1b]8;;\x1b\\", out.buffered());
}
