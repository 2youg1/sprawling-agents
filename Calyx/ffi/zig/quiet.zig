// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

const std = @import("std");
const scene = @import("scene.zig");
const paint = @import("paint.zig");
const text = @import("text.zig");

pub const Error = scene.Error || paint.Error;

pub fn quiet(out: *std.Io.Writer, r: *scene.Reader) Error!void {
    const columns = try r.short();
    const rows = try r.short();
    const url = try r.text();
    const code = try r.text();
    const key = try r.text();
    const transient = try r.text();
    const label = "pairing code";
    const label_room = label.len + 3;
    const code_room = text.width(code);
    const key_label = "   key  ";
    const instructions = "Enter: open WebUI  C: show pairing code";
    const hint = if (columns >= instructions.len) instructions else "Enter: open  C: show code";
    const second = (if (code.len == 0) hint.len else label_room + code_room) + if (key.len > 0) key_label.len + text.width(key) else 0;
    const block = @max(@max(text.width(url), second), text.width(transient));
    const left = (@as(usize, columns) -| block) / 2;
    const stacked = code.len > 0 and columns >= code_room and columns < label_room + code_room and rows > 2;
    const lines: usize = if (stacked) 3 else 2;
    const top = (@as(usize, rows) -| lines) / 2;
    try out.writeAll("\x1b[?2026h\x1b[H\x1b[2J");
    var p = paint.Painter.init(out, columns);
    try place(out, top, left);
    p.column = left;
    try p.link(.plain, url, url);
    try p.close();
    try place(out, top + 1, left);
    p.column = left;
    if (code.len == 0) {
        try p.fit(.faint, hint, p.room());
    } else if (code_room > p.room()) {
        try p.fit(.faint, "widen for pairing code", p.room());
    } else {
        if (stacked) {
            try p.cut(.faint, label, label.len);
            try place(out, top + 2, left);
            p.column = left;
        } else if (label_room + code_room <= p.room()) {
            try p.cut(.faint, label, label.len);
            try p.to(p.column + 3);
        }
        try p.cut(.bold, code, code_room);
    }
    if (key.len > 0) {
        try p.fit(.faint, key_label, key_label.len);
        try p.fit(.bold, key, p.room());
    }
    if (transient.len > 0) {
        var row = top + lines + 1;
        var notices = std.mem.splitScalar(u8, transient, '\n');
        while (notices.next()) |notice| : (row += 1) {
            if (row >= rows) break;
            try place(out, row, left);
            p.column = left;
            try p.fit(.faint, notice, p.room());
        }
    }
    try p.close();
    try out.writeAll("\x1b[?2026l");
}

fn place(out: *std.Io.Writer, row: usize, column: usize) paint.Error!void {
    try out.print("\x1b[{d};{d}H", .{ row + 1, column + 1 });
}

test "a forty-column quiet host explains opening and renders a code only when requested" {
    const write = @import("write.zig");
    const gpa = std.testing.allocator;
    const code = "0123456789abcdef0123456789abcdef";
    var hidden = try write.Scene.quietFrame(gpa, .{
        .columns = 40,
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
    try std.testing.expect(std.mem.indexOf(u8, hidden_output.buffered(), "Enter: open WebUI  C: show pairing code") != null);
    try std.testing.expect(std.mem.indexOf(u8, hidden_output.buffered(), code) == null);
    var frame = try write.Scene.quietFrame(gpa, .{
        .columns = 40,
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
}
