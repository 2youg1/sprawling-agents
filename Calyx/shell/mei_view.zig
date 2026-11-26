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
pub const View = viewport.View;
pub const Point = viewport.Point;
pub const Location = viewport.Location;
pub const Selection = viewport.Selection;
pub const workingPatch = viewport.workingPatch;

pub const Fold = enum(u2) { time, source, output, both };

pub const ToolCard = struct {
    code: []u8,
    output: []u8,
    fold: Fold = .time,

    fn cycle(self: *ToolCard) void {
        self.fold = @fromBackingInt(@intCast(@as(u2, @backingInt(self.fold)) +% 1));
    }
};

pub const Card = union(enum) {
    python: ToolCard,
    reasoning: bool,

    pub fn cycle(self: *Card) void {
        switch (self.*) {
            .python => |*python| python.cycle(),
            .reasoning => |*open| open.* = !open.*,
        }
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
    .{ .spelling = "/effect", .takes = "", .summary = "choose reasoning effort", .then = .carry },
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

pub const LogEntry = struct {
    item: write.Entry,
    card: ?usize = null,
};

const Window = struct {
    start: usize,
    end: usize,

    fn around(view: *const View, count: usize, rows: u16) Window {
        const anchor = if (view.anchor) |a| @min(a.origin.entry, count) else count;
        const margin = @as(usize, rows) + 2;
        var window = Window{
            .start = anchor -| margin -| (if (view.movement < 0) @as(usize, @abs(view.movement)) else 0),
            .end = @min(count, anchor +| margin +| (if (view.movement > 0) @as(usize, @intCast(view.movement)) else 0)),
        };
        if (view.selection) |range| {
            const ordered = range.ordered();
            window.start = @min(window.start, ordered.start.origin.entry -| 1);
            window.end = @max(window.end, @min(count, ordered.finish.origin.entry +| 2));
        }
        return window;
    }
};

pub fn render(
    gpa: std.mem.Allocator,
    log: []const LogEntry,
    cards: []const Card,
    focus: usize,
    draft: []const u8,
    region: write.Live,
    columns: u16,
    rows: u16,
    view: *View,
    hits: *std.ArrayList(Hit),
) ![]u8 {
    var arena_state = std.heap.ArenaAllocator.init(gpa);
    defer arena_state.deinit();
    const arena = arena_state.allocator();
    var entries: std.ArrayList(write.Entry) = .empty;
    var items: std.ArrayList(viewport.Item) = .empty;
    view.trackDraft(if (draft.len != 0) log.len else null);
    const window = Window.around(view, log.len, rows);
    if (window.start > 0 and log[0].item == .banner) {
        try entries.append(arena, log[0].item);
        try items.append(arena, .{ .origin = .{ .entry = 0 } });
    }
    for (log[window.start..window.end], window.start..) |record, index| {
        const entry = record.item;
        const card_i = record.card orelse {
            try entries.append(arena, entry);
            try items.append(arena, .{ .origin = .{ .entry = index } });
            continue;
        };
        const card_id = std.math.cast(u16, card_i) orelse viewport.no_card;
        if (cards[card_i] == .python) {
            const card = &cards[card_i].python;
            var tool = entry.tool;
            tool.subject = foldWord(card.fold, card_i == focus);
            try entries.append(arena, .{ .tool = tool });
            try items.append(arena, .{ .origin = .{ .entry = index }, .card = card_id });
            if (card.fold == .source or card.fold == .both) {
                try entries.append(arena, .{ .note = try tail(arena, card.code) });
                try items.append(arena, .{ .origin = .{ .entry = index, .fragment = .input } });
            }
            if (card.fold == .output or card.fold == .both) {
                try entries.append(arena, .{ .note = try tail(arena, card.output) });
                try items.append(arena, .{ .origin = .{ .entry = index, .fragment = .output } });
            }
        } else {
            const open = cards[card_i].reasoning;
            try entries.append(arena, if (open) .{ .note = "▾ reasoning" } else entry);
            try items.append(arena, .{ .origin = .{ .entry = index }, .card = card_id });
            if (open) {
                try entries.append(arena, .{ .note = entry.reasoning.body });
                try items.append(arena, .{ .origin = .{ .entry = index, .fragment = .thought } });
            }
        }
    }
    if (draft.len > 0 and window.end == log.len) {
        try entries.append(arena, .{ .reply = draft });
        try items.append(arena, .{ .origin = .{ .entry = log.len } });
    }
    return viewport.draw(gpa, entries.items, items.items, region, columns, rows, false, view, hits);
}

fn foldWord(fold: Fold, focused: bool) []const u8 {
    return switch (fold) {
        .time => if (focused) "· ▸" else "  ▸",
        .source => if (focused) "· ⇥" else "  ⇥",
        .output => if (focused) "· ⇤" else "  ⇤",
        .both => if (focused) "· ↔" else "  ↔",
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
    var card = Card{ .python = .{ .code = code, .output = output_text } };
    const log = [_]LogEntry{
        .{ .item = .{ .tool = .{
            .at = null,
            .name = "python",
            .subject = "",
            .took_us = 1_500_000,
            .outcome = .Answered,
        } }, .card = 0 },
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
    var view: View = .{};
    defer view.deinit(gpa);
    const timed = try render(gpa, &log, &.{card}, 0, "", region, 80, 24, &view, &hits);
    defer gpa.free(timed);
    try std.testing.expect(std.mem.indexOf(u8, timed, "1.5 s") != null);
    try std.testing.expect(std.mem.indexOf(u8, timed, "print(1)") == null);
    try std.testing.expect(std.mem.indexOf(u8, timed, "/web") == null);
    card.cycle();
    const source = try render(gpa, &log, &.{card}, 0, "", region, 80, 24, &view, &hits);
    defer gpa.free(source);
    try std.testing.expect(std.mem.indexOf(u8, source, "print(1)") != null);
    try std.testing.expect(std.mem.indexOf(u8, source, "RESULT") == null);
    card.cycle();
    const output = try render(gpa, &log, &.{card}, 0, "", region, 80, 24, &view, &hits);
    defer gpa.free(output);
    try std.testing.expect(std.mem.indexOf(u8, output, "print(1)") == null);
    try std.testing.expect(std.mem.indexOf(u8, output, "RESULT") != null);
    card.cycle();
    const both = try render(gpa, &log, &.{card}, 0, "", region, 80, 24, &view, &hits);
    defer gpa.free(both);
    try std.testing.expect(std.mem.indexOf(u8, both, "print(1)") != null);
    try std.testing.expect(std.mem.indexOf(u8, both, "RESULT") != null);
    card.cycle();
    try std.testing.expectEqual(Fold.time, card.python.fold);
}

test "the user line stays in the frame above a long reply" {
    const gpa = std.testing.allocator;
    const log = [_]LogEntry{
        .{ .item = .{ .you = .{ .at = null, .said = "用户的问题" } } },
        .{ .item = .{ .reply = "第一段答复，写得很长也很完整，窗口里仍然能看见上面的提问。" } },
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
    var view: View = .{};
    defer view.deinit(gpa);
    const bytes = try render(gpa, &log, &.{}, 0, "还在写", region, 80, 24, &view, &hits);
    defer gpa.free(bytes);
    try std.testing.expect(std.mem.indexOf(u8, bytes, "用户的问题") != null);
    try std.testing.expect(std.mem.indexOf(u8, bytes, "还在写") != null);
    try std.testing.expect(std.mem.indexOf(u8, bytes, "to Mei…") != null);
}

test "the visible reasoning card has a click target and discloses its original text" {
    const gpa = std.testing.allocator;
    const log = [_]LogEntry{.{ .item = .{ .reasoning = .{ .characters = 6, .body = "原样思考文本" } }, .card = 0 }};
    var card: Card = .{ .reasoning = false };
    var hits: std.ArrayList(Hit) = .empty;
    defer hits.deinit(gpa);
    var view: View = .{};
    defer view.deinit(gpa);
    const region: write.Live = .{ .composer = .{ .typed = "", .cursor = 0, .placeholder = "Mei", .room = "Mei", .offer = "" } };
    const closed = try render(gpa, &log, &.{card}, 0, "", region, 80, 24, &view, &hits);
    defer gpa.free(closed);
    try std.testing.expect(std.mem.indexOf(u8, closed, "原样思考文本") == null);
    try std.testing.expectEqual(@as(usize, 1), hits.items.len);
    card.cycle();
    const open = try render(gpa, &log, &.{card}, 0, "", region, 80, 24, &view, &hits);
    defer gpa.free(open);
    try std.testing.expect(std.mem.indexOf(u8, open, "原样思考文本") != null);
}

test "widening and opening cards above the viewport preserve its reading place" {
    const gpa = std.testing.allocator;
    const log = [_]LogEntry{
        .{ .item = .{ .tool = .{ .at = null, .name = "python", .subject = "", .took_us = null, .outcome = .Answered } }, .card = 0 },
        .{ .item = .{ .reply = "segment-00 segment-01 segment-02 segment-03 segment-04 segment-05 segment-06 segment-07 segment-08 segment-09 segment-10 segment-11 segment-12 segment-13 segment-14 segment-15" } },
    };
    var code = "visible on open".*;
    var cards = [_]Card{.{ .python = .{ .code = &code, .output = &.{} } }};
    var hits: std.ArrayList(Hit) = .empty;
    defer hits.deinit(gpa);
    var view: View = .{};
    defer view.deinit(gpa);
    const region: write.Live = .{ .composer = .{ .typed = "", .cursor = 0, .placeholder = "Mei", .room = "Mei", .offer = "" } };
    const bottom = try render(gpa, &log, &cards, 0, "", region, 22, 5, &view, &hits);
    defer gpa.free(bottom);
    view.scroll(-10);
    const narrow = try render(gpa, &log, &cards, 0, "", region, 22, 5, &view, &hits);
    defer gpa.free(narrow);
    const row = narrow[0..std.mem.indexOfScalar(u8, narrow, '\n').?];
    const at = std.mem.indexOf(u8, row, "segment-").?;
    const word = row[at..][0.."segment-00".len];
    const wide = try render(gpa, &log, &cards, 0, "", region, 31, 5, &view, &hits);
    defer gpa.free(wide);
    try std.testing.expect(std.mem.indexOf(u8, wide[0..std.mem.indexOfScalar(u8, wide, '\n').?], word) != null);
    cards[0].cycle();
    const open = try render(gpa, &log, &cards, 0, "", region, 31, 5, &view, &hits);
    defer gpa.free(open);
    try std.testing.expect(std.mem.indexOf(u8, open[0..std.mem.indexOfScalar(u8, open, '\n').?], word) != null);
}

test "a result wave animates both failure and success without moving their names" {
    const gpa = std.testing.allocator;
    const log = [_]LogEntry{
        .{ .item = .{ .tool = .{ .at = null, .name = "python", .subject = "", .took_us = null, .outcome = .Failed } }, .card = 0 },
        .{ .item = .{ .tool = .{ .at = null, .name = "python", .subject = "", .took_us = null, .outcome = .Answered } }, .card = 1 },
    };
    const card: Card = .{ .python = .{ .code = &.{}, .output = &.{} } };
    const cards = [_]Card{ card, card };
    var hits: std.ArrayList(Hit) = .empty;
    defer hits.deinit(gpa);
    var view: View = .{ .completion = .{ .first = 0, .last = 1, .tone = 142 } };
    defer view.deinit(gpa);
    const region: write.Live = .{ .composer = .{ .typed = "", .cursor = 0, .placeholder = "Mei", .room = "Mei", .offer = "" } };
    const arriving = try render(gpa, &log, &cards, 0, "", region, 80, 24, &view, &hits);
    defer gpa.free(arriving);
    try std.testing.expect(std.mem.indexOf(u8, arriving, "\x1b[38;2;142;142;142m×\x1b[39m python") != null);
    try std.testing.expect(std.mem.indexOf(u8, arriving, "\x1b[38;2;142;142;142m✓\x1b[39m python") != null);
    view.completion = null;
    const settled = try render(gpa, &log, &cards, 0, "", region, 80, 24, &view, &hits);
    defer gpa.free(settled);
    try std.testing.expect(std.mem.indexOf(u8, settled, "\x1b[38;2;172;172;172m×\x1b[39m python") != null);
    try std.testing.expect(std.mem.indexOf(u8, settled, "\x1b[38;2;172;172;172m✓\x1b[39m python") != null);
}

test "a windowed transcript agrees with full layout across scroll, resize and selection" {
    const gpa = std.testing.allocator;
    const region: write.Live = .{ .composer = .{ .typed = "", .cursor = 0, .placeholder = "Mei", .room = "Mei", .offer = "" } };
    var log: [128]LogEntry = undefined;
    var full: [log.len]write.Entry = undefined;
    var items: [log.len]viewport.Item = undefined;
    var labels: [log.len][48]u8 = undefined;
    for (&log, &full, &items, 0..) |*record, *entry, *item, i| {
        entry.* = if (i == 0) .{ .banner = .{ .city = "test", .address = "", .url = "" } } else .{ .reply = try std.fmt.bufPrint(&labels[i], "line {d} · 会话正文 text wraps", .{i}) };
        record.* = .{ .item = entry.* };
        item.* = .{ .origin = .{ .entry = i } };
    }
    const cards = [_]Card{.{ .python = .{ .code = &.{}, .output = &.{} } }};
    log[120] = .{ .item = .{ .tool = .{ .at = null, .name = "python", .subject = "", .took_us = null, .outcome = .Answered } }, .card = 0 };
    full[120] = log[120].item;
    full[120].tool.subject = foldWord(.time, true);
    items[120].card = 0;
    var view: View = .{};
    defer view.deinit(gpa);
    var reference: View = .{};
    defer reference.deinit(gpa);
    var hits: std.ArrayList(Hit) = .empty;
    defer hits.deinit(gpa);
    var all_hits: std.ArrayList(Hit) = .empty;
    defer all_hits.deinit(gpa);
    for ([_]usize{ 64, 128 }) |count| {
        const window = Window.around(&view, count, 24);
        try std.testing.expectEqual(@as(usize, 26), window.end - window.start);
    }
    for ([_]i32{ 0, -13, -42, 2, 60, -3 }, 0..) |movement, i| {
        view.scroll(movement);
        reference.scroll(movement);
        if (i == 3) {
            view.selection = .{ .start = .{ .origin = .{ .entry = 35 } }, .finish = .{ .origin = .{ .entry = 109 }, .offset = 8 } };
            reference.selection = view.selection;
        }
        const width: u16 = if (i % 2 == 0) 30 else 48;
        const expected = try viewport.draw(gpa, &full, &items, region, width, 24, false, &reference, &all_hits);
        defer gpa.free(expected);
        const actual = try render(gpa, &log, &cards, 0, "", region, width, 24, &view, &hits);
        defer gpa.free(actual);
        try std.testing.expectEqualSlices(u8, expected, actual);
        try std.testing.expectEqualSlices(Hit, all_hits.items, hits.items);
        try std.testing.expectEqualDeep(reference.anchor, view.anchor);
        try std.testing.expectEqualDeep(reference.visible.items, view.visible.items);
    }
}
