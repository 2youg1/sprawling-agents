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

    fn sameLine(a: Location, b: Location) bool {
        return std.meta.eql(a.origin, b.origin) and a.line == b.line;
    }

    fn before(a: Location, b: Location) bool {
        if (a.origin.entry != b.origin.entry) return a.origin.entry < b.origin.entry;
        if (a.origin.fragment != b.origin.fragment) return @backingInt(a.origin.fragment) < @backingInt(b.origin.fragment);
        if (a.line != b.line) return a.line < b.line;
        return a.offset < b.offset;
    }
};

pub const Range = struct {
    start: Location,
    finish: Location,

    pub fn ordered(self: Range) Range {
        return if (self.finish.before(self.start)) .{ .start = self.finish, .finish = self.start } else self;
    }
};

const Span = struct {
    column: usize,
    width: usize,
    offset: usize,
    cell: usize,
};

pub const Row = struct {
    start: Location,
    indent: usize,
    width: usize,
    spans: []const Span = &.{},

    pub fn at(self: Row, col: usize) Location {
        var position = self.start;
        if (self.spans.len != 0) {
            var previous: ?Span = null;
            for (self.spans) |span| {
                if (col < span.column) {
                    position.offset = if (previous) |prior| prior.offset + prior.width else span.offset;
                    return position;
                }
                if (col < span.column + span.width) {
                    position.offset = span.offset + col - span.column;
                    return position;
                }
                previous = span;
            }
            const last = previous.?;
            position.offset = last.offset + last.width;
            return position;
        }
        position.offset += @min(col, self.width) -| self.indent;
        return position;
    }

    fn column(self: Row, position: Location) usize {
        if (!self.start.sameLine(position)) return @min(self.indent, self.width);
        return @min(self.indent + (position.offset -| self.start.offset), self.width);
    }
};

pub const Affinity = enum { forward, backward };

const SpanHit = struct { row: usize, span: Span };

fn spanAt(rows: []const Row, position: Location, affinity: Affinity) ?SpanHit {
    var line = position;
    line.offset = 0;
    const first = upperBound(rows, line) -| 1;
    var edge: ?SpanHit = null;
    for (rows[first..], first..) |row, i| {
        if (!row.start.sameLine(position)) break;
        for (row.spans) |span| {
            const end = span.offset + span.width;
            if (span.offset < position.offset and position.offset < end) return .{ .row = i, .span = span };
            if (position.offset == span.offset or position.offset == end) {
                const hit: SpanHit = .{ .row = i, .span = span };
                if (if (affinity == .forward) position.offset == span.offset else position.offset == end) return hit;
                edge = hit;
            }
        }
    }
    return edge;
}

pub const CellRange = struct {
    rows: []const Row,
    range: Range,
    cell: usize,

    pub fn contains(self: CellRange, row: usize, col: usize, width: usize) bool {
        if (row >= self.rows.len or !self.rows[row].start.sameLine(self.range.start)) return false;
        for (self.rows[row].spans) |span| {
            if (span.cell == self.cell and col >= span.column and col < span.column + span.width) {
                const offset = span.offset + col - span.column;
                return offset + width > self.range.start.offset and offset < self.range.finish.offset;
            }
        }
        return false;
    }
};

pub fn cellRange(rows: []const Row, range: Range) ?CellRange {
    if (!range.start.sameLine(range.finish)) return null;
    const start = spanAt(rows, range.start, .forward) orelse return null;
    const finish = spanAt(rows, range.finish, .backward) orelse return null;
    if (start.span.cell != finish.span.cell) return null;
    return .{ .rows = rows, .range = range, .cell = start.span.cell };
}

pub fn locate(rows: []const Row, position: Location, affinity: Affinity) Point {
    if (rows.len == 0) return .{ .row = 0, .column = 0 };
    if (spanAt(rows, position, affinity)) |hit| {
        return .{ .row = hit.row, .column = hit.span.column + position.offset - hit.span.offset };
    }
    const index = upperBound(rows, position) -| 1;
    const row = rows[index];
    if (position.origin.fragment != .main and row.start.origin.entry == position.origin.entry and row.start.origin.fragment != position.origin.fragment) {
        return locate(rows, .{ .origin = .{ .entry = position.origin.entry } }, affinity);
    }
    return .{ .row = index, .column = row.column(position) };
}

fn upperBound(rows: []const Row, position: Location) usize {
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
    return lo;
}

pub const Index = struct {
    gpa: std.mem.Allocator,
    arena: std.mem.Allocator,
    rows: std.ArrayList(Row) = .empty,
    start: Location = .{ .origin = .{ .entry = 0 } },
    indent: ?usize = null,
    spans: std.ArrayList(Span) = .empty,

    pub fn deinit(self: *Index) void {
        self.rows.deinit(self.gpa);
        self.spans.deinit(self.gpa);
    }

    pub fn begin(self: *Index, origin: Origin) void {
        self.start = .{ .origin = origin };
        self.indent = null;
    }

    pub fn span(self: *Index, column: usize, width: usize, offset: usize, cell: usize) std.mem.Allocator.Error!void {
        if (width == 0) return;
        if (self.spans.items.len != 0) {
            const previous = &self.spans.items[self.spans.items.len - 1];
            if (previous.cell == cell and previous.column + previous.width == column and previous.offset + previous.width == offset) {
                previous.width += width;
                return;
            }
        }
        try self.spans.append(self.gpa, .{ .column = column, .width = width, .offset = offset, .cell = cell });
    }

    pub fn line(self: *Index, width: usize, separation: []const u8) std.mem.Allocator.Error!void {
        const indent = self.indent orelse 0;
        try self.rows.append(self.gpa, .{
            .start = self.start,
            .indent = indent,
            .width = width,
            .spans = try self.arena.dupe(Span, self.spans.items),
        });
        self.spans.clearRetainingCapacity();
        if (separation.len != 0 and separation[0] == '\n') {
            self.start.line += 1;
            self.start.offset = 0;
        } else {
            self.start.offset += width -| indent;
            self.start.offset += separation.len;
        }
        self.indent = null;
    }
};
