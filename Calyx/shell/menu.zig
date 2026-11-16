// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

const std = @import("std");
const language = @import("language.zig");

const shown_count: usize = 5;

pub const Then = enum { carry, wait };

pub const Fit = struct {
    line: []const u8,
    spelling: []const u8,
    takes: []const u8,
    summary: []const u8,
    then: Then,
};

pub const Choice = struct {
    spelling: []const u8,
    takes: []const u8,
    summary: []const u8,
};

pub fn verbs(allocator: std.mem.Allocator, typed: []const u8) ![]Fit {
    var buf: [language.Slash.all.len]language.Slash = undefined;
    const offered = language.slashVerbs(&buf);
    var out: std.ArrayList(Fit) = .empty;
    errdefer {
        for (out.items) |fit| allocator.free(fit.line);
        out.deinit(allocator);
    }
    for (offered) |verb| {
        if (!std.mem.startsWith(u8, verb.spelling(), typed)) continue;
        const then: Then = if (verb.takes().len == 0) .carry else .wait;
        const line = if (verb.takes().len == 0)
            try allocator.dupe(u8, verb.spelling())
        else
            try std.fmt.allocPrint(allocator, "{s} ", .{verb.spelling()});
        try out.append(allocator, .{
            .line = line,
            .spelling = verb.spelling(),
            .takes = verb.takes(),
            .summary = verb.says(),
            .then = then,
        });
    }
    return try out.toOwnedSlice(allocator);
}

pub fn freeVerbs(allocator: std.mem.Allocator, fits: []Fit) void {
    for (fits) |fit| allocator.free(fit.line);
    allocator.free(fits);
}

pub fn arguments(allocator: std.mem.Allocator, verb: []const u8, begun: []const u8, offered: []const []const u8) ![]Fit {
    var out: std.ArrayList(Fit) = .empty;
    errdefer {
        for (out.items) |fit| {
            allocator.free(fit.line);
            allocator.free(fit.spelling);
        }
        out.deinit(allocator);
    }
    for (offered) |argument| {
        if (!std.mem.startsWith(u8, argument, begun)) continue;
        try out.append(allocator, .{
            .line = try std.fmt.allocPrint(allocator, "{s} {s}", .{ verb, argument }),
            .spelling = try allocator.dupe(u8, argument),
            .takes = "",
            .summary = "",
            .then = .carry,
        });
    }
    return try out.toOwnedSlice(allocator);
}

pub fn freeArguments(allocator: std.mem.Allocator, fits: []Fit) void {
    for (fits) |fit| {
        allocator.free(fit.line);
        allocator.free(fit.spelling);
    }
    allocator.free(fits);
}

pub const Menu = struct {
    fits: []Fit,
    chosen: usize,

    pub fn of(fits: []Fit) ?Menu {
        if (fits.len == 0) return null;
        return .{ .fits = fits, .chosen = 0 };
    }

    pub fn next(self: *Menu) void {
        const n = self.chosen +| 1;
        self.chosen = if (n < self.fits.len) n else 0;
    }

    pub fn previous(self: *Menu) void {
        self.chosen = if (self.chosen == 0) self.fits.len -| 1 else self.chosen - 1;
    }

    pub fn chosenFit(self: *const Menu) ?Fit {
        if (self.chosen >= self.fits.len) return null;
        return self.fits[self.chosen];
    }

    pub fn window(self: *const Menu, out: []Choice) struct { []Choice, usize, usize } {
        const last_start = self.fits.len -| shown_count;
        const start = @min(self.chosen -| (shown_count -| 1), last_start);
        var n: usize = 0;
        var i = start;
        while (i < self.fits.len and n < shown_count and n < out.len) : (i += 1) {
            out[n] = .{
                .spelling = self.fits[i].spelling,
                .takes = self.fits[i].takes,
                .summary = self.fits[i].summary,
            };
            n += 1;
        }
        const more = self.fits.len -| n;
        return .{ out[0..n], self.chosen -| start, more };
    }
};

test "w then tab then enter is web" {
    const fits = try verbs(std.testing.allocator, "/w");
    defer freeVerbs(std.testing.allocator, fits);
    var menu = Menu.of(fits).?;
    const chosen = menu.chosenFit().?;
    try std.testing.expectEqualStrings(language.Slash.web.spelling(), chosen.line);
    try std.testing.expect(chosen.then == .carry);
    var buf: [shown_count]Choice = undefined;
    const win = menu.window(&buf);
    try std.testing.expectEqual(@as(usize, 2), win[0].len);
}

test "the window follows the cursor and counts the rest" {
    const fits = try verbs(std.testing.allocator, "/");
    defer freeVerbs(std.testing.allocator, fits);
    const total = fits.len;
    var menu = Menu.of(fits).?;
    var i: usize = 0;
    while (i < 7) : (i += 1) menu.next();
    var buf: [shown_count]Choice = undefined;
    const win = menu.window(&buf);
    try std.testing.expectEqualStrings(fits[7].spelling, win[0][win[1]].spelling);
    try std.testing.expectEqual(shown_count, win[0].len);
    try std.testing.expectEqual(total - shown_count, win[2]);
    menu.previous();
    var j: usize = 0;
    while (j < total) : (j += 1) menu.next();
    try std.testing.expectEqualStrings(fits[6].spelling, menu.chosenFit().?.spelling);
}
