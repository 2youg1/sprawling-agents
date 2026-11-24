// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

const std = @import("std");
const part = @import("part.zig");
const scene = @import("scene.zig");
const paint = @import("paint.zig");
const format = @import("format.zig");
const flow = @import("flow.zig");
const prose = @import("prose.zig");
const text = @import("text.zig");
const Grid = @import("grid.zig").Grid;

pub const Error = scene.Error || paint.Error;

pub fn spaced(previous: ?part.Part, next: part.Part) bool {
    const before = previous orelse return false;
    return switch (next) {
        .You, .Head, .Reply => true,
        else => before == .Reply,
    };
}

pub fn entry(p: *paint.Painter, r: *scene.Reader, tag: part.Part, grid: Grid) Error!void {
    switch (tag) {
        .Banner => try banner(p, r),
        .You => try you(p, r, grid),
        .Head => try head(p, r, grid),
        .Reasoning => try reasoning(p, r, grid),
        .Tool => try tool(p, r, grid),
        .Reply => try reply(p, r, grid),
        .Note => try note(p, r, grid),
        .Ended => try ended(p, r, grid),
        .Resolved => try resolved(p, r, grid),
        .Inline, .Quiet, .Waiting, .Working, .Calling, .Asking, .Composer, .Menu, .Replying => return error.Malformed,
    }
    try p.end();
}

const MarkStyle = union(enum) { styled: paint.Style, gray: u8 };

fn lead(p: *paint.Painter, grid: Grid, seconds: u32, mark: []const u8, style: MarkStyle) paint.Error!void {
    if (grid.timed and seconds != scene.no_time) {
        var b: format.Buffer = undefined;
        try p.fit(.faint, format.clock(&b, seconds), 8);
    }
    try p.to(grid.mark);
    switch (style) {
        .styled => |styled| try p.fit(styled, mark, 1),
        .gray => |tone| try p.grayMark(mark, tone),
    }
    try p.textLeft(grid.words);
}

fn banner(p: *paint.Painter, r: *scene.Reader) Error!void {
    try bannerChoice(p, r, true);
}

pub fn bannerChoice(p: *paint.Painter, r: *scene.Reader, web: bool) Error!void {
    const city = try r.text();
    const address = try r.text();
    const url = try r.text();
    if (p.mei and url.len == 0) {
        const metadata = text.width(address);
        const available = p.room();
        const path_room = @max(available / 3, available -| (metadata + 3));
        const workspace = if (text.width(city) <= path_room) city else std.fs.path.basename(city);
        try p.fit(.plain, workspace, path_room);
        if (metadata != 0 and p.room() > 3) {
            try p.cut(.faint, " · ", 3);
            try p.fit(.faint, address, p.room());
        }
        return;
    }
    const instruction = "  Type ";
    const command = "/web";
    const action = " to open the WebUI";
    const hint = instruction.len + command.len + action.len;
    try p.fit(.bold, "sprawling", 9);
    try p.to(11);
    const room = p.room() -| (text.width(city) + 2);
    const right = text.width(address) + hint;
    const shown_hint = web and room >= right;
    const shown_address = room >= text.width(address);
    try p.fit(.faint, city, p.room());
    if (!shown_address) return;
    try p.to(p.limit - if (shown_hint) right else text.width(address));
    try p.link(.faint, url, address);
    if (shown_hint) {
        try p.fit(.faint, instruction, instruction.len);
        try p.fit(.bold, command, command.len);
        try p.fit(.faint, action, action.len);
    }
}

fn you(p: *paint.Painter, r: *scene.Reader, grid: Grid) Error!void {
    const at = try r.word();
    const said = try r.text();
    try p.raw("\x1b]133;A\x1b\\");
    try lead(p, grid, at, "›", .{ .styled = .faint });
    try flow.sourced(p, said, .{ .left = grid.words, .measure = grid.limit -| grid.words, .style = .plain, .marks = .plain });
}

fn head(p: *paint.Painter, r: *scene.Reader, grid: Grid) Error!void {
    const at = try r.word();
    const resident = try r.text();
    const facts = try r.word();
    try lead(p, grid, at, "●", .{ .styled = .faint });
    try p.fit(.bold, resident, p.room());
    var gap: []const u8 = "  ";
    for (0..facts) |_| {
        const fact = try r.text();
        try p.fit(.faint, gap, gap.len);
        try p.fit(.faint, fact, p.room());
        gap = " · ";
    }
}

fn reasoning(p: *paint.Painter, r: *scene.Reader, grid: Grid) Error!void {
    const chars = try r.long();
    var b: format.Buffer = undefined;
    try p.textLeft(grid.words);
    try p.fit(.faint, "▸ reasoning ", 12);
    try p.fit(.faint, format.count(&b, chars), p.room());
    try p.fit(.faint, " characters", p.room());
}

fn tool(p: *paint.Painter, r: *scene.Reader, grid: Grid) Error!void {
    const at = try r.word();
    const name = try r.text();
    const subject = try r.text();
    const micros = try r.long();
    const outcome = try r.outcome();
    var b: format.Buffer = undefined;
    const took = if (micros == scene.no_count) "" else format.took(&b, micros);
    const mark = if (!p.mei) "◇" else if (outcome == .Failed) "×" else "✓";
    const style: MarkStyle = if (p.mei) .{ .gray = p.completion_tone orelse paint.settled_tool_tone } else .{ .styled = .faint };
    try lead(p, grid, at, mark, style);
    try p.fit(.plain, name, 8);
    try p.to(grid.words + 9);
    const figures = grid.figures();
    const took_width = text.width(took);
    try p.fit(.plain, subject, (figures -| (p.column + took_width + 2)));
    try p.to(figures -| took_width);
    try p.fit(.faint, took, took_width);
    switch (outcome) {
        .Answered => {},
        .Failed => if (!p.mei) {
            try p.to(p.column + 2);
            try p.fit(.alert, "failed", 6);
        },
    }
}

fn reply(p: *paint.Painter, r: *scene.Reader, grid: Grid) Error!void {
    const said = try r.text();
    try prose.blocks(p, std.mem.trim(u8, said, "\n"), .{ .left = grid.words, .measure = grid.measure(), .style = .plain, .marks = .prose });
}

fn note(p: *paint.Painter, r: *scene.Reader, grid: Grid) Error!void {
    const said = try r.text();
    try flow.sourced(p, std.mem.trimEnd(u8, said, "\n"), .{ .left = grid.words, .measure = grid.limit -| grid.words, .style = .faint, .marks = .kept });
}

fn ended(p: *paint.Painter, r: *scene.Reader, grid: Grid) Error!void {
    const at = try r.word();
    const ending = try r.ending();
    const seconds = try r.long();
    const word: []const u8, const style: paint.Style = switch (ending) {
        .Done => .{ "done", .faint },
        .Limit => .{ "stopped at a limit", .alert },
        .Cancelled => .{ "cancelled", .faint },
        .Failed => .{ "failed", .alert },
    };
    try lead(p, grid, at, "■", .{ .styled = style });
    try p.fit(style, word, p.room());
    if (seconds != scene.no_count) {
        var b: format.Buffer = undefined;
        try p.fit(.faint, " · ", 3);
        try p.fit(.faint, format.span(&b, seconds), p.room());
    }
}

fn resolved(p: *paint.Painter, r: *scene.Reader, grid: Grid) Error!void {
    const at = try r.word();
    const verdict = try r.verdict();
    const what = try r.text();
    const mark: []const u8, const word: []const u8 = switch (verdict) {
        .Approved => .{ "✓", "approved  " },
        .Denied => .{ "✗", "denied  " },
    };
    try lead(p, grid, at, mark, .{ .styled = .faint });
    try p.fit(.faint, word, word.len);
    try p.fit(.faint, what, p.room());
}
