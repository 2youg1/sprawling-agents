// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// The one writer of a frame's bytes: styled text by the column, and the
// handful of control sequences a frame uses. It counts the column and
// the row it is on, so no line it draws runs past `limit`, the last
// column a line may use; the terminal's own last column is never
// written, because some terminals wrap the moment it is filled.
//
// Colour comes only from the terminal's own palette - the default
// foreground, faint, bold, blue for ACCENT and yellow for ALERT - so a
// frame takes the person's theme, light or dark, as it is.

const std = @import("std");
const text = @import("text.zig");

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

pub const Painter = struct {
    out: *std.Io.Writer,
    /// The last column a line may reach, counted from one.
    limit: usize,
    column: usize = 0,
    row: usize = 0,
    style: Style = .plain,

    pub fn init(out: *std.Io.Writer, terminal_width: usize) Painter {
        return .{ .out = out, .limit = @max(terminal_width, 2) - 1 };
    }

    /// Room left on this line.
    pub fn room(self: *const Painter) usize {
        return self.limit -| self.column;
    }

    pub fn raw(self: *Painter, bytes: []const u8) Error!void {
        try self.out.writeAll(bytes);
    }

    fn styled(self: *Painter, style: Style) Error!void {
        if (style == self.style) return;
        try self.out.writeAll(style.sgr());
        self.style = style;
    }

    /// One character, when it fits on this line.
    pub fn char(self: *Painter, style: Style, c: text.Char) Error!bool {
        if (c.columns > self.room()) return false;
        try self.styled(style);
        try self.out.writeAll(c.shown);
        self.column += c.columns;
        return true;
    }

    /// A text on this line, cut with an ellipsis where it would run past
    /// `max` columns or the end of the line.
    pub fn fit(self: *Painter, style: Style, s: []const u8, max: usize) Error!void {
        const allowed = @min(max, self.room());
        if (text.width(s) <= allowed) return self.cut(style, s, allowed);
        if (allowed == 0) return;
        try self.cut(style, s, allowed - 1);
        try self.styled(style);
        try self.out.writeAll("…");
        self.column += 1;
    }

    /// As much of `s` as fits in `max` columns, no ellipsis: for a
    /// drawing, which loses nothing by stopping short.
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

    /// Spaces up to `target`, counted from zero; nothing when already past.
    pub fn to(self: *Painter, target: usize) Error!void {
        const stop = @min(target, self.limit);
        if (self.column >= stop) return;
        try self.styled(.plain);
        try self.out.splatByteAll(' ', stop - self.column);
        self.column = stop;
    }

    /// The end of a line: style reset, then carriage return and line feed.
    pub fn end(self: *Painter) Error!void {
        try self.styled(.plain);
        try self.out.writeAll("\r\n");
        self.column = 0;
        self.row += 1;
    }

    /// The end of the last line of a frame, which leaves the cursor on it.
    pub fn close(self: *Painter) Error!void {
        try self.styled(.plain);
    }

    /// A link a terminal may open (OSC 8) around `label`.
    pub fn link(self: *Painter, style: Style, uri: []const u8, label: []const u8) Error!void {
        try self.out.writeAll("\x1b]8;;");
        try self.out.writeAll(uri);
        try self.out.writeAll("\x1b\\");
        try self.fit(style, label, self.room());
        try self.out.writeAll("\x1b]8;;\x1b\\");
    }
};

test "a fitted text never runs past its room" {
    var buffer: [256]u8 = undefined;
    var out: std.Io.Writer = .fixed(&buffer);
    var p = Painter.init(&out, 11);
    try p.fit(.plain, "a long subject", 6);
    try std.testing.expectEqual(@as(usize, 6), p.column);
    try std.testing.expect(std.mem.endsWith(u8, out.buffered(), "a lon…"));
}
