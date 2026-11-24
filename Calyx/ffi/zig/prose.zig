// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

const std = @import("std");
const paint = @import("paint.zig");
const text = @import("text.zig");
const flow = @import("flow.zig");
const table = @import("table.zig");
const math = @import("math.zig");
const diagram = @import("diagram.zig");

pub fn blocks(p: *paint.Painter, body: []const u8, column: flow.Column) paint.Error!void {
    if (!needsBlocks(body)) return flow.flow(p, body, column);
    var open: ?Fence = null;
    var display_math = false;
    var lines = std.mem.splitScalar(u8, body, '\n');
    var first = true;
    while (lines.next()) |raw| {
        const source = std.mem.trimEnd(u8, raw, "\r");
        if (!first) try p.end();
        first = false;
        p.sourceLine(body, raw);
        try p.to(column.left);
        const trimmed_source = std.mem.trimStart(u8, source, " ");
        if (takesFence(open, trimmed_source)) {
            if (p.mei and open == null) {
                const fence = fenceOf(trimmed_source).?;
                const info = std.mem.trim(u8, trimmed_source[fence.len..], " \t");
                if (std.mem.eql(u8, info, "mermaid") and try diagram.draw(p, &lines, trimmed_source[0..fence.len], column, body)) continue;
            }
            open = if (open == null) fenceOf(trimmed_source) else null;
            try p.fit(.faint, source, column.measure);
            continue;
        }
        if (open != null) {
            try codeLine(p, source, column);
            continue;
        }
        if (p.mei and try table.draw(p, source, &lines, column, body)) continue;
        if (p.mei and std.mem.eql(u8, std.mem.trimEnd(u8, trimmed_source, " \t"), "$$")) {
            display_math = !display_math;
            continue;
        }
        var converted: [4096]u8 = undefined;
        if (display_math) {
            try flow.flow(p, math.expression(&converted, trimmed_source) orelse source, column);
            continue;
        }
        const line = if (p.mei and std.mem.indexOfScalar(u8, source, '$') != null)
            math.line(&converted, source) orelse source
        else
            source;
        const trimmed = std.mem.trimStart(u8, line, " ");
        if (heading(trimmed)) |mark| {
            const cut = cutAt(line, trimmed, mark);
            if (!p.mei) try p.fit(.faint, line[0..cut], cut);
            try flow.flow(p, line[cut..], .{ .left = p.column, .measure = p.room(), .style = .bold, .marks = column.marks });
            continue;
        }
        if (std.mem.startsWith(u8, trimmed, ">")) {
            const cut = cutAt(line, trimmed, 1);
            try p.fit(.faint, line[0..cut], cut);
            const rest = flow.Column{ .left = p.column, .measure = p.room(), .style = .faint, .marks = column.marks };
            try flow.flow(p, line[cut..], rest);
            continue;
        }
        if (listMark(trimmed)) |mark| {
            const cut = cutAt(line, trimmed, mark);
            if (p.mei and (trimmed[0] == '-' or trimmed[0] == '*')) {
                try p.cut(.faint, "• ", 2);
            } else try p.fit(.faint, line[0..cut], cut);
            const rest = flow.Column{ .left = p.column, .measure = p.room(), .style = column.style, .marks = column.marks };
            try flow.flow(p, line[cut..], rest);
            continue;
        }
        if (std.mem.count(u8, line, "|") >= 2) {
            const kept = flow.Column{ .left = column.left, .measure = column.measure, .style = if (ruleRow(trimmed)) .faint else column.style, .marks = .kept };
            try flow.flow(p, line, kept);
            continue;
        }
        if (std.mem.indexOf(u8, line, "](") != null) {
            try links(p, line, column);
            continue;
        }
        try flow.flow(p, line, column);
    }
}

fn needsBlocks(body: []const u8) bool {
    var lines = std.mem.splitScalar(u8, body, '\n');
    while (lines.next()) |raw| {
        const line = std.mem.trimStart(u8, std.mem.trimEnd(u8, raw, "\r"), " ");
        if (fenceOf(line) != null) return true;
        if (heading(line) != null) return true;
        if (std.mem.startsWith(u8, line, ">")) return true;
        if (listMark(line) != null) return true;
        if (std.mem.count(u8, line, "|") >= 2) return true;
        if (std.mem.indexOf(u8, line, "](") != null) return true;
        if (std.mem.indexOfScalar(u8, line, '$') != null) return true;
    }
    return false;
}

const Fence = struct { mark: u8, len: usize };

fn fenceOf(trimmed: []const u8) ?Fence {
    if (trimmed.len < 3) return null;
    const mark = trimmed[0];
    if (mark != '`' and mark != '~') return null;
    var n: usize = 0;
    while (n < trimmed.len and trimmed[n] == mark) n += 1;
    if (n < 3) return null;
    return .{ .mark = mark, .len = n };
}

fn takesFence(open: ?Fence, trimmed: []const u8) bool {
    const found = fenceOf(trimmed) orelse return false;
    const current = open orelse return true;
    if (found.mark != current.mark or found.len < current.len) return false;
    return std.mem.trimStart(u8, trimmed[found.len..], " \t").len == 0;
}

fn cutAt(line: []const u8, trimmed: []const u8, mark: usize) usize {
    return line.len - trimmed.len + mark;
}

fn heading(line: []const u8) ?usize {
    var n: usize = 0;
    while (n < line.len and n < 6 and line[n] == '#') n += 1;
    if (n == 0 or n >= line.len or line[n] != ' ') return null;
    return n + 1;
}

fn listMark(line: []const u8) ?usize {
    if (std.mem.startsWith(u8, line, "- ") or std.mem.startsWith(u8, line, "* ")) return 2;
    var n: usize = 0;
    while (n < line.len and line[n] >= '0' and line[n] <= '9') n += 1;
    if (n > 0 and n + 1 < line.len and line[n] == '.' and line[n + 1] == ' ') return n + 2;
    return null;
}

fn ruleRow(line: []const u8) bool {
    for (line) |byte| {
        if (byte != '|' and byte != '-' and byte != ':' and byte != ' ') return false;
    }
    return std.mem.indexOfScalar(u8, line, '-') != null;
}

fn codeLine(p: *paint.Painter, line: []const u8, column: flow.Column) paint.Error!void {
    const trimmed = std.mem.trimStart(u8, line, " ");
    if (std.mem.startsWith(u8, trimmed, "//") or std.mem.startsWith(u8, trimmed, "#")) {
        try p.fit(.faint, line, column.measure);
        return;
    }
    var at: usize = 0;
    var quote: u8 = 0;
    var slash = false;
    while (at < line.len and p.room() > 0) {
        if (quote == 0 and std.mem.startsWith(u8, line[at..], "//")) {
            try p.fit(.faint, line[at..], p.room());
            return;
        }
        const c = text.char(line, at);
        if (quote == 0 and (c.cp == '"' or c.cp == '\'')) {
            quote = @intCast(c.cp);
        } else if (quote != 0 and c.cp == quote and !slash) quote = 0;
        slash = quote != 0 and c.cp == '\\' and !slash;
        const style: paint.Style = if (quote != 0) .accent else .plain;
        if (!try p.char(style, c)) return;
        at += c.len;
    }
}

fn links(p: *paint.Painter, line: []const u8, column: flow.Column) paint.Error!void {
    var at: usize = 0;
    while (at < line.len) {
        if (line[at] == '[') {
            if (linkAt(line, at)) |found| {
                if (text.width(found.label) > p.room() and p.column > column.left) {
                    try p.wrap(false);
                    try p.to(column.left);
                }
                try p.link(.accent, found.url, found.label);
                at = found.next;
                continue;
            }
        }
        const c = text.char(line, at);
        if (!try p.char(column.style, c)) {
            try p.wrap(false);
            try p.to(column.left);
            if (!try p.char(column.style, c)) return;
        }
        at += c.len;
    }
}

const Link = struct { label: []const u8, url: []const u8, next: usize };

fn linkAt(line: []const u8, at: usize) ?Link {
    const close = std.mem.indexOfScalarPos(u8, line, at + 1, ']') orelse return null;
    if (close + 1 >= line.len or line[close + 1] != '(') return null;
    const end = std.mem.indexOfScalarPos(u8, line, close + 2, ')') orelse return null;
    if (end == close + 2) return null;
    return .{
        .label = line[at + 1 .. close],
        .url = line[close + 2 .. end],
        .next = end + 1,
    };
}

test "formulas preserve the surrounding markdown and the destination of a link" {
    var buffer: [1024]u8 = undefined;
    var output: std.Io.Writer = .fixed(&buffer);
    var painter = paint.Painter.meiFrame(&output, 90);
    try blocks(&painter, "# A $x^2$ heading\n[docs](https://example.test/$x^2$) gives $x^2$", .{
        .left = 0,
        .measure = 89,
        .style = .plain,
        .marks = .prose,
    });
    const drawn = output.buffered();
    try std.testing.expect(std.mem.indexOf(u8, drawn, "\x1b[0;1mA") != null);
    try std.testing.expect(std.mem.indexOf(u8, drawn, "\x1b]8;;https://example.test/$x^2$\x1b\\") != null);
    try std.testing.expect(std.mem.indexOf(u8, drawn, "gives x²") != null);
}

test "table escapes are drawn as one cell character" {
    var buffer: [1024]u8 = undefined;
    var output: std.Io.Writer = .fixed(&buffer);
    var painter = paint.Painter.meiFrame(&output, 36);
    try blocks(&painter, "| regex | value |\n| --- | --- |\n| a\\|b | yes |", .{
        .left = 0,
        .measure = 35,
        .style = .plain,
        .marks = .prose,
    });
    try std.testing.expect(std.mem.indexOf(u8, output.buffered(), "a|b") != null);
}

test "a narrow Mermaid chart keeps the edge and both node identities" {
    var buffer: [1024]u8 = undefined;
    var output: std.Io.Writer = .fixed(&buffer);
    var painter = paint.Painter.meiFrame(&output, 21);
    try blocks(&painter, "```mermaid\nflowchart LR\nA[Very long left label here] --> B[Check]\nB --> C[Done]\n```", .{
        .left = 0,
        .measure = 20,
        .style = .plain,
        .marks = .prose,
    });
    const drawn = output.buffered();
    try std.testing.expect(std.mem.indexOf(u8, drawn, "↓") != null);
    try std.testing.expect(std.mem.indexOf(u8, drawn, "B: Check") != null);
    try std.testing.expect(std.mem.indexOf(u8, drawn, "C: Done") != null);
}

test "plain prose stays on the flow path" {
    var buffer: [256]u8 = undefined;
    var out: std.Io.Writer = .fixed(&buffer);
    var p = paint.Painter.init(&out, 40);
    try blocks(&p, "one `city` two", .{ .left = 0, .measure = 30, .style = .plain, .marks = .prose });
    try std.testing.expect(std.mem.indexOf(u8, out.buffered(), "city") != null);
    try std.testing.expect(std.mem.indexOf(u8, out.buffered(), "`") != null);
}

test "a Mei transcript lays out tables, math and a Mermaid flowchart" {
    var buffer: [4096]u8 = undefined;
    var writer: std.Io.Writer = .fixed(&buffer);
    var p = paint.Painter.meiFrame(&writer, 70);
    try blocks(&p,
        \\| 项目 | 结果 |
        \\| --- | ---: |
        \\| 会话 | 12 |
        \\
        \\$\pi r^2$
        \\
        \\```mermaid
        \\flowchart LR
        \\A[读取] --> B[完成]
        \\```
    , .{ .left = 0, .measure = 69, .style = .plain, .marks = .prose });
    const drawn = writer.buffered();
    try std.testing.expect(std.mem.indexOf(u8, drawn, "┼") != null);
    try std.testing.expect(std.mem.indexOf(u8, drawn, "π r²") != null);
    try std.testing.expect(std.mem.indexOf(u8, drawn, "──→") != null);
    try std.testing.expect(std.mem.indexOf(u8, drawn, "flowchart") == null);
}

test "headings lists quotes tables and links are marked" {
    var buffer: [1024]u8 = undefined;
    var out: std.Io.Writer = .fixed(&buffer);
    var p = paint.Painter.init(&out, 80);
    try blocks(&p, "# Title\n- item\n> quoted\n| a | b |\n| --- | --- |\n[docs](https://example.test)\n```\n// note\n```", .{
        .left = 0,
        .measure = 70,
        .style = .plain,
        .marks = .prose,
    });
    const drawn = out.buffered();
    try std.testing.expect(std.mem.indexOf(u8, drawn, "Title") != null);
    try std.testing.expect(std.mem.indexOf(u8, drawn, "item") != null);
    try std.testing.expect(std.mem.indexOf(u8, drawn, "quoted") != null);
    try std.testing.expect(std.mem.indexOf(u8, drawn, "https://example.test") != null);
    try std.testing.expect(std.mem.indexOf(u8, drawn, "note") != null);
}

test "an open fence does not swallow the following paragraph" {
    var buffer: [256]u8 = undefined;
    var out: std.Io.Writer = .fixed(&buffer);
    var p = paint.Painter.init(&out, 40);
    try blocks(&p, "```\ncode\n", .{ .left = 0, .measure = 30, .style = .plain, .marks = .prose });
    try std.testing.expect(std.mem.indexOf(u8, out.buffered(), "code") != null);
}

test "a longer fence keeps an inner fence as code" {
    var buffer: [512]u8 = undefined;
    var out: std.Io.Writer = .fixed(&buffer);
    var p = paint.Painter.init(&out, 80);
    try blocks(&p, "````\n[docs](https://example.test)\n````", .{ .left = 0, .measure = 70, .style = .plain, .marks = .prose });
    try std.testing.expect(std.mem.indexOf(u8, out.buffered(), "\x1b]8;;") == null);
    try std.testing.expect(std.mem.indexOf(u8, out.buffered(), "docs") != null);
}

test "a long quote keeps the tail of the line" {
    var buffer: [1024]u8 = undefined;
    var out: std.Io.Writer = .fixed(&buffer);
    var p = paint.Painter.init(&out, 40);
    const quote = "> abcdefghijabcdefghijabcdefghijabcdefghij";
    try blocks(&p, quote, .{ .left = 0, .measure = 12, .style = .plain, .marks = .prose });
    const drawn = out.buffered();
    try std.testing.expect(std.mem.indexOf(u8, drawn, "…") == null);
    try std.testing.expect(std.mem.indexOf(u8, drawn, "abcdefghijabcdefghij") != null);
}

test "wide characters keep their columns inside a list" {
    var buffer: [256]u8 = undefined;
    var out: std.Io.Writer = .fixed(&buffer);
    var p = paint.Painter.init(&out, 20);
    try blocks(&p, "- 中文中文中文", .{ .left = 0, .measure = 16, .style = .plain, .marks = .prose });
    try std.testing.expect(std.mem.indexOf(u8, out.buffered(), "中文") != null);
    try std.testing.expect(p.column <= p.limit);
}
