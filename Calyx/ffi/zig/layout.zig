// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

const std = @import("std");

pub const Point = struct {
    row: usize,
    column: usize,

    pub fn before(a: Point, b: Point) bool {
        return a.row < b.row or (a.row == b.row and a.column < b.column);
    }
};

pub const Origin = struct {
    entry: usize,
    fragment: enum { main, input, output, thought } = .main,
};

pub const Location = struct {
    origin: Origin,
    line: usize = 0,
    offset: usize = 0,

    fn before(a: Location, b: Location) bool {
        if (a.origin.entry != b.origin.entry) return a.origin.entry < b.origin.entry;
        if (a.origin.fragment != b.origin.fragment) return @backingInt(a.origin.fragment) < @backingInt(b.origin.fragment);
        if (a.line != b.line) return a.line < b.line;
        return a.offset < b.offset;
    }
};

pub const Range = struct { start: Location, finish: Location };

pub const Row = struct {
    start: Location,
    indent: usize,
    width: usize,

    pub fn at(self: Row, col: usize) Location {
        var position = self.start;
        position.offset += @min(col, self.width) -| self.indent;
        return position;
    }

    fn column(self: Row, position: Location) usize {
        if (!std.meta.eql(self.start.origin, position.origin) or self.start.line != position.line) return @min(self.indent, self.width);
        return @min(self.indent + (position.offset -| self.start.offset), self.width);
    }
};

pub fn locate(rows: []const Row, position: Location) Point {
    if (rows.len == 0) return .{ .row = 0, .column = 0 };
    var lo: usize = 0;
    var hi = rows.len;
    while (lo < hi) {
        const middle = lo + (hi - lo) / 2;
        if (position.before(rows[middle].start)) {
            hi = middle;
        } else {
            lo = middle + 1;
        }
    }
    const index = lo -| 1;
    const row = rows[index];
    if (position.origin.fragment != .main and row.start.origin.entry == position.origin.entry and row.start.origin.fragment != position.origin.fragment) {
        return locate(rows, .{ .origin = .{ .entry = position.origin.entry } });
    }
    return .{ .row = index, .column = row.column(position) };
}

pub const Index = struct {
    gpa: std.mem.Allocator,
    rows: std.ArrayList(Row) = .empty,
    start: Location = .{ .origin = .{ .entry = 0 } },
    indent: usize = 0,

    pub fn begin(self: *Index, origin: Origin) void {
        self.start = .{ .origin = origin };
        self.indent = 0;
    }

    pub fn line(self: *Index, width: usize, separation: []const u8) std.mem.Allocator.Error!void {
        try self.rows.append(self.gpa, .{ .start = self.start, .indent = self.indent, .width = width });
        if (separation.len != 0 and separation[0] == '\n') {
            self.start.line += 1;
            self.start.offset = 0;
        } else {
            self.start.offset += width -| self.indent;
            self.start.offset += separation.len;
        }
        self.indent = 0;
    }
};
