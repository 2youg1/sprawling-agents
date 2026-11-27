// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

const std = @import("std");
const scene = @import("scene.zig");
const paint = @import("paint.zig");
const text = @import("text.zig");

pub const Error = scene.Error || paint.Error;

pub const CodeRegion = struct {
    row: usize,
    column: usize,
    width: usize,

    pub fn contains(self: CodeRegion, row: u16, column: u16) bool {
        return row == self.row and column >= self.column and column < self.column + self.width;
    }
};

const primary_rows = 3;
const open_hint = "Enter: open WebUI";
const pairing_hint = "Hold Ctrl+G for pairing code";
const copy_hint = "Hold Ctrl+G for code · click to copy";

pub fn codeRegion(columns: u16, rows: u16, code: []const u8) ?CodeRegion {
    const width = text.width(code);
    if (rows < primary_rows or width == 0 or width >= columns) return null;
    return .{ .row = topRow(rows) + primary_rows, .column = centered(columns, width) + 1, .width = width };
}

pub fn quiet(out: *std.Io.Writer, r: *scene.Reader) Error!void {
    const columns = try r.short();
    const rows = try r.short();
    const url = try r.text();
    const code = try r.text();
    const key = try r.text();
    const transient = try r.text();

    var p = paint.Painter.init(out, columns);
    const available = p.room();
    const top = topRow(rows);
    try out.writeAll("\x1b[?2026h\x1b[H\x1b[2J");
    if (rows >= 1) {
        try place(&p, top, centered(columns, @min(text.width(url), available)));
        try p.link(.plain, url, url);
        try p.close();
    }
    if (rows >= 2) try line(&p, columns, top + 1, .faint, open_hint);
    if (rows >= primary_rows) {
        if (codeRegion(columns, rows, code)) |region| {
            try place(&p, region.row - 1, region.column - 1);
            try p.cut(.bold, code, region.width);
        } else {
            const hint = if (code.len != 0)
                "Widen for pairing code"
            else if (available >= text.width(copy_hint))
                copy_hint
            else if (available >= pairing_hint.len)
                pairing_hint
            else
                "Hold Ctrl+G";
            try line(&p, columns, top + 2, .faint, hint);
        }
    }

    var row = top + primary_rows;
    if (key.len != 0 and row < rows) {
        const label = "key  ";
        const width = label.len + text.width(key);
        try place(&p, row, centered(columns, @min(width, available)));
        try p.fit(.faint, label, p.room());
        try p.fit(.bold, key, p.room());
        row += 1;
    }
    if (transient.len != 0) {
        row += 1;
        var notices = std.mem.splitScalar(u8, transient, '\n');
        while (row < rows) : (row += 1) {
            const notice = notices.next() orelse break;
            try line(&p, columns, row, .faint, notice);
        }
    }
    try p.close();
    try out.writeAll("\x1b[?2026l");
}

fn topRow(rows: u16) usize {
    return (rows -| primary_rows) / 2;
}

fn centered(columns: u16, width: usize) usize {
    return (@as(usize, columns) -| width) / 2;
}

fn line(p: *paint.Painter, columns: u16, row: usize, style: paint.Style, value: []const u8) paint.Error!void {
    try place(p, row, centered(columns, @min(text.width(value), p.limit)));
    try p.fit(style, value, p.room());
}

fn place(p: *paint.Painter, row: usize, column: usize) paint.Error!void {
    try p.out.print("\x1b[{d};{d}H", .{ row + 1, column + 1 });
    p.column = column;
}

test "a thirty-three-column quiet host explains opening and renders a code only when requested" {
    const write = @import("write.zig");
    const gpa = std.testing.allocator;
    const code = "0123456789abcdef0123456789abcdef";
    var hidden = try write.Scene.quietFrame(gpa, .{
        .columns = 33,
        .rows = 12,
        .url = "http://127.0.0.1:3425/",
        .code = "",
    });
    defer hidden.deinit(gpa);
    var idle: scene.Reader = .{ .bytes = hidden.bytes.items };
    _ = try idle.tag();
    var hidden_bytes: [1024]u8 = undefined;
    var hidden_output: std.Io.Writer = .fixed(&hidden_bytes);
    try quiet(&hidden_output, &idle);
    try std.testing.expect(std.mem.indexOf(u8, hidden_output.buffered(), "Hold Ctrl+G for pairing code") != null);
    try std.testing.expect(std.mem.indexOf(u8, hidden_output.buffered(), code) == null);
    var frame = try write.Scene.quietFrame(gpa, .{
        .columns = 33,
        .rows = 12,
        .url = "http://127.0.0.1:3425/",
        .code = code,
    });
    defer frame.deinit(gpa);
    var input: scene.Reader = .{ .bytes = frame.bytes.items };
    _ = try input.tag();
    var bytes: [1024]u8 = undefined;
    var output: std.Io.Writer = .fixed(&bytes);
    try quiet(&output, &input);
    try std.testing.expect(std.mem.indexOf(u8, output.buffered(), code) != null);
    try std.testing.expect(std.mem.indexOf(u8, output.buffered(), "Enter: open WebUI") != null);
}

test "narrow quiet host instructions leave room for the existing key" {
    const write = @import("write.zig");
    const gpa = std.testing.allocator;
    var frame = try write.Scene.quietFrame(gpa, .{
        .columns = 40,
        .rows = 12,
        .url = "http://127.0.0.1:3425/",
        .code = "",
        .key = "test-key",
    });
    defer frame.deinit(gpa);
    var input: scene.Reader = .{ .bytes = frame.bytes.items };
    _ = try input.tag();
    var bytes: [1024]u8 = undefined;
    var output: std.Io.Writer = .fixed(&bytes);
    try quiet(&output, &input);
    try std.testing.expect(std.mem.indexOf(u8, output.buffered(), "Enter: open") != null);
    try std.testing.expect(std.mem.indexOf(u8, output.buffered(), "test-key") != null);
}

test "pairing code occupies a clickable row only when every character fits" {
    const write = @import("write.zig");
    const gpa = std.testing.allocator;
    const code = "0123456789abcdef0123456789abcdef";
    const region = codeRegion(33, 12, code).?;
    try std.testing.expect(region.contains(7, 1));
    try std.testing.expect(region.contains(7, 32));
    try std.testing.expect(!region.contains(7, 33));
    try std.testing.expect(!region.contains(6, 1));
    try std.testing.expect(codeRegion(32, 12, code) == null);
    try std.testing.expect(codeRegion(80, 2, code) == null);
    try std.testing.expect(codeRegion(80, 12, "") == null);

    var frame = try write.Scene.quietFrame(gpa, .{
        .columns = 32,
        .rows = 12,
        .url = "http://127.0.0.1:3425/",
        .code = code,
    });
    defer frame.deinit(gpa);
    var input: scene.Reader = .{ .bytes = frame.bytes.items };
    _ = try input.tag();
    var bytes: [1024]u8 = undefined;
    var output: std.Io.Writer = .fixed(&bytes);
    try quiet(&output, &input);
    try std.testing.expect(std.mem.indexOf(u8, output.buffered(), "Widen for pairing code") != null);
    try std.testing.expect(std.mem.indexOf(u8, output.buffered(), code[0..16]) == null);
    try std.testing.expect(std.mem.indexOf(u8, output.buffered(), "Enter: open WebUI") != null);
}
