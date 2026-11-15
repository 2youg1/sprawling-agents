// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// Running text laid into a column: words wrapped at the column's width,
// a wide character a place to break on its own, and, in a reply, the
// three marks a model leans on. Inside backticks the code stays as written and
// the backticks are drawn faint; `**` makes the words between bold and
// is not drawn; a fence line opens or closes a block whose lines keep
// their spacing. Every continued line starts at the column's left edge.

const std = @import("std");
const text = @import("text.zig");
const paint = @import("paint.zig");

/// How a text is read: a reply's marks and words; plain words; words
/// with their spacing kept, as a console's own table is written; or every
/// character as written, which is how a fenced block's lines are kept.
pub const Marks = enum { prose, plain, kept, verbatim };

pub const Column = struct {
    left: usize,
    measure: usize,
    style: paint.Style,
    marks: Marks,
};

/// Lays `body` into `column`, starting on the painter's current line at
/// `column.left`, and leaves the painter at the end of the last line,
/// unterminated.
pub fn flow(p: *paint.Painter, body: []const u8, column: Column) paint.Error!void {
    var fenced = false;
    var lines = std.mem.splitScalar(u8, body, '\n');
    var first = true;
    while (lines.next()) |raw| {
        const line = std.mem.trimEnd(u8, raw, "\r");
        if (!first) try p.end();
        first = false;
        try p.to(column.left);
        const fence = column.marks == .prose and std.mem.startsWith(u8, std.mem.trimStart(u8, line, " "), "```");
        if (fence) {
            fenced = !fenced;
            try p.fit(.faint, line, column.measure);
            continue;
        }
        const marks: Marks = if (fenced) .verbatim else column.marks;
        try words(p, line, .{ .left = column.left, .measure = column.measure, .style = column.style, .marks = marks });
    }
}

/// How a run of characters is drawn while marks are being read.
const Inline = struct {
    code: bool = false,
    strong: bool = false,

    fn style(self: Inline, base: paint.Style) paint.Style {
        if (self.code) return .plain;
        if (self.strong) return .bold;
        return base;
    }
};

/// One source line, wrapped word by word.
fn words(p: *paint.Painter, line: []const u8, column: Column) paint.Error!void {
    const right = column.left + column.measure;
    var state: Inline = .{};
    var at: usize = 0;
    var pending_space = false;
    while (at < line.len) {
        if (line[at] == ' ' and (column.marks == .prose or column.marks == .plain)) {
            pending_space = p.column > column.left;
            at += 1;
            continue;
        }
        const word_end = endOfWord(line, at, column.marks);
        const needed = measure(line[at..word_end], column.marks) + @intFromBool(pending_space);
        if (p.column + needed > @min(right, p.limit) and p.column > column.left) {
            try p.end();
            try p.to(column.left);
            pending_space = false;
        }
        if (pending_space) {
            try p.to(p.column + 1);
            pending_space = false;
        }
        at = try emit(p, line, .{ .from = at, .to = word_end }, column, &state);
    }
}

const Span = struct { from: usize, to: usize };

/// The end of the word starting at `at`: the next space in prose, the
/// next character after a wide one, the end of the line verbatim.
fn endOfWord(line: []const u8, at: usize, marks: Marks) usize {
    const first = text.char(line, at);
    if (first.columns == 2 or first.cp == ' ' or marks == .verbatim) return at + first.len;
    var end = at;
    while (end < line.len and line[end] != ' ') {
        const c = text.char(line, end);
        if (c.columns == 2) break;
        end += c.len;
    }
    return end;
}

/// Columns a word takes once its marks are read: `**` takes none.
fn measure(word: []const u8, marks: Marks) usize {
    var total: usize = 0;
    var at: usize = 0;
    while (at < word.len) {
        if (marks == .prose and std.mem.startsWith(u8, word[at..], "**")) {
            at += 2;
            continue;
        }
        const c = text.char(word, at);
        total += c.columns;
        at += c.len;
    }
    return total;
}

/// Draws one word, breaking it by force where it is longer than the
/// column, and answers where the next one starts.
fn emit(p: *paint.Painter, line: []const u8, span: Span, column: Column, state: *Inline) paint.Error!usize {
    var at = span.from;
    while (at < span.to) {
        if (column.marks == .prose and !state.code and std.mem.startsWith(u8, line[at..], "**")) {
            state.strong = !state.strong;
            at += 2;
            continue;
        }
        const c = text.char(line, at);
        const tick = column.marks == .prose and c.cp == '`';
        if (tick) state.code = !state.code;
        const style: paint.Style = if (tick) .faint else state.style(column.style);
        if (!try p.char(style, c)) {
            try p.end();
            try p.to(column.left);
            _ = try p.char(style, c);
        }
        at += c.len;
    }
    return at;
}

test "words wrap at the column and continue at its left edge" {
    var buffer: [512]u8 = undefined;
    var out: std.Io.Writer = .fixed(&buffer);
    var p = paint.Painter.init(&out, 13);
    try flow(&p, "one two three four", .{ .left = 2, .measure = 10, .style = .plain, .marks = .prose });
    try std.testing.expectEqual(@as(usize, 1), p.row);
}

test "the bold mark takes no column" {
    try std.testing.expectEqual(@as(usize, 4), measure("**ab**cd", .prose));
}
