// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

const std = @import("std");
const flow = @import("flow.zig");
const paint = @import("paint.zig");
const text = @import("text.zig");

const Lines = std.mem.SplitIterator(u8, .scalar);
const Column = struct { width: usize, alignment: enum { left, center, right } };
const gutter = 3;

const Cells = struct {
    rest: []const u8,

    fn of(line: []const u8) Cells {
        const body = std.mem.trim(u8, line, " \t\r");
        return .{ .rest = if (std.mem.startsWith(u8, body, "|")) body[1..] else body };
    }

    fn next(self: *Cells) ?[]const u8 {
        if (self.rest.len == 0) return null;
        var escaped = false;
        for (self.rest, 0..) |byte, i| {
            if (byte == '|' and !escaped) {
                const cell = std.mem.trim(u8, self.rest[0..i], " \t");
                self.rest = self.rest[i + 1 ..];
                return cell;
            }
            escaped = byte == '\\' and !escaped;
        }
        const cell = std.mem.trim(u8, self.rest, " \t");
        self.rest = "";
        return cell;
    }
};

pub fn draw(p: *paint.Painter, header: []const u8, lines: *Lines, area: flow.Column) paint.Error!bool {
    if (std.mem.indexOfScalar(u8, header, '|') == null) return false;
    var scan = lines.*;
    const divider = scan.next() orelse return false;
    var titles = Cells.of(header);
    var divider_cells = Cells.of(divider);
    var buffer: [24]Column = undefined;
    var count: usize = 0;
    while (titles.next()) |title| {
        if (count == buffer.len) return false;
        const bar = divider_cells.next() orelse return false;
        const left = std.mem.startsWith(u8, bar, ":");
        const right = std.mem.endsWith(u8, bar, ":");
        const ends: usize = @as(usize, @intFromBool(left)) + @intFromBool(right);
        if (bar.len <= ends) return false;
        const dash = bar[@intFromBool(left) .. bar.len - @intFromBool(right)];
        if (std.mem.count(u8, dash, "-") != dash.len) return false;
        buffer[count] = .{
            .width = @max(2, @min(area.measure, text.width(title))),
            .alignment = if (left and right) .center else if (right) .right else .left,
        };
        count += 1;
    }
    if (count == 0 or divider_cells.next() != null or area.measure < count * 2 + (count - 1) * gutter) return false;
    const columns = buffer[0..count];
    const start = scan;
    while (scan.next()) |line| {
        if (!tableLine(line)) break;
        var cells = Cells.of(line);
        for (columns) |*column| {
            const cell = cells.next() orelse break;
            column.width = @max(column.width, @min(area.measure, text.width(cell)));
        }
    }
    const room = area.measure - (count - 1) * gutter;
    var total: usize = 0;
    for (columns) |column| total += column.width;
    while (total > room) {
        var widest = &columns[0];
        for (columns[1..]) |*column| if (column.width > widest.width) {
            widest = column;
        };
        widest.width -= 1;
        total -= 1;
    }
    try row(p, header, columns, area.left, .bold);
    try p.end();
    try p.to(area.left);
    for (columns, 0..) |column, i| {
        if (i != 0) try p.cut(.faint, "─┼─", gutter);
        for (0..column.width) |_| try p.cut(.faint, "─", 1);
    }
    lines.* = start;
    while (true) {
        var ahead = lines.*;
        const line = ahead.next() orelse break;
        if (!tableLine(line)) break;
        lines.* = ahead;
        try p.end();
        try row(p, line, columns, area.left, area.style);
    }
    return true;
}

fn tableLine(line: []const u8) bool {
    return std.mem.indexOfScalar(u8, line, '|') != null;
}

fn row(p: *paint.Painter, line: []const u8, columns: []const Column, left: usize, style: paint.Style) paint.Error!void {
    var cells = Cells.of(line);
    var fragments: [24][]const u8 = undefined;
    for (fragments[0..columns.len]) |*fragment| fragment.* = cells.next() orelse "";
    var more = true;
    while (more) {
        more = false;
        var start = left;
        try p.to(start);
        for (columns, fragments[0..columns.len], 0..) |column, *fragment, index| {
            if (index != 0) {
                try p.cut(.faint, " │ ", gutter);
                start += gutter;
            }
            const prefix, const width = take(fragment.*, column.width);
            const empty = column.width - width;
            try p.to(start + switch (column.alignment) {
                .left => 0,
                .center => empty / 2,
                .right => empty,
            });
            try p.cut(style, prefix, width);
            start += column.width;
            try p.to(start);
            fragment.* = std.mem.trimStart(u8, fragment.*[prefix.len..], " ");
            more = more or fragment.len != 0;
        }
        if (more) try p.end();
    }
}

fn take(s: []const u8, columns: usize) struct { []const u8, usize } {
    var bytes: usize = 0;
    var width: usize = 0;
    while (bytes < s.len) {
        const c = text.char(s, bytes);
        if (width + c.columns > columns) break;
        bytes += c.len;
        width += c.columns;
    }
    return .{ s[0..bytes], width };
}
