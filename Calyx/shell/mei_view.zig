// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

const std = @import("std");
const leaf = @import("leaf");
const write = leaf.write;
const viewport = leaf.viewport;
const menu_mod = @import("menu.zig");

pub const Hit = viewport.Hit;

pub const Fold = enum(u2) { time, source, output, both };

pub const Card = struct {
    code: []u8,
    output: []u8,
    fold: Fold = .time,

    pub fn cycle(self: *Card) void {
        self.fold = @fromBackingInt(@intCast(@as(u2, @backingInt(self.fold)) +% 1));
    }
};

const Verb = struct {
    spelling: []const u8,
    takes: []const u8,
    summary: []const u8,
    then: menu_mod.Then,
};

const verbs = [_]Verb{
    .{ .spelling = "/new", .takes = "", .summary = "start a new session", .then = .carry },
    .{ .spelling = "/resume", .takes = "[jsonl]", .summary = "resume a session", .then = .wait },
    .{ .spelling = "/effect", .takes = "", .summary = "cycle effort for the next run", .then = .carry },
    .{ .spelling = "/quit", .takes = "", .summary = "leave Mei", .then = .carry },
};

pub fn wantsMenu(typed: []const u8) bool {
    if (typed.len == 0 or typed[0] != '/') return false;
    if (std.mem.indexOfScalar(u8, typed, '\n') != null) return false;
    return std.mem.indexOfScalar(u8, std.mem.trimEnd(u8, typed, " "), ' ') == null;
}

pub fn commands(allocator: std.mem.Allocator, typed: []const u8) ![]menu_mod.Fit {
    var out: std.ArrayList(menu_mod.Fit) = .empty;
    errdefer {
        for (out.items) |fit| allocator.free(fit.line);
        out.deinit(allocator);
    }
    for (verbs) |verb| {
        if (!matches(verb.spelling, typed)) continue;
        const line = if (verb.takes.len == 0)
            try allocator.dupe(u8, verb.spelling)
        else
            try std.fmt.allocPrint(allocator, "{s} ", .{verb.spelling});
        try out.append(allocator, .{
            .line = line,
            .spelling = verb.spelling,
            .takes = verb.takes,
            .summary = verb.summary,
            .then = verb.then,
        });
    }
    return try out.toOwnedSlice(allocator);
}

fn matches(spelling: []const u8, typed: []const u8) bool {
    const token = std.mem.trim(u8, typed, " ");
    if (token.len == 0) return true;
    return std.mem.startsWith(u8, spelling, token);
}

const shown_body = 12_000;

pub fn render(
    gpa: std.mem.Allocator,
    log: []const write.Entry,
    cards: []const Card,
    focus: usize,
    draft: []const u8,
    region: write.Live,
    columns: u16,
    rows: u16,
    dim: bool,
    hits: *std.ArrayList(Hit),
) ![]u8 {
    var arena_state = std.heap.ArenaAllocator.init(gpa);
    defer arena_state.deinit();
    const arena = arena_state.allocator();
    var entries: std.ArrayList(write.Entry) = .empty;
    var marks: std.ArrayList(u16) = .empty;
    var card_i: usize = 0;
    for (log) |entry| {
        if (entry == .tool and std.mem.eql(u8, entry.tool.name, "python") and card_i < cards.len) {
            const card = &cards[card_i];
            var tool = entry.tool;
            tool.subject = foldWord(card.fold, card_i == focus);
            try entries.append(arena, .{ .tool = tool });
            try marks.append(arena, std.math.cast(u16, card_i) orelse viewport.no_card);
            if (card.fold == .source or card.fold == .both) {
                try entries.append(arena, .{ .note = try tail(arena, card.code) });
                try marks.append(arena, std.math.cast(u16, card_i) orelse viewport.no_card);
            }
            if (card.fold == .output or card.fold == .both) {
                try entries.append(arena, .{ .note = try tail(arena, card.output) });
                try marks.append(arena, std.math.cast(u16, card_i) orelse viewport.no_card);
            }
            card_i += 1;
        } else {
            try entries.append(arena, entry);
            try marks.append(arena, viewport.no_card);
        }
    }
    if (draft.len > 0) {
        try entries.append(arena, .{ .reply = draft });
        try marks.append(arena, viewport.no_card);
    }
    return viewport.draw(gpa, entries.items, marks.items, region, columns, rows, false, dim, hits);
}

fn foldWord(fold: Fold, focused: bool) []const u8 {
    return switch (fold) {
        .time => if (focused) "▸ time" else "time",
        .source => if (focused) "▸ source" else "source",
        .output => if (focused) "▸ output" else "output",
        .both => if (focused) "▸ both" else "both",
    };
}

fn tail(arena: std.mem.Allocator, text: []const u8) ![]const u8 {
    if (text.len <= shown_body) return text;
    var at = text.len - shown_body;
    while (at < text.len and text[at] & 0xC0 == 0x80) at += 1;
    var out: std.ArrayList(u8) = .empty;
    try out.appendSlice(arena, "…\n");
    try out.appendSlice(arena, text[at..]);
    return try out.toOwnedSlice(arena);
}

test "slash menu lists mei commands and filters aliases" {
    const fits = try commands(std.testing.allocator, "/");
    defer menu_mod.freeVerbs(std.testing.allocator, fits);
    try std.testing.expectEqual(@as(usize, 4), fits.len);
    try std.testing.expectEqualStrings("/new", fits[0].spelling);
    try std.testing.expect(wantsMenu("/"));
    try std.testing.expect(wantsMenu("/r"));
    try std.testing.expect(wantsMenu("/resume "));
    try std.testing.expect(!wantsMenu("/resume old.jsonl"));
    try std.testing.expect(!wantsMenu("hello"));
    const narrowed = try commands(std.testing.allocator, "/q");
    defer menu_mod.freeVerbs(std.testing.allocator, narrowed);
    try std.testing.expectEqual(@as(usize, 1), narrowed.len);
    try std.testing.expectEqualStrings("/quit", narrowed[0].spelling);
    const effect = try commands(std.testing.allocator, "/e");
    defer menu_mod.freeVerbs(std.testing.allocator, effect);
    try std.testing.expectEqualStrings("/effect", effect[0].spelling);
}

test "a python card defaults to elapsed time and cycles four ways" {
    const gpa = std.testing.allocator;
    const code = try gpa.dupe(u8, "print(1)");
    defer gpa.free(code);
    const output_text = try gpa.dupe(u8, "RESULT");
    defer gpa.free(output_text);
    var card = Card{ .code = code, .output = output_text };
    const log = [_]write.Entry{
        .{ .tool = .{
            .at = null,
            .name = "python",
            .subject = "",
            .took_us = 1_500_000,
            .outcome = .Answered,
        } },
    };
    var hits: std.ArrayList(Hit) = .empty;
    defer hits.deinit(gpa);
    const region: write.Live = .{
        .composer = .{
            .typed = "",
            .cursor = 0,
            .placeholder = "to Mei…",
            .room = "Mei",
            .offer = "",
        },
    };
    const timed = try render(gpa, &log, &.{card}, 0, "", region, 80, 24, false, &hits);
    defer gpa.free(timed);
    try std.testing.expect(std.mem.indexOf(u8, timed, "1.5 s") != null);
    try std.testing.expect(std.mem.indexOf(u8, timed, "print(1)") == null);
    try std.testing.expect(std.mem.indexOf(u8, timed, "/web") == null);
    card.cycle();
    const source = try render(gpa, &log, &.{card}, 0, "", region, 80, 24, false, &hits);
    defer gpa.free(source);
    try std.testing.expect(std.mem.indexOf(u8, source, "print(1)") != null);
    try std.testing.expect(std.mem.indexOf(u8, source, "RESULT") == null);
    card.cycle();
    const output = try render(gpa, &log, &.{card}, 0, "", region, 80, 24, false, &hits);
    defer gpa.free(output);
    try std.testing.expect(std.mem.indexOf(u8, output, "print(1)") == null);
    try std.testing.expect(std.mem.indexOf(u8, output, "RESULT") != null);
    card.cycle();
    const both = try render(gpa, &log, &.{card}, 0, "", region, 80, 24, false, &hits);
    defer gpa.free(both);
    try std.testing.expect(std.mem.indexOf(u8, both, "print(1)") != null);
    try std.testing.expect(std.mem.indexOf(u8, both, "RESULT") != null);
    card.cycle();
    try std.testing.expectEqual(Fold.time, card.fold);
}

test "the user line stays in the frame above a long reply" {
    const gpa = std.testing.allocator;
    const log = [_]write.Entry{
        .{ .you = .{ .at = null, .said = "用户的问题" } },
        .{ .reply = "第一段答复，写得很长也很完整，窗口里仍然能看见上面的提问。" },
    };
    var hits: std.ArrayList(Hit) = .empty;
    defer hits.deinit(gpa);
    const region: write.Live = .{
        .composer = .{
            .typed = "",
            .cursor = 0,
            .placeholder = "to Mei…",
            .room = "Mei",
            .offer = "model · effort high",
        },
    };
    const bytes = try render(gpa, &log, &.{}, 0, "还在写", region, 80, 24, false, &hits);
    defer gpa.free(bytes);
    try std.testing.expect(std.mem.indexOf(u8, bytes, "用户的问题") != null);
    try std.testing.expect(std.mem.indexOf(u8, bytes, "还在写") != null);
    try std.testing.expect(std.mem.indexOf(u8, bytes, "to Mei…") != null);
}
