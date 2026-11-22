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
const Grid = @import("grid.zig").Grid;

pub const Error = transcript.Error;

pub const Cursor = struct { row: usize, column: usize };

const fade = "╌╌╌┄┄┄";

pub const menu_rows = 5;

pub fn region(p: *paint.Painter, r: *scene.Reader, grid: Grid) Error!Cursor {
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
                cursor = try composer(p, r, grid);
            },
            .Inline, .Quiet, .Banner, .You, .Head, .Reasoning, .Tool, .Reply, .Note, .Ended, .Resolved, .Menu => return error.Malformed,
        }
        drew_status = true;
    }
    try p.close();
    return cursor;
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
    try flow.flow(p, std.mem.trim(u8, said, "\n"), .{ .left = grid.words, .measure = grid.measure(), .style = .plain, .marks = .prose });
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
        try p.to(left);
        try p.fit(.faint, room, p.room() -| (text.width(offer) + 2));
        if (offer.len > 0 and p.room() >= text.width(offer) + 2) {
            try p.to(p.limit - text.width(offer));
            try p.fit(.faint, offer, p.room());
        }
    }
    return cursor;
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
