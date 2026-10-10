// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// The lines that go into the terminal's own scrollback and stay there:
// the banner, what the person said, each run's head, its reasoning, its
// tool calls with how long each took, its reply, and how it ended. Each
// is drawn once, so search, selection and copying are the terminal's.
//
// A line the person typed is marked as a prompt (OSC 133), so a terminal
// that keeps those marks can jump from one thing said to the next.

const std = @import("std");
const part = @import("part.zig");
const scene = @import("scene.zig");
const paint = @import("paint.zig");
const format = @import("format.zig");
const flow = @import("flow.zig");
const text = @import("text.zig");
const Grid = @import("grid.zig").Grid;

pub const Error = scene.Error || paint.Error;

/// Whether a blank line stands before `next` when `previous` was the
/// last line drawn: before each thing said, each head and each reply,
/// and after each reply.
pub fn spaced(previous: ?part.Part, next: part.Part) bool {
    const before = previous orelse return false;
    return switch (next) {
        .You, .Head, .Reply => true,
        else => before == .Reply,
    };
}

/// Draws the entry `tag` names, read from `r`, ending its last line.
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
        .Inline, .Quiet, .Waiting, .Working, .Calling, .Asking, .Composer, .Menu => return error.Malformed,
    }
    try p.end();
}

/// The time gutter, then the mark: the start of every line with a moment.
pub fn lead(p: *paint.Painter, grid: Grid, seconds: u32, mark: []const u8, style: paint.Style) paint.Error!void {
    if (grid.timed and seconds != scene.no_time) {
        var b: format.Buffer = undefined;
        try p.fit(.faint, format.clock(&b, seconds), 8);
    }
    try p.to(grid.mark);
    try p.fit(style, mark, 1);
    try p.to(grid.words);
}

/// The first line: the city on the left, and on the right where its
/// page is served and how to open it, the hint going first and then the
/// address when the window is too narrow for them.
fn banner(p: *paint.Painter, r: *scene.Reader) Error!void {
    const city = try r.text();
    const address = try r.text();
    const url = try r.text();
    const hint = "  /web opens the page";
    try p.fit(.bold, "sprawling", 9);
    try p.to(11);
    const room = p.room() -| (text.width(city) + 2);
    const right = text.width(address) + hint.len;
    const shown_hint = room >= right;
    const shown_address = room >= text.width(address);
    try p.fit(.faint, city, p.room());
    if (!shown_address) return;
    try p.to(p.limit - if (shown_hint) right else text.width(address));
    try p.link(.faint, url, address);
    if (shown_hint) try p.fit(.faint, hint, hint.len);
}

fn you(p: *paint.Painter, r: *scene.Reader, grid: Grid) Error!void {
    const at = try r.word();
    const said = try r.text();
    try p.raw("\x1b]133;A\x1b\\");
    try lead(p, grid, at, "›", .faint);
    try flow.flow(p, said, .{ .left = grid.words, .measure = grid.limit -| grid.words, .style = .plain, .marks = .plain });
}

fn head(p: *paint.Painter, r: *scene.Reader, grid: Grid) Error!void {
    const at = try r.word();
    const resident = try r.text();
    const facts = try r.word();
    try lead(p, grid, at, "●", .faint);
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
    try p.to(grid.words);
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
    try lead(p, grid, at, "◇", .faint);
    try p.fit(.plain, name, 8);
    try p.to(grid.words + 9);
    const figures = grid.figures();
    const took_width = text.width(took);
    try p.fit(.plain, subject, (figures -| (p.column + took_width + 2)));
    try p.to(figures -| took_width);
    try p.fit(.faint, took, took_width);
    switch (outcome) {
        .Answered => {},
        .Failed => {
            try p.to(p.column + 2);
            try p.fit(.alert, "failed", 6);
        },
    }
}

fn reply(p: *paint.Painter, r: *scene.Reader, grid: Grid) Error!void {
    const said = try r.text();
    try flow.flow(p, std.mem.trim(u8, said, "\n"), .{ .left = grid.words, .measure = grid.measure(), .style = .plain, .marks = .prose });
}

fn note(p: *paint.Painter, r: *scene.Reader, grid: Grid) Error!void {
    const said = try r.text();
    try flow.flow(p, std.mem.trimEnd(u8, said, "\n"), .{ .left = grid.words, .measure = grid.limit -| grid.words, .style = .faint, .marks = .kept });
}

fn ended(p: *paint.Painter, r: *scene.Reader, grid: Grid) Error!void {
    const at = try r.word();
    const ending = try r.ending();
    const seconds = try r.long();
    const word: []const u8, const style: paint.Style = switch (ending) {
        .Done => .{ "done", .faint },
        .Limit => .{ "stopped at a limit", .alert },
        .Cancelled => .{ "cancelled", .faint },
    };
    try lead(p, grid, at, "■", style);
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
    try lead(p, grid, at, mark, .faint);
    try p.fit(.faint, word, word.len);
    try p.fit(.faint, what, p.room());
}
