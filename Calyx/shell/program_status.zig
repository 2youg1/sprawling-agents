// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

const std = @import("std");

const switch_name = "SPRAWLING_PROGRAM_STATUS";
const never = "never";

pub const Application = enum { sprawling, mei };

pub const Reporting = enum {
    on,
    off,

    pub fn deciding(variable: ?[]const u8) Reporting {
        if (variable) |value| {
            if (std.mem.eql(u8, value, never)) return .off;
        }
        return .on;
    }
};

pub const Rest = enum {
    idle,
    done,
    failed,

    pub fn ofCompletion(word: []const u8) Rest {
        if (std.mem.eql(u8, word, "cancelled")) return .idle;
        if (std.mem.eql(u8, word, "limit")) return .failed;
        return .done;
    }
};

pub const Counts = struct {
    runs: u64,
    approvals: u64,
};

pub const State = union(enum) {
    resting: Rest,
    working: u64,
    blocked: u64,
};

pub fn judged(counts: Counts, rest: Rest) State {
    if (counts.approvals > 0) return .{ .blocked = counts.approvals };
    if (counts.runs > 0) return .{ .working = counts.runs };
    return .{ .resting = rest };
}

pub const Event = union(enum) {
    ended: Rest,
    counted: Counts,
    closed,
};

pub const EventKind = enum {
    run_started,
    run_frozen,
    approval_requested,
    approval_resolved,
    other,
};

pub fn moves(kind: EventKind) bool {
    return switch (kind) {
        .run_started, .run_frozen, .approval_requested, .approval_resolved => true,
        .other => false,
    };
}

pub const Shown = union(enum) {
    nothing,
    reported: State,
    cleared,
};

pub const Held = struct {
    rest: Rest,
    shown: Shown,

    pub const opening: Held = .{ .rest = .idle, .shown = .nothing };
};

pub const Report = union(enum) {
    of: State,
    cleared,

    pub fn pairs(self: Report) struct { []const u8, ?[]const u8 } {
        return switch (self) {
            .of => |state| switch (state) {
                .resting => |r| switch (r) {
                    .idle => .{ "idle", null },
                    .done => .{ "done", null },
                    .failed => .{ "error", null },
                },
                .working => .{ "working", null },
                .blocked => .{ "blocked", "permission" },
            },
            .cleared => .{ "clear", null },
        };
    }

    pub fn message(self: Report, allocator: std.mem.Allocator) !?[]u8 {
        return switch (self) {
            .of => |state| switch (state) {
                .working => |runs| if (runs == 1)
                    try allocator.dupe(u8, "1 run is going")
                else
                    try std.fmt.allocPrint(allocator, "{d} runs are going", .{runs}),
                .blocked => |approvals| if (approvals == 1)
                    try allocator.dupe(u8, "1 request waits for an answer")
                else
                    try std.fmt.allocPrint(allocator, "{d} requests wait for an answer", .{approvals}),
                .resting => |r| switch (r) {
                    .done => try allocator.dupe(u8, "every run has finished"),
                    .failed => try allocator.dupe(u8, "a run stopped at its limit"),
                    .idle => null,
                },
            },
            .cleared => null,
        };
    }

    pub fn writeAnsi(self: Report, allocator: std.mem.Allocator, application: Application) ![]u8 {
        const pair = self.pairs();
        var out: std.ArrayList(u8) = .empty;
        errdefer out.deinit(allocator);
        try out.appendSlice(allocator, "\x1b]7501;state=");
        try out.appendSlice(allocator, pair[0]);
        if (self == .of) {
            try out.appendSlice(allocator, ":app=");
            try out.appendSlice(allocator, @tagName(application));
        }
        if (pair[1]) |kind| {
            try out.appendSlice(allocator, ":kind=");
            try out.appendSlice(allocator, kind);
        }
        if (try self.message(allocator)) |msg| {
            defer allocator.free(msg);
            var encoded: [4096]u8 = undefined;
            const n = std.base64.standard.Encoder.calcSize(msg.len);
            if (n > encoded.len) return error.MessageTooLong;
            const wrote = std.base64.standard.Encoder.encode(encoded[0..n], msg);
            try out.appendSlice(allocator, ":msg=");
            try out.appendSlice(allocator, wrote);
        }
        try out.appendSlice(allocator, "\x1b\\");
        return try out.toOwnedSlice(allocator);
    }
};

pub const Stepped = struct {
    held: Held,
    report: ?Report,
};

pub fn step(held: Held, event: Event) Stepped {
    return switch (held.shown) {
        .cleared => .{ .held = held, .report = null },
        else => switch (event) {
            .ended => |rest| .{ .held = .{ .rest = rest, .shown = held.shown }, .report = null },
            .counted => |counts| blk: {
                const state = judged(counts, held.rest);
                if (held.shown == .reported and std.meta.eql(held.shown.reported, state)) {
                    break :blk Stepped{ .held = held, .report = null };
                }
                break :blk .{
                    .held = .{ .rest = held.rest, .shown = .{ .reported = state } },
                    .report = .{ .of = state },
                };
            },
            .closed => switch (held.shown) {
                .nothing => .{
                    .held = .{ .rest = held.rest, .shown = .cleared },
                    .report = null,
                },
                .reported => .{
                    .held = .{ .rest = held.rest, .shown = .cleared },
                    .report = .cleared,
                },
                .cleared => .{ .held = held, .report = null },
            },
        },
    };
}

const vectors =
    \\judged 0 0 Idle Resting/Idle
    \\judged 0 0 Done Resting/Done
    \\judged 0 0 Failed Resting/Failed
    \\judged 1 0 Idle Working/1
    \\judged 1 0 Done Working/1
    \\judged 1 0 Failed Working/1
    \\judged 2 0 Idle Working/2
    \\judged 2 0 Done Working/2
    \\judged 2 0 Failed Working/2
    \\judged 0 1 Idle Blocked/1
    \\judged 0 1 Done Blocked/1
    \\judged 0 1 Failed Blocked/1
    \\judged 1 1 Idle Blocked/1
    \\judged 1 1 Done Blocked/1
    \\judged 1 1 Failed Blocked/1
    \\judged 0 2 Idle Blocked/2
    \\judged 0 2 Done Blocked/2
    \\judged 0 2 Failed Blocked/2
    \\judged 3 2 Idle Blocked/2
    \\judged 3 2 Done Blocked/2
    \\judged 3 2 Failed Blocked/2
    \\step Nothing Idle Ended/done Nothing Done -
    \\step Nothing Idle Ended/limit Nothing Failed -
    \\step Nothing Idle Ended/cancelled Nothing Idle -
    \\step Nothing Idle Counted/0/0 Reported/Resting/Idle Idle Of/Resting/Idle
    \\step Nothing Idle Counted/1/0 Reported/Working/1 Idle Of/Working/1
    \\step Nothing Idle Counted/0/1 Reported/Blocked/1 Idle Of/Blocked/1
    \\step Nothing Idle Closed Cleared Idle -
    \\step Nothing Done Ended/done Nothing Done -
    \\step Nothing Done Ended/limit Nothing Failed -
    \\step Nothing Done Ended/cancelled Nothing Idle -
    \\step Nothing Done Counted/0/0 Reported/Resting/Done Done Of/Resting/Done
    \\step Nothing Done Counted/1/0 Reported/Working/1 Done Of/Working/1
    \\step Nothing Done Counted/0/1 Reported/Blocked/1 Done Of/Blocked/1
    \\step Nothing Done Closed Cleared Done -
    \\step Nothing Failed Ended/done Nothing Done -
    \\step Nothing Failed Ended/limit Nothing Failed -
    \\step Nothing Failed Ended/cancelled Nothing Idle -
    \\step Nothing Failed Counted/0/0 Reported/Resting/Failed Failed Of/Resting/Failed
    \\step Nothing Failed Counted/1/0 Reported/Working/1 Failed Of/Working/1
    \\step Nothing Failed Counted/0/1 Reported/Blocked/1 Failed Of/Blocked/1
    \\step Nothing Failed Closed Cleared Failed -
    \\step Reported/Working/1 Idle Ended/done Reported/Working/1 Done -
    \\step Reported/Working/1 Idle Ended/limit Reported/Working/1 Failed -
    \\step Reported/Working/1 Idle Ended/cancelled Reported/Working/1 Idle -
    \\step Reported/Working/1 Idle Counted/0/0 Reported/Resting/Idle Idle Of/Resting/Idle
    \\step Reported/Working/1 Idle Counted/1/0 Reported/Working/1 Idle -
    \\step Reported/Working/1 Idle Counted/0/1 Reported/Blocked/1 Idle Of/Blocked/1
    \\step Reported/Working/1 Idle Closed Cleared Idle Cleared
    \\step Reported/Working/1 Done Ended/done Reported/Working/1 Done -
    \\step Reported/Working/1 Done Ended/limit Reported/Working/1 Failed -
    \\step Reported/Working/1 Done Ended/cancelled Reported/Working/1 Idle -
    \\step Reported/Working/1 Done Counted/0/0 Reported/Resting/Done Done Of/Resting/Done
    \\step Reported/Working/1 Done Counted/1/0 Reported/Working/1 Done -
    \\step Reported/Working/1 Done Counted/0/1 Reported/Blocked/1 Done Of/Blocked/1
    \\step Reported/Working/1 Done Closed Cleared Done Cleared
    \\step Reported/Working/1 Failed Ended/done Reported/Working/1 Done -
    \\step Reported/Working/1 Failed Ended/limit Reported/Working/1 Failed -
    \\step Reported/Working/1 Failed Ended/cancelled Reported/Working/1 Idle -
    \\step Reported/Working/1 Failed Counted/0/0 Reported/Resting/Failed Failed Of/Resting/Failed
    \\step Reported/Working/1 Failed Counted/1/0 Reported/Working/1 Failed -
    \\step Reported/Working/1 Failed Counted/0/1 Reported/Blocked/1 Failed Of/Blocked/1
    \\step Reported/Working/1 Failed Closed Cleared Failed Cleared
    \\step Reported/Blocked/1 Idle Ended/done Reported/Blocked/1 Done -
    \\step Reported/Blocked/1 Idle Ended/limit Reported/Blocked/1 Failed -
    \\step Reported/Blocked/1 Idle Ended/cancelled Reported/Blocked/1 Idle -
    \\step Reported/Blocked/1 Idle Counted/0/0 Reported/Resting/Idle Idle Of/Resting/Idle
    \\step Reported/Blocked/1 Idle Counted/1/0 Reported/Working/1 Idle Of/Working/1
    \\step Reported/Blocked/1 Idle Counted/0/1 Reported/Blocked/1 Idle -
    \\step Reported/Blocked/1 Idle Closed Cleared Idle Cleared
    \\step Reported/Blocked/1 Done Ended/done Reported/Blocked/1 Done -
    \\step Reported/Blocked/1 Done Ended/limit Reported/Blocked/1 Failed -
    \\step Reported/Blocked/1 Done Ended/cancelled Reported/Blocked/1 Idle -
    \\step Reported/Blocked/1 Done Counted/0/0 Reported/Resting/Done Done Of/Resting/Done
    \\step Reported/Blocked/1 Done Counted/1/0 Reported/Working/1 Done Of/Working/1
    \\step Reported/Blocked/1 Done Counted/0/1 Reported/Blocked/1 Done -
    \\step Reported/Blocked/1 Done Closed Cleared Done Cleared
    \\step Reported/Blocked/1 Failed Ended/done Reported/Blocked/1 Done -
    \\step Reported/Blocked/1 Failed Ended/limit Reported/Blocked/1 Failed -
    \\step Reported/Blocked/1 Failed Ended/cancelled Reported/Blocked/1 Idle -
    \\step Reported/Blocked/1 Failed Counted/0/0 Reported/Resting/Failed Failed Of/Resting/Failed
    \\step Reported/Blocked/1 Failed Counted/1/0 Reported/Working/1 Failed Of/Working/1
    \\step Reported/Blocked/1 Failed Counted/0/1 Reported/Blocked/1 Failed -
    \\step Reported/Blocked/1 Failed Closed Cleared Failed Cleared
    \\step Reported/Resting/Done Idle Ended/done Reported/Resting/Done Done -
    \\step Reported/Resting/Done Idle Ended/limit Reported/Resting/Done Failed -
    \\step Reported/Resting/Done Idle Ended/cancelled Reported/Resting/Done Idle -
    \\step Reported/Resting/Done Idle Counted/0/0 Reported/Resting/Idle Idle Of/Resting/Idle
    \\step Reported/Resting/Done Idle Counted/1/0 Reported/Working/1 Idle Of/Working/1
    \\step Reported/Resting/Done Idle Counted/0/1 Reported/Blocked/1 Idle Of/Blocked/1
    \\step Reported/Resting/Done Idle Closed Cleared Idle Cleared
    \\step Reported/Resting/Done Done Ended/done Reported/Resting/Done Done -
    \\step Reported/Resting/Done Done Ended/limit Reported/Resting/Done Failed -
    \\step Reported/Resting/Done Done Ended/cancelled Reported/Resting/Done Idle -
    \\step Reported/Resting/Done Done Counted/0/0 Reported/Resting/Done Done -
    \\step Reported/Resting/Done Done Counted/1/0 Reported/Working/1 Done Of/Working/1
    \\step Reported/Resting/Done Done Counted/0/1 Reported/Blocked/1 Done Of/Blocked/1
    \\step Reported/Resting/Done Done Closed Cleared Done Cleared
    \\step Reported/Resting/Done Failed Ended/done Reported/Resting/Done Done -
    \\step Reported/Resting/Done Failed Ended/limit Reported/Resting/Done Failed -
    \\step Reported/Resting/Done Failed Ended/cancelled Reported/Resting/Done Idle -
    \\step Reported/Resting/Done Failed Counted/0/0 Reported/Resting/Failed Failed Of/Resting/Failed
    \\step Reported/Resting/Done Failed Counted/1/0 Reported/Working/1 Failed Of/Working/1
    \\step Reported/Resting/Done Failed Counted/0/1 Reported/Blocked/1 Failed Of/Blocked/1
    \\step Reported/Resting/Done Failed Closed Cleared Failed Cleared
    \\step Reported/Resting/Idle Idle Ended/done Reported/Resting/Idle Done -
    \\step Reported/Resting/Idle Idle Ended/limit Reported/Resting/Idle Failed -
    \\step Reported/Resting/Idle Idle Ended/cancelled Reported/Resting/Idle Idle -
    \\step Reported/Resting/Idle Idle Counted/0/0 Reported/Resting/Idle Idle -
    \\step Reported/Resting/Idle Idle Counted/1/0 Reported/Working/1 Idle Of/Working/1
    \\step Reported/Resting/Idle Idle Counted/0/1 Reported/Blocked/1 Idle Of/Blocked/1
    \\step Reported/Resting/Idle Idle Closed Cleared Idle Cleared
    \\step Reported/Resting/Idle Done Ended/done Reported/Resting/Idle Done -
    \\step Reported/Resting/Idle Done Ended/limit Reported/Resting/Idle Failed -
    \\step Reported/Resting/Idle Done Ended/cancelled Reported/Resting/Idle Idle -
    \\step Reported/Resting/Idle Done Counted/0/0 Reported/Resting/Done Done Of/Resting/Done
    \\step Reported/Resting/Idle Done Counted/1/0 Reported/Working/1 Done Of/Working/1
    \\step Reported/Resting/Idle Done Counted/0/1 Reported/Blocked/1 Done Of/Blocked/1
    \\step Reported/Resting/Idle Done Closed Cleared Done Cleared
    \\step Reported/Resting/Idle Failed Ended/done Reported/Resting/Idle Done -
    \\step Reported/Resting/Idle Failed Ended/limit Reported/Resting/Idle Failed -
    \\step Reported/Resting/Idle Failed Ended/cancelled Reported/Resting/Idle Idle -
    \\step Reported/Resting/Idle Failed Counted/0/0 Reported/Resting/Failed Failed Of/Resting/Failed
    \\step Reported/Resting/Idle Failed Counted/1/0 Reported/Working/1 Failed Of/Working/1
    \\step Reported/Resting/Idle Failed Counted/0/1 Reported/Blocked/1 Failed Of/Blocked/1
    \\step Reported/Resting/Idle Failed Closed Cleared Failed Cleared
    \\step Cleared Idle Ended/done Cleared Idle -
    \\step Cleared Idle Ended/limit Cleared Idle -
    \\step Cleared Idle Ended/cancelled Cleared Idle -
    \\step Cleared Idle Counted/0/0 Cleared Idle -
    \\step Cleared Idle Counted/1/0 Cleared Idle -
    \\step Cleared Idle Counted/0/1 Cleared Idle -
    \\step Cleared Idle Closed Cleared Idle -
    \\step Cleared Done Ended/done Cleared Done -
    \\step Cleared Done Ended/limit Cleared Done -
    \\step Cleared Done Ended/cancelled Cleared Done -
    \\step Cleared Done Counted/0/0 Cleared Done -
    \\step Cleared Done Counted/1/0 Cleared Done -
    \\step Cleared Done Counted/0/1 Cleared Done -
    \\step Cleared Done Closed Cleared Done -
    \\step Cleared Failed Ended/done Cleared Failed -
    \\step Cleared Failed Ended/limit Cleared Failed -
    \\step Cleared Failed Ended/cancelled Cleared Failed -
    \\step Cleared Failed Counted/0/0 Cleared Failed -
    \\step Cleared Failed Counted/1/0 Cleared Failed -
    \\step Cleared Failed Counted/0/1 Cleared Failed -
    \\step Cleared Failed Closed Cleared Failed -
;

fn parseRest(word: []const u8) Rest {
    if (std.mem.eql(u8, word, "Idle")) return .idle;
    if (std.mem.eql(u8, word, "Done")) return .done;
    if (std.mem.eql(u8, word, "Failed")) return .failed;
    @panic("no rest");
}

fn parseNumber(word: []const u8) u64 {
    return std.fmt.parseInt(u64, word, 10) catch @panic("number");
}

fn parseState(word: []const u8) State {
    var it = std.mem.splitScalar(u8, word, '/');
    const a = it.next() orelse @panic("no state");
    const b = it.next() orelse @panic("no state");
    if (std.mem.eql(u8, a, "Resting")) return .{ .resting = parseRest(b) };
    if (std.mem.eql(u8, a, "Working")) return .{ .working = parseNumber(b) };
    if (std.mem.eql(u8, a, "Blocked")) return .{ .blocked = parseNumber(b) };
    @panic("no state");
}

fn parseShown(word: []const u8) Shown {
    if (std.mem.eql(u8, word, "Nothing")) return .nothing;
    if (std.mem.eql(u8, word, "Cleared")) return .cleared;
    if (std.mem.startsWith(u8, word, "Reported/")) return .{ .reported = parseState(word["Reported/".len..]) };
    @panic("no shown");
}

fn parseEvent(word: []const u8) Event {
    var it = std.mem.splitScalar(u8, word, '/');
    const a = it.next() orelse @panic("no event");
    if (std.mem.eql(u8, a, "Ended")) {
        return .{ .ended = Rest.ofCompletion(it.next() orelse @panic("completion")) };
    }
    if (std.mem.eql(u8, a, "Counted")) {
        return .{ .counted = .{
            .runs = parseNumber(it.next() orelse @panic("runs")),
            .approvals = parseNumber(it.next() orelse @panic("approvals")),
        } };
    }
    if (std.mem.eql(u8, a, "Closed")) return .closed;
    @panic("no event");
}

fn parseReport(word: []const u8) ?Report {
    if (std.mem.eql(u8, word, "-")) return null;
    if (std.mem.eql(u8, word, "Cleared")) return .cleared;
    if (std.mem.startsWith(u8, word, "Of/")) return .{ .of = parseState(word["Of/".len..]) };
    @panic("no report");
}

fn steppedEql(a: Stepped, b: Stepped) bool {
    return std.meta.eql(a.held, b.held) and std.meta.eql(a.report, b.report);
}

test "every row is the models" {
    var rows: usize = 0;
    var differing: usize = 0;
    var lines = std.mem.splitScalar(u8, vectors, '\n');
    while (lines.next()) |line| {
        if (line.len == 0) continue;
        var words = std.mem.tokenizeScalar(u8, line, ' ');
        const kind = words.next() orelse @panic("row");
        if (std.mem.eql(u8, kind, "judged")) {
            const runs = parseNumber(words.next() orelse @panic("runs"));
            const approvals = parseNumber(words.next() orelse @panic("approvals"));
            const rest = parseRest(words.next() orelse @panic("rest"));
            const state = parseState(words.next() orelse @panic("state"));
            rows += 1;
            if (!std.meta.eql(judged(.{ .runs = runs, .approvals = approvals }, rest), state)) differing += 1;
        } else if (std.mem.eql(u8, kind, "step")) {
            const from = words.next() orelse @panic("from");
            const r = words.next() orelse @panic("rest");
            const on = words.next() orelse @panic("on");
            const to = words.next() orelse @panic("to");
            const r_to = words.next() orelse @panic("rest_to");
            const wrote = words.next() orelse @panic("wrote");
            const held = Held{ .rest = parseRest(r), .shown = parseShown(from) };
            const wanted = Stepped{
                .held = .{ .rest = parseRest(r_to), .shown = parseShown(to) },
                .report = parseReport(wrote),
            };
            rows += 1;
            if (!steppedEql(step(held, parseEvent(on)), wanted)) differing += 1;
        } else @panic("a vector row has a known shape");
    }
    try std.testing.expectEqual(@as(usize, 147), rows);
    try std.testing.expectEqual(@as(usize, 0), differing);
}

test "status identifies the calling application to the terminal" {
    const report: Report = .{ .of = .{ .resting = .idle } };
    const allocator = std.testing.allocator;
    const mei = try report.writeAnsi(allocator, .mei);
    defer allocator.free(mei);
    try std.testing.expectEqualStrings("\x1b]7501;state=idle:app=mei\x1b\\", mei);
    const sprawling = try report.writeAnsi(allocator, .sprawling);
    defer allocator.free(sprawling);
    try std.testing.expectEqualStrings("\x1b]7501;state=idle:app=sprawling\x1b\\", sprawling);
}

test "cleared report is the OSC clear sequence" {
    const report: Report = .cleared;
    const rendered = try report.writeAnsi(std.testing.allocator, .mei);
    defer std.testing.allocator.free(rendered);
    try std.testing.expectEqualStrings("\x1b]7501;state=clear\x1b\\", rendered);
}

test "deciding treats never as off" {
    try std.testing.expect(Reporting.deciding("never") == .off);
    try std.testing.expect(Reporting.deciding(null) == .on);
    try std.testing.expect(Reporting.deciding("1") == .on);
    _ = switch_name;
}
