// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// The rows under the transcript that are drawn again on every frame:
// what waits for the person, who is working, the calls under way, the
// question `/quit` asks, and the composer. The composer is laid out as
// the WebUI's is - the words, a line under them that runs as far as the
// words do and then fades, and the room and the offer under the line -
// with the slash menu taking the place of that last row while it is
// open. Nothing here moves on its own: a frame is drawn because
// something changed, so a terminal that is left alone stays still.

const std = @import("std");
const part = @import("part.zig");
const scene = @import("scene.zig");
const paint = @import("paint.zig");
const text = @import("text.zig");
const format = @import("format.zig");
const transcript = @import("transcript.zig");
const Grid = @import("grid.zig").Grid;

pub const Error = transcript.Error;

/// Where the cursor belongs once the live region is drawn, counted from
/// the region's first row.
pub const Cursor = struct { row: usize, column: usize };

/// The fading end of the composer's line.
const fade = "╌╌╌┄┄┄";

/// How many rows the slash menu lists at once.
pub const menu_rows = 5;

/// Draws every live part left in `r`, the first row being the blank one
/// that parts the region from the transcript, and answers where the
/// cursor goes. The last row is left unterminated.
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

/// The right end of a row: `keys` placed so they end on the last column.
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
    try p.to(grid.mark);
    try p.fit(.accent, "●", 1);
    try p.to(grid.words);
    try p.fit(.plain, resident, p.room() -| 30);
    try p.fit(.faint, " is working", 11);
    if (since != scene.no_time) {
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

/// The composer, and under it the settings row or the slash menu.
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

/// The line under the words: solid as far as the words run, at least
/// two columns, then a fade of thinner dashes. While the line is empty
/// the solid stretch is faint too, as the WebUI's line drops to half
/// opacity, so the words written read like progress.
fn underline(p: *paint.Painter, left: usize, words: usize, solid_style: paint.Style) paint.Error!void {
    try p.to(left);
    const solid = @max(words, 2);
    var drawn: usize = 0;
    while (drawn < solid and p.room() > 0) : (drawn += 1) {
        _ = try p.char(solid_style, text.char("─", 0));
    }
    try p.cut(.faint, fade, p.room());
}

/// The slash menu: up to `menu_rows` entries around the cursor, the one
/// under the cursor marked with the accent bar, and a last row that says
/// how many more there are and which keys move through them.
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

/// The widest spelling and its arguments among the entries ahead of
/// `r`, read from a copy so the entries are still there to draw, and
/// never so wide that no room is left for what each one does.
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
