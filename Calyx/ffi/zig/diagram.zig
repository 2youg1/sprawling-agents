// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

const std = @import("std");
const flow = @import("flow.zig");
const paint = @import("paint.zig");

const Lines = std.mem.SplitIterator(u8, .scalar);
const Kind = enum { flowchart, sequence };
const Edge = struct { left: []const u8, right: []const u8, label: []const u8 = "" };

pub fn draw(p: *paint.Painter, lines: *Lines, fence: []const u8, area: flow.Column) paint.Error!bool {
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
        try p.to(area.left);
        if (kind == .flowchart and (std.mem.endsWith(u8, header, " TD") or std.mem.endsWith(u8, header, " TB"))) {
            try node(p, item.left);
            try p.end();
            try p.to(area.left + 2);
            try p.cut(.faint, "↓", 1);
            try p.end();
            try p.to(area.left);
            try node(p, item.right);
        } else {
            try node(p, item.left);
            try p.cut(.faint, " ──→ ", 5);
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
    const arrow = if (kind == .flowchart) "-->" else if (std.mem.indexOf(u8, line, "-->>") != null) "-->>" else "->>";
    const split = std.mem.indexOf(u8, line, arrow) orelse return null;
    const from = nodeLabel(line[0..split]) orelse return null;
    var to = trim(line[split + arrow.len ..]);
    var label: []const u8 = "";
    if (kind == .sequence) {
        const colon = std.mem.indexOfScalar(u8, to, ':') orelse return null;
        label = trim(to[colon + 1 ..]);
        to = to[0..colon];
    }
    return .{ .left = from, .right = nodeLabel(to) orelse return null, .label = label };
}

fn nodeLabel(raw: []const u8) ?[]const u8 {
    const label = std.mem.trimEnd(u8, trim(raw), ";");
    var index: usize = 0;
    while (index < label.len and (std.ascii.isAlphanumeric(label[index]) or label[index] == '_')) : (index += 1) {}
    if (index == 0) return null;
    if (index == label.len) return label;
    const close: u8 = switch (label[index]) {
        '[' => ']',
        '{' => '}',
        '(' => ')',
        else => return null,
    };
    if (label[label.len - 1] != close or index + 1 >= label.len - 1) return null;
    const body = label[index + 1 .. label.len - 1];
    if (std.mem.indexOfScalar(u8, body, close) != null) return null;
    return std.mem.trim(u8, body, "\"");
}

fn node(p: *paint.Painter, name: []const u8) paint.Error!void {
    try p.cut(.faint, "[", 1);
    try p.fit(.plain, name, p.room() -| 1);
    try p.cut(.faint, "]", 1);
}

fn trim(line: []const u8) []const u8 {
    return std.mem.trim(u8, line, " \t\r");
}
