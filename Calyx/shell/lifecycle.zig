// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

const std = @import("std");

pub const interrupt_grace_ns: u64 = 4 * std.time.ns_per_s;

pub const CloseMode = enum { drain, interrupt };

pub const ClosedBy = enum {
    console,
    page,
    interrupt_signal,
    break_signal,
    terminate,
    terminal_lost,
};

pub const Face = union(enum) {
    opening,
    cli,
    quiet_host,
    headless,
    stopping: struct { mode: CloseMode, sinks: Sinks },
    gone: Handoff,

    pub fn armed(self: Face) bool {
        return switch (self) {
            .stopping => |s| s.mode == .interrupt,
            else => false,
        };
    }

    fn interactive(self: Face) bool {
        return switch (self) {
            .cli, .quiet_host => true,
            else => false,
        };
    }
};

pub const Sinks = enum { held, cut };

pub const Handoff = enum { written, skipped };

pub const Surface = enum { cli, quiet_host, headless };

pub const Asker = enum { console, page };

pub const Event = union(enum) {
    ready: Surface,
    web,
    back,
    quit: struct { asker: Asker, mode: CloseMode },
    interrupt_signal,
    break_signal,
    terminate,
    terminal_lost,
    failed,
    landed,
    deadline,
};

pub const Cause = union(enum) {
    closed: ClosedBy,
    failed,
};

pub const Stepped = struct {
    face: Face,
    cause: ?Cause,
};

pub fn step(face: Face, event: Event) Stepped {
    return switch (face) {
        .stopping => |s| .{ .face = stopping(s.mode, s.sinks, event), .cause = null },
        .gone => |handoff| .{ .face = .{ .gone = handoff }, .cause = null },
        .opening, .cli, .quiet_host, .headless => serving(face, event),
    };
}

const Asked = struct { mode: CloseMode, sinks: Sinks, cause: Cause };

fn asked(face: Face, event: Event) ?Asked {
    const held = struct {
        fn call(mode: CloseMode, by: ClosedBy) Asked {
            return .{ .mode = mode, .sinks = .held, .cause = .{ .closed = by } };
        }
    }.call;
    return switch (event) {
        .quit => |q| switch (q.asker) {
            .console => held(q.mode, .console),
            .page => held(q.mode, .page),
        },
        .interrupt_signal => if (face.interactive()) null else held(.drain, .interrupt_signal),
        .break_signal => held(.drain, .break_signal),
        .terminate => held(.interrupt, .terminate),
        .terminal_lost => if (face.interactive())
            Asked{ .mode = .interrupt, .sinks = .cut, .cause = .{ .closed = .terminal_lost } }
        else
            null,
        .failed => Asked{ .mode = .drain, .sinks = .held, .cause = .failed },
        .ready, .web, .back, .landed, .deadline => null,
    };
}

fn serving(face: Face, event: Event) Stepped {
    if (asked(face, event)) |a| {
        return .{
            .face = .{ .stopping = .{ .mode = a.mode, .sinks = a.sinks } },
            .cause = a.cause,
        };
    }
    const next: Face = switch (face) {
        .opening => switch (event) {
            .ready => |surface| switch (surface) {
                .cli => .cli,
                .quiet_host => .quiet_host,
                .headless => .headless,
            },
            else => face,
        },
        .quiet_host => if (event == .back) .cli else face,
        .cli => if (event == .web) .quiet_host else face,
        else => face,
    };
    return .{ .face = next, .cause = null };
}

fn again(mode: CloseMode, sinks: Sinks) Face {
    return switch (mode) {
        .drain => .{ .stopping = .{ .mode = .interrupt, .sinks = sinks } },
        .interrupt => .{ .gone = .skipped },
    };
}

fn stopping(mode: CloseMode, sinks: Sinks, event: Event) Face {
    return switch (event) {
        .quit, .interrupt_signal, .break_signal => again(mode, sinks),
        .terminate => .{ .gone = .skipped },
        .terminal_lost => .{ .stopping = .{ .mode = .interrupt, .sinks = .cut } },
        .landed => .{ .gone = .written },
        .deadline => switch (mode) {
            .interrupt => .{ .gone = .skipped },
            .drain => .{ .stopping = .{ .mode = mode, .sinks = sinks } },
        },
        .ready, .web, .back, .failed => .{ .stopping = .{ .mode = mode, .sinks = sinks } },
    };
}

const vectors =
    \\Opening Ready/Cli Cli -
    \\Opening Ready/QuietHost QuietHost -
    \\Opening Ready/Headless Headless -
    \\Opening Web Opening -
    \\Opening Back Opening -
    \\Opening Quit/Console/Drain Stopping/Drain/Held Console
    \\Opening Quit/Console/Interrupt Stopping/Interrupt/Held Console
    \\Opening Quit/Page/Drain Stopping/Drain/Held Page
    \\Opening Quit/Page/Interrupt Stopping/Interrupt/Held Page
    \\Opening InterruptSignal Stopping/Drain/Held InterruptSignal
    \\Opening BreakSignal Stopping/Drain/Held BreakSignal
    \\Opening Terminate Stopping/Interrupt/Held Terminate
    \\Opening TerminalLost Opening -
    \\Opening Failed Stopping/Drain/Held Failed
    \\Opening Landed Opening -
    \\Opening Deadline Opening -
    \\Cli Ready/Cli Cli -
    \\Cli Ready/QuietHost Cli -
    \\Cli Ready/Headless Cli -
    \\Cli Web QuietHost -
    \\Cli Back Cli -
    \\Cli Quit/Console/Drain Stopping/Drain/Held Console
    \\Cli Quit/Console/Interrupt Stopping/Interrupt/Held Console
    \\Cli Quit/Page/Drain Stopping/Drain/Held Page
    \\Cli Quit/Page/Interrupt Stopping/Interrupt/Held Page
    \\Cli InterruptSignal Cli -
    \\Cli BreakSignal Stopping/Drain/Held BreakSignal
    \\Cli Terminate Stopping/Interrupt/Held Terminate
    \\Cli TerminalLost Stopping/Interrupt/Cut TerminalLost
    \\Cli Failed Stopping/Drain/Held Failed
    \\Cli Landed Cli -
    \\Cli Deadline Cli -
    \\QuietHost Ready/Cli QuietHost -
    \\QuietHost Ready/QuietHost QuietHost -
    \\QuietHost Ready/Headless QuietHost -
    \\QuietHost Web QuietHost -
    \\QuietHost Back Cli -
    \\QuietHost Quit/Console/Drain Stopping/Drain/Held Console
    \\QuietHost Quit/Console/Interrupt Stopping/Interrupt/Held Console
    \\QuietHost Quit/Page/Drain Stopping/Drain/Held Page
    \\QuietHost Quit/Page/Interrupt Stopping/Interrupt/Held Page
    \\QuietHost InterruptSignal QuietHost -
    \\QuietHost BreakSignal Stopping/Drain/Held BreakSignal
    \\QuietHost Terminate Stopping/Interrupt/Held Terminate
    \\QuietHost TerminalLost Stopping/Interrupt/Cut TerminalLost
    \\QuietHost Failed Stopping/Drain/Held Failed
    \\QuietHost Landed QuietHost -
    \\QuietHost Deadline QuietHost -
    \\Headless Ready/Cli Headless -
    \\Headless Ready/QuietHost Headless -
    \\Headless Ready/Headless Headless -
    \\Headless Web Headless -
    \\Headless Back Headless -
    \\Headless Quit/Console/Drain Stopping/Drain/Held Console
    \\Headless Quit/Console/Interrupt Stopping/Interrupt/Held Console
    \\Headless Quit/Page/Drain Stopping/Drain/Held Page
    \\Headless Quit/Page/Interrupt Stopping/Interrupt/Held Page
    \\Headless InterruptSignal Stopping/Drain/Held InterruptSignal
    \\Headless BreakSignal Stopping/Drain/Held BreakSignal
    \\Headless Terminate Stopping/Interrupt/Held Terminate
    \\Headless TerminalLost Headless -
    \\Headless Failed Stopping/Drain/Held Failed
    \\Headless Landed Headless -
    \\Headless Deadline Headless -
    \\Stopping/Drain/Held Ready/Cli Stopping/Drain/Held -
    \\Stopping/Drain/Held Ready/QuietHost Stopping/Drain/Held -
    \\Stopping/Drain/Held Ready/Headless Stopping/Drain/Held -
    \\Stopping/Drain/Held Web Stopping/Drain/Held -
    \\Stopping/Drain/Held Back Stopping/Drain/Held -
    \\Stopping/Drain/Held Quit/Console/Drain Stopping/Interrupt/Held -
    \\Stopping/Drain/Held Quit/Console/Interrupt Stopping/Interrupt/Held -
    \\Stopping/Drain/Held Quit/Page/Drain Stopping/Interrupt/Held -
    \\Stopping/Drain/Held Quit/Page/Interrupt Stopping/Interrupt/Held -
    \\Stopping/Drain/Held InterruptSignal Stopping/Interrupt/Held -
    \\Stopping/Drain/Held BreakSignal Stopping/Interrupt/Held -
    \\Stopping/Drain/Held Terminate Gone/Skipped -
    \\Stopping/Drain/Held TerminalLost Stopping/Interrupt/Cut -
    \\Stopping/Drain/Held Failed Stopping/Drain/Held -
    \\Stopping/Drain/Held Landed Gone/Written -
    \\Stopping/Drain/Held Deadline Stopping/Drain/Held -
    \\Stopping/Drain/Cut Ready/Cli Stopping/Drain/Cut -
    \\Stopping/Drain/Cut Ready/QuietHost Stopping/Drain/Cut -
    \\Stopping/Drain/Cut Ready/Headless Stopping/Drain/Cut -
    \\Stopping/Drain/Cut Web Stopping/Drain/Cut -
    \\Stopping/Drain/Cut Back Stopping/Drain/Cut -
    \\Stopping/Drain/Cut Quit/Console/Drain Stopping/Interrupt/Cut -
    \\Stopping/Drain/Cut Quit/Console/Interrupt Stopping/Interrupt/Cut -
    \\Stopping/Drain/Cut Quit/Page/Drain Stopping/Interrupt/Cut -
    \\Stopping/Drain/Cut Quit/Page/Interrupt Stopping/Interrupt/Cut -
    \\Stopping/Drain/Cut InterruptSignal Stopping/Interrupt/Cut -
    \\Stopping/Drain/Cut BreakSignal Stopping/Interrupt/Cut -
    \\Stopping/Drain/Cut Terminate Gone/Skipped -
    \\Stopping/Drain/Cut TerminalLost Stopping/Interrupt/Cut -
    \\Stopping/Drain/Cut Failed Stopping/Drain/Cut -
    \\Stopping/Drain/Cut Landed Gone/Written -
    \\Stopping/Drain/Cut Deadline Stopping/Drain/Cut -
    \\Stopping/Interrupt/Held Ready/Cli Stopping/Interrupt/Held -
    \\Stopping/Interrupt/Held Ready/QuietHost Stopping/Interrupt/Held -
    \\Stopping/Interrupt/Held Ready/Headless Stopping/Interrupt/Held -
    \\Stopping/Interrupt/Held Web Stopping/Interrupt/Held -
    \\Stopping/Interrupt/Held Back Stopping/Interrupt/Held -
    \\Stopping/Interrupt/Held Quit/Console/Drain Gone/Skipped -
    \\Stopping/Interrupt/Held Quit/Console/Interrupt Gone/Skipped -
    \\Stopping/Interrupt/Held Quit/Page/Drain Gone/Skipped -
    \\Stopping/Interrupt/Held Quit/Page/Interrupt Gone/Skipped -
    \\Stopping/Interrupt/Held InterruptSignal Gone/Skipped -
    \\Stopping/Interrupt/Held BreakSignal Gone/Skipped -
    \\Stopping/Interrupt/Held Terminate Gone/Skipped -
    \\Stopping/Interrupt/Held TerminalLost Stopping/Interrupt/Cut -
    \\Stopping/Interrupt/Held Failed Stopping/Interrupt/Held -
    \\Stopping/Interrupt/Held Landed Gone/Written -
    \\Stopping/Interrupt/Held Deadline Gone/Skipped -
    \\Stopping/Interrupt/Cut Ready/Cli Stopping/Interrupt/Cut -
    \\Stopping/Interrupt/Cut Ready/QuietHost Stopping/Interrupt/Cut -
    \\Stopping/Interrupt/Cut Ready/Headless Stopping/Interrupt/Cut -
    \\Stopping/Interrupt/Cut Web Stopping/Interrupt/Cut -
    \\Stopping/Interrupt/Cut Back Stopping/Interrupt/Cut -
    \\Stopping/Interrupt/Cut Quit/Console/Drain Gone/Skipped -
    \\Stopping/Interrupt/Cut Quit/Console/Interrupt Gone/Skipped -
    \\Stopping/Interrupt/Cut Quit/Page/Drain Gone/Skipped -
    \\Stopping/Interrupt/Cut Quit/Page/Interrupt Gone/Skipped -
    \\Stopping/Interrupt/Cut InterruptSignal Gone/Skipped -
    \\Stopping/Interrupt/Cut BreakSignal Gone/Skipped -
    \\Stopping/Interrupt/Cut Terminate Gone/Skipped -
    \\Stopping/Interrupt/Cut TerminalLost Stopping/Interrupt/Cut -
    \\Stopping/Interrupt/Cut Failed Stopping/Interrupt/Cut -
    \\Stopping/Interrupt/Cut Landed Gone/Written -
    \\Stopping/Interrupt/Cut Deadline Gone/Skipped -
    \\Gone/Written Ready/Cli Gone/Written -
    \\Gone/Written Ready/QuietHost Gone/Written -
    \\Gone/Written Ready/Headless Gone/Written -
    \\Gone/Written Web Gone/Written -
    \\Gone/Written Back Gone/Written -
    \\Gone/Written Quit/Console/Drain Gone/Written -
    \\Gone/Written Quit/Console/Interrupt Gone/Written -
    \\Gone/Written Quit/Page/Drain Gone/Written -
    \\Gone/Written Quit/Page/Interrupt Gone/Written -
    \\Gone/Written InterruptSignal Gone/Written -
    \\Gone/Written BreakSignal Gone/Written -
    \\Gone/Written Terminate Gone/Written -
    \\Gone/Written TerminalLost Gone/Written -
    \\Gone/Written Failed Gone/Written -
    \\Gone/Written Landed Gone/Written -
    \\Gone/Written Deadline Gone/Written -
    \\Gone/Skipped Ready/Cli Gone/Skipped -
    \\Gone/Skipped Ready/QuietHost Gone/Skipped -
    \\Gone/Skipped Ready/Headless Gone/Skipped -
    \\Gone/Skipped Web Gone/Skipped -
    \\Gone/Skipped Back Gone/Skipped -
    \\Gone/Skipped Quit/Console/Drain Gone/Skipped -
    \\Gone/Skipped Quit/Console/Interrupt Gone/Skipped -
    \\Gone/Skipped Quit/Page/Drain Gone/Skipped -
    \\Gone/Skipped Quit/Page/Interrupt Gone/Skipped -
    \\Gone/Skipped InterruptSignal Gone/Skipped -
    \\Gone/Skipped BreakSignal Gone/Skipped -
    \\Gone/Skipped Terminate Gone/Skipped -
    \\Gone/Skipped TerminalLost Gone/Skipped -
    \\Gone/Skipped Failed Gone/Skipped -
    \\Gone/Skipped Landed Gone/Skipped -
    \\Gone/Skipped Deadline Gone/Skipped -
;

fn parseMode(word: []const u8) CloseMode {
    if (std.mem.eql(u8, word, "Drain")) return .drain;
    if (std.mem.eql(u8, word, "Interrupt")) return .interrupt;
    @panic("no mode");
}

fn parseFace(word: []const u8) Face {
    var it = std.mem.splitScalar(u8, word, '/');
    const a = it.next() orelse @panic("no face");
    const b = it.next();
    const c = it.next();
    if (std.mem.eql(u8, a, "Opening") and b == null) return .opening;
    if (std.mem.eql(u8, a, "Cli") and b == null) return .cli;
    if (std.mem.eql(u8, a, "QuietHost") and b == null) return .quiet_host;
    if (std.mem.eql(u8, a, "Headless") and b == null) return .headless;
    if (std.mem.eql(u8, a, "Stopping")) {
        return .{ .stopping = .{
            .mode = parseMode(b orelse @panic("no mode")),
            .sinks = blk: {
                const k = c orelse @panic("no sinks");
                if (std.mem.eql(u8, k, "Held")) break :blk .held;
                if (std.mem.eql(u8, k, "Cut")) break :blk .cut;
                @panic("no sinks");
            },
        } };
    }
    if (std.mem.eql(u8, a, "Gone")) {
        const h = b orelse @panic("no handoff");
        if (std.mem.eql(u8, h, "Written")) return .{ .gone = .written };
        if (std.mem.eql(u8, h, "Skipped")) return .{ .gone = .skipped };
    }
    @panic("no face");
}

fn parseEvent(word: []const u8) Event {
    var it = std.mem.splitScalar(u8, word, '/');
    const a = it.next() orelse @panic("no event");
    const b = it.next();
    const c = it.next();
    if (std.mem.eql(u8, a, "Ready")) {
        const s = b orelse @panic("no surface");
        if (std.mem.eql(u8, s, "Cli")) return .{ .ready = .cli };
        if (std.mem.eql(u8, s, "QuietHost")) return .{ .ready = .quiet_host };
        if (std.mem.eql(u8, s, "Headless")) return .{ .ready = .headless };
    }
    if (std.mem.eql(u8, a, "Web") and b == null) return .web;
    if (std.mem.eql(u8, a, "Back") and b == null) return .back;
    if (std.mem.eql(u8, a, "Quit")) {
        const asker: Asker = blk: {
            const who = b orelse @panic("no asker");
            if (std.mem.eql(u8, who, "Console")) break :blk .console;
            if (std.mem.eql(u8, who, "Page")) break :blk .page;
            @panic("no asker");
        };
        return .{ .quit = .{ .asker = asker, .mode = parseMode(c orelse @panic("no mode")) } };
    }
    if (std.mem.eql(u8, a, "InterruptSignal") and b == null) return .interrupt_signal;
    if (std.mem.eql(u8, a, "BreakSignal") and b == null) return .break_signal;
    if (std.mem.eql(u8, a, "Terminate") and b == null) return .terminate;
    if (std.mem.eql(u8, a, "TerminalLost") and b == null) return .terminal_lost;
    if (std.mem.eql(u8, a, "Failed") and b == null) return .failed;
    if (std.mem.eql(u8, a, "Landed") and b == null) return .landed;
    if (std.mem.eql(u8, a, "Deadline") and b == null) return .deadline;
    @panic("no event");
}

fn parseCause(word: []const u8) ?Cause {
    if (std.mem.eql(u8, word, "-")) return null;
    if (std.mem.eql(u8, word, "Console")) return .{ .closed = .console };
    if (std.mem.eql(u8, word, "Page")) return .{ .closed = .page };
    if (std.mem.eql(u8, word, "InterruptSignal")) return .{ .closed = .interrupt_signal };
    if (std.mem.eql(u8, word, "BreakSignal")) return .{ .closed = .break_signal };
    if (std.mem.eql(u8, word, "Terminate")) return .{ .closed = .terminate };
    if (std.mem.eql(u8, word, "TerminalLost")) return .{ .closed = .terminal_lost };
    if (std.mem.eql(u8, word, "Failed")) return .failed;
    @panic("no cause");
}

fn steppedEql(a: Stepped, b: Stepped) bool {
    return std.meta.eql(a.face, b.face) and std.meta.eql(a.cause, b.cause);
}

test "every transition is the models" {
    var rows: usize = 0;
    var differing: usize = 0;
    var lines = std.mem.splitScalar(u8, vectors, '\n');
    while (lines.next()) |line| {
        if (line.len == 0) continue;
        var words = std.mem.tokenizeScalar(u8, line, ' ');
        const from = words.next() orelse @panic("vector");
        const on = words.next() orelse @panic("vector");
        const to = words.next() orelse @panic("vector");
        const why = words.next() orelse @panic("vector");
        if (words.next() != null) @panic("a vector has four words");
        const got = step(parseFace(from), parseEvent(on));
        const wanted = Stepped{ .face = parseFace(to), .cause = parseCause(why) };
        rows += 1;
        if (!steppedEql(got, wanted)) differing += 1;
    }
    try std.testing.expectEqual(@as(usize, 160), rows);
    try std.testing.expectEqual(@as(usize, 0), differing);
}
