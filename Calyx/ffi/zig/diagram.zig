// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

const std = @import("std");
const flow = @import("flow.zig");
const paint = @import("paint.zig");
const text = @import("text.zig");

const Lines = std.mem.SplitIterator(u8, .scalar);
const Kind = enum { flowchart, sequence };
const Node = struct {
    id: []const u8,
    label: []const u8 = "",

    fn width(self: Node) usize {
        return 2 + self.id.len + if (self.label.len != 0) 2 + text.width(self.label) else 0;
    }
};
const Edge = struct { left: Node, right: Node, label: []const u8 = "" };
const arrow = " ──→ ";
const arrow_width = 5;

pub fn draw(p: *paint.Painter, lines: *Lines, fence: []const u8, area: flow.Column, body: []const u8) paint.Error!bool {
    if (area.measure < 3) return false;
    var source = lines.*;
    const header = std.mem.trim(u8, source.next() orelse return false, " \t\r");
    const kind: Kind = if (std.mem.startsWith(u8, header, "graph ") or std.mem.startsWith(u8, header, "flowchart "))
        .flowchart
    else if (std.mem.eql(u8, header, "sequenceDiagram"))
        .sequence
    else
        return false;
    var scan = source;
    var found = false;
    var count: usize = 0;
    while (scan.next()) |raw| {
        const line = trim(raw);
        if (closing(line, fence)) {
            found = true;
            break;
        }
        if (line.len == 0) continue;
        if (edge(line, kind) == null) return false;
        count += 1;
    }
    if (!found or count == 0) return false;
    lines.* = scan;

    var first = true;
    while (source.next()) |raw| {
        const line = trim(raw);
        if (closing(line, fence)) break;
        const item = edge(line, kind) orelse continue;
        if (!first) try p.end();
        first = false;
        p.sourceLine(body, raw);
        try p.textLeft(area.left);
        const down = kind == .flowchart and (std.mem.endsWith(u8, header, " TD") or std.mem.endsWith(u8, header, " TB"));
        const label_width = if (item.label.len != 0) 2 + text.width(item.label) else 0;
        if (down or item.left.width() + arrow_width + item.right.width() + label_width > p.room()) {
            try node(p, item.left);
            try p.wrap(false);
            try p.textLeft(area.left);
            try p.cut(.faint, "↓", 1);
            try p.wrap(false);
            try p.textLeft(area.left);
            try node(p, item.right);
            if (item.label.len != 0) {
                try p.wrap(false);
                try flow.flow(p, item.label, .{ .left = area.left, .measure = area.measure, .style = .faint, .marks = .plain });
            }
        } else {
            try node(p, item.left);
            try p.cut(.faint, arrow, arrow_width);
            try node(p, item.right);
            if (item.label.len != 0) {
                try p.to(p.column + 2);
                try flow.flow(p, item.label, .{ .left = p.column, .measure = p.room(), .style = .faint, .marks = .plain });
            }
        }
    }
    return true;
}

fn closing(line: []const u8, fence: []const u8) bool {
    return std.mem.startsWith(u8, line, fence) and std.mem.trim(u8, line, fence).len == 0;
}

fn edge(line: []const u8, kind: Kind) ?Edge {
    const delimiter = if (kind == .flowchart) "-->" else if (std.mem.indexOf(u8, line, "-->>") != null) "-->>" else "->>";
    const split = std.mem.indexOf(u8, line, delimiter) orelse return null;
    const from = parseNode(line[0..split]) orelse return null;
    var to = trim(line[split + delimiter.len ..]);
    var label: []const u8 = "";
    if (kind == .sequence) {
        const colon = std.mem.indexOfScalar(u8, to, ':') orelse return null;
        label = trim(to[colon + 1 ..]);
        to = to[0..colon];
    }
    return .{ .left = from, .right = parseNode(to) orelse return null, .label = label };
}

fn parseNode(raw: []const u8) ?Node {
    const label = std.mem.trimEnd(u8, trim(raw), ";");
    var index: usize = 0;
    while (index < label.len and (std.ascii.isAlphanumeric(label[index]) or label[index] == '_')) : (index += 1) {}
    if (index == 0) return null;
    if (index == label.len) return .{ .id = label };
    const close: u8 = switch (label[index]) {
        '[' => ']',
        '{' => '}',
        '(' => ')',
        else => return null,
    };
    if (label[label.len - 1] != close or index + 1 >= label.len - 1) return null;
    const body = label[index + 1 .. label.len - 1];
    if (std.mem.indexOfScalar(u8, body, close) != null) return null;
    return .{ .id = label[0..index], .label = std.mem.trim(u8, body, "\"") };
}

fn node(p: *paint.Painter, value: Node) paint.Error!void {
    const left = p.column;
    try p.cut(.faint, "[", 1);
    if (value.id.len > p.room() -| 1) {
        try flow.flow(p, value.id, .{ .left = left, .measure = p.limit - left, .style = .plain, .marks = .verbatim });
        if (p.room() == 0) {
            try p.wrap(false);
            try p.textLeft(left);
        }
        try p.cut(.faint, "]", 1);
        return;
    }
    try p.cut(.plain, value.id, value.id.len);
    if (value.label.len != 0 and p.room() >= 4) {
        try p.cut(.plain, ": ", 2);
        try p.fit(.plain, value.label, p.room() -| 1);
    }
    try p.cut(.faint, "]", 1);
}

fn trim(line: []const u8) []const u8 {
    return std.mem.trim(u8, line, " \t\r");
}
