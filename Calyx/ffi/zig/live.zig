// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

const std = @import("std");
const part = @import("part.zig");
const scene = @import("scene.zig");
const paint = @import("paint.zig");
const text = @import("text.zig");
const format = @import("format.zig");
const transcript = @import("transcript.zig");
const flow = @import("flow.zig");
const prose = @import("prose.zig");
const Grid = @import("grid.zig").Grid;

pub const Error = transcript.Error;

pub const Cursor = struct { row: usize, column: usize };

const fade = "╌╌╌┄┄┄";

pub const menu_rows = 5;

pub const TextWindow = struct {
    skip: usize = 0,
    rows: usize = std.math.maxInt(usize),
};

pub const TypedShape = struct { rows: usize, cursor: usize };

pub fn region(p: *paint.Painter, r: *scene.Reader, grid: Grid) Error!Cursor {
    return regionShown(p, r, grid, .{});
}

pub fn regionShown(p: *paint.Painter, r: *scene.Reader, grid: Grid, win: TextWindow) Error!Cursor {
    var cursor: Cursor = .{ .row = p.row, .column = grid.mark };
    var drew_status = false;
    while (!r.done()) {
        const tag = try r.tag();
        try p.end();
        switch (tag) {
            .Waiting => try waiting(p, r, grid),
            .Working => try working(p, r, grid),
            .Calling => try calling(p, r, grid),
            .Asking => try asking(p, r, grid),
            .Replying => try replying(p, r, grid),
            .Composer => {
                if (drew_status) try p.end();
                cursor = try composerShown(p, r, grid, win);
            },
            .Inline, .Quiet, .Banner, .You, .Head, .Reasoning, .Tool, .Reply, .Note, .Ended, .Resolved, .Menu => return error.Malformed,
        }
        drew_status = true;
    }
    try p.close();
    return cursor;
}

pub fn typedShape(typed: []const u8, at: usize, columns: usize) TypedShape {
    const limit = @max(columns, 2) - 1;
    const left: usize = if (columns >= 60) 10 else 0;
    const room = limit -| left;
    if (typed.len == 0 or room == 0) return .{ .rows = 1, .cursor = 0 };
    var row: usize = 0;
    var col: usize = 0;
    var cursor_row: usize = 0;
    var index: usize = 0;
    var byte: usize = 0;
    while (byte < typed.len) {
        if (index == at) cursor_row = row;
        if (typed[byte] == '\n') {
            row += 1;
            col = 0;
            byte += 1;
            index += 1;
            continue;
        }
        const c = text.char(typed, byte);
        if (col > 0 and col + c.columns > room) {
            row += 1;
            col = 0;
        }
        col += c.columns;
        byte += c.len;
        index += 1;
    }
    if (index <= at) cursor_row = row;
    return .{ .rows = row + 1, .cursor = cursor_row };
}

fn right(p: *paint.Painter, keys: []const Key) paint.Error!void {
    var total: usize = 0;
    for (keys) |key| total += text.width(key.word) + key.gap;
    if (p.room() < total + 2) return;
    try p.to(p.limit - total);
    for (keys) |key| {
        try p.fit(key.style, key.word, p.room());
        try p.to(p.column + key.gap);
    }
}

const Key = struct { word: []const u8, style: paint.Style, gap: usize = 0 };

fn waiting(p: *paint.Painter, r: *scene.Reader, grid: Grid) Error!void {
    const what = try r.text();
    const more = try r.word();
    try p.to(grid.mark);
    try p.fit(.alert_bold, "?", 1);
    try p.to(grid.words);
    const keys = [_]Key{
        .{ .word = "y", .style = .alert_bold },
        .{ .word = " approve", .style = .faint, .gap = 3 },
        .{ .word = "n", .style = .alert_bold },
        .{ .word = " deny", .style = .faint },
    };
    var later: format.Buffer = undefined;
    const tail = if (more == 0) "" else std.fmt.bufPrint(&later, "  +{d} more", .{more}) catch "";
    try p.fit(.plain, what, p.room() -| (text.width(tail) + 22));
    try p.fit(.faint, tail, text.width(tail));
    try right(p, &keys);
}

fn working(p: *paint.Painter, r: *scene.Reader, grid: Grid) Error!void {
    const resident = try r.text();
    const since = try r.word();
    const doing = " is working";
    const since_width = 17;
    try p.to(grid.mark);
    try p.fit(.accent, "●", 1);
    try p.to(grid.words);
    const named = @min(text.width(resident), p.room() -| doing.len);
    const timed = since != scene.no_time and p.room() >= named + doing.len + since_width;
    try p.fit(.plain, resident, named);
    try p.fit(.faint, doing, doing.len);
    if (timed) {
        var b: format.Buffer = undefined;
        try p.fit(.faint, " · since ", 9);
        try p.fit(.faint, format.clock(&b, since), 8);
    }
    try right(p, &.{ .{ .word = "esc", .style = .plain }, .{ .word = " stops it", .style = .faint } });
}

fn calling(p: *paint.Painter, r: *scene.Reader, grid: Grid) Error!void {
    const name = try r.text();
    const subject = try r.text();
    try p.to(grid.mark);
    try p.fit(.faint, "◇", 1);
    try p.to(grid.words);
    try p.fit(.plain, name, 8);
    try p.to(grid.words + 9);
    const running = "running";
    try p.fit(.plain, subject, grid.figures() -| (p.column + running.len + 2));
    try p.to(grid.figures() -| running.len);
    try p.fit(.faint, running, running.len);
}

fn asking(p: *paint.Painter, r: *scene.Reader, grid: Grid) Error!void {
    const runs = try r.long();
    var b: format.Buffer = undefined;
    const said = std.fmt.bufPrint(&b, "{d} {s} going", .{ runs, if (runs == 1) "run is" else "runs are" }) catch "";
    try p.to(grid.mark);
    try p.fit(.alert_bold, "?", 1);
    try p.to(grid.words);
    try p.fit(.plain, said, p.room());
    try right(p, &.{
        .{ .word = "enter", .style = .alert_bold },
        .{ .word = " waits", .style = .faint, .gap = 3 },
        .{ .word = "n", .style = .alert_bold },
        .{ .word = " stops them now", .style = .faint, .gap = 3 },
        .{ .word = "esc", .style = .alert_bold },
        .{ .word = " keeps serving", .style = .faint },
    });
}

fn replying(p: *paint.Painter, r: *scene.Reader, grid: Grid) Error!void {
    const said = try r.text();
    try prose.blocks(p, std.mem.trim(u8, said, "\n"), .{ .left = grid.words, .measure = grid.measure(), .style = .plain, .marks = .prose });
}

fn showing(win: TextWindow, row: usize) bool {
    return row >= win.skip and row - win.skip < win.rows;
}

fn composerShown(p: *paint.Painter, r: *scene.Reader, grid: Grid, win: TextWindow) Error!Cursor {
    if (win.skip == 0 and win.rows == std.math.maxInt(usize)) return composer(p, r, grid);
    const typed = try r.text();
    const at = try r.word();
    const placeholder = try r.text();
    const room_name = try r.text();
    const offer = try r.text();
    const left = grid.mark;
    const width = p.limit -| left;
    try p.to(left);
    var cursor: Cursor = .{ .row = p.row, .column = left };
    var widest: usize = 0;
    var text_row: usize = 0;
    var col: usize = 0;
    if (typed.len == 0) {
        if (showing(win, 0)) try p.fit(.faint, placeholder, p.room());
    } else {
        var index: usize = 0;
        var byte: usize = 0;
        while (byte < typed.len) {
            const visible = showing(win, text_row);
            if (index == at and visible) cursor = .{ .row = p.row, .column = p.column };
            if (typed[byte] == '\n') {
                if (visible) {
                    widest = @max(widest, col);
                    try p.end();
                }
                text_row += 1;
                col = 0;
                if (showing(win, text_row)) try p.to(left);
                byte += 1;
                index += 1;
                continue;
            }
            const c = text.char(typed, byte);
            if (col > 0 and col + c.columns > width) {
                if (visible) {
                    widest = @max(widest, col);
                    try p.end();
                }
                text_row += 1;
                col = 0;
                if (showing(win, text_row)) try p.to(left);
            }
            if (showing(win, text_row)) _ = try p.char(.plain, c);
            col += c.columns;
            byte += c.len;
            index += 1;
        }
        if (index <= at and showing(win, text_row)) cursor = .{ .row = p.row, .column = p.column };
        widest = @max(widest, col);
    }
    try p.end();
    try underline(p, left, widest, if (typed.len > 0) .plain else .faint);
    try p.end();
    if (!r.done() and r.bytes[r.at] == @backingInt(part.Part.Menu)) {
        _ = try r.tag();
        try menu(p, r, left);
    } else {
        try statusRow(p, left, room_name, offer);
    }
    return cursor;
}

fn composer(p: *paint.Painter, r: *scene.Reader, grid: Grid) Error!Cursor {
    const typed = try r.text();
    const at = try r.word();
    const placeholder = try r.text();
    const room = try r.text();
    const offer = try r.text();
    const left = grid.mark;
    try p.to(left);
    var cursor: Cursor = .{ .row = p.row, .column = left };
    var widest: usize = 0;
    if (typed.len == 0) {
        try p.fit(.faint, placeholder, p.room());
    } else {
        var index: usize = 0;
        var byte: usize = 0;
        while (byte < typed.len) : (index += 1) {
            if (index == at) cursor = .{ .row = p.row, .column = p.column };
            if (typed[byte] == '\n') {
                widest = @max(widest, p.column - left);
                try p.end();
                try p.to(left);
                byte += 1;
                continue;
            }
            const c = text.char(typed, byte);
            if (!try p.char(.plain, c)) {
                widest = @max(widest, p.column - left);
                try p.end();
                try p.to(left);
                _ = try p.char(.plain, c);
            }
            byte += c.len;
        }
        if (index <= at) cursor = .{ .row = p.row, .column = p.column };
    }
    if (typed.len > 0) widest = @max(widest, p.column - left);
    try p.end();
    try underline(p, left, widest, if (typed.len > 0) .plain else .faint);
    try p.end();
    if (!r.done() and r.bytes[r.at] == @backingInt(part.Part.Menu)) {
        _ = try r.tag();
        try menu(p, r, left);
    } else {
        try statusRow(p, left, room, offer);
    }
    return cursor;
}

fn statusRow(p: *paint.Painter, left: usize, room: []const u8, offer: []const u8) paint.Error!void {
    try p.to(left);
    const avail = p.room();
    if (offer.len == 0 or avail == 0) {
        try p.fit(.faint, room, avail);
        return;
    }
    const room_w = text.width(room);
    const offer_w = text.width(offer);
    const room_budget = @min(room_w, @max(@as(usize, 1), avail / 4));
    const offer_budget = if (offer_w + room_budget + 2 <= avail)
        offer_w
    else
        avail -| (room_budget + 2);
    const room_fit = if (offer_budget > 0) avail -| (offer_budget + 2) else avail;
    try p.fit(.faint, room, room_fit);
    if (offer_budget > 0) {
        try p.to(p.limit - offer_budget);
        try p.fit(.faint, offer, offer_budget);
    }
}

fn underline(p: *paint.Painter, left: usize, words: usize, solid_style: paint.Style) paint.Error!void {
    try p.to(left);
    const solid = @max(words, 2);
    var drawn: usize = 0;
    while (drawn < solid and p.room() > 0) : (drawn += 1) {
        _ = try p.char(solid_style, text.char("─", 0));
    }
    try p.cut(.faint, fade, p.room());
}

fn menu(p: *paint.Painter, r: *scene.Reader, left: usize) Error!void {
    const chosen = try r.word();
    const more = try r.word();
    const count = try r.word();
    const summary_at = left + 1 + try widestChoice(r.*, count) + 3;
    for (0..count) |index| {
        const spelling = try r.text();
        const takes = try r.text();
        const summary = try r.text();
        if (index > 0) try p.end();
        try p.to(left);
        const here = index == chosen;
        if (here) try p.fit(.accent, "▎", 1);
        try p.to(left + 1);
        try p.fit(if (here) .bold else .plain, spelling, p.room());
        if (takes.len > 0) {
            try p.to(p.column + 1);
            try p.fit(.faint, takes, p.room());
        }
        try p.to(summary_at);
        try p.fit(.faint, summary, p.room());
    }
    try p.end();
    try p.to(left + 1);
    var b: format.Buffer = undefined;
    if (more > 0) {
        try p.fit(.faint, std.fmt.bufPrint(&b, "{d} more · ", .{more}) catch "", p.room());
    }
    try p.fit(.faint, "tab moves · enter chooses · esc closes", p.room());
}

fn widestChoice(ahead: scene.Reader, count: u32) Error!usize {
    var r = ahead;
    var widest: usize = 0;
    for (0..count) |_| {
        const spelling = try r.text();
        const takes = try r.text();
        _ = try r.text();
        const takes_width = if (takes.len > 0) text.width(takes) + 1 else 0;
        widest = @max(widest, text.width(spelling) + takes_width);
    }
    return @min(widest, 28);
}

test "composer status row keeps room and truncates long offer across widths" {
    const offer = "provider/actual · effort high · Agent Step 12 · 12.34% · 123456/1048576";
    for ([_]usize{ 20, 40, 60, 80, 120, 200 }) |columns| {
        var buffer: [512]u8 = undefined;
        var out: std.Io.Writer = .fixed(&buffer);
        var p = paint.Painter.init(&out, columns);
        try statusRow(&p, 0, "Mei", offer);
        try std.testing.expect(p.column <= columns);
        const drawn = out.buffered();
        try std.testing.expect(std.mem.indexOf(u8, drawn, "Mei") != null);
        if (columns >= 80) {
            try std.testing.expect(std.mem.indexOf(u8, drawn, "Agent Step") != null);
        }
        if (text.width(offer) + text.width("Mei") + 2 > columns) {
            try std.testing.expect(std.mem.indexOf(u8, drawn, "…") != null);
        }
    }
}
