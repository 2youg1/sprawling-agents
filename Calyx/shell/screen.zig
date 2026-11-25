// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

const std = @import("std");
const leaf = @import("leaf");
const write = leaf.write;
const part = leaf.part;
const termio = @import("termio.zig");

const first_capacity: usize = 16 * 1024;
const longest: usize = 64 * 1024 * 1024;

pub const Written = enum { shown, lost };

pub const QuietLines = struct {
    url: []const u8,
    code: []const u8,
    key: ?[]const u8 = null,
    transient: ?[]const u8 = null,
};

pub const Refused = error{
    malformed,
    too_long,
};

fn refusedMessage(reason: Refused) []const u8 {
    return switch (reason) {
        error.malformed => "the console's renderer could not read the scene it was given; transcript in refused frames is discarded; read replies in the WebUI\r\n",
        error.too_long => "the console's frame did not fit in the output buffer; transcript in refused frames is discarded; read replies in the WebUI\r\n",
    };
}

pub const Screen = struct {
    gpa: std.mem.Allocator,
    lost: bool = false,
    erase: u16 = 0,
    previous: ?part.Part = null,
    frame: std.ArrayList(u8) = .empty,
    refused: ?Refused = null,

    pub fn deinit(self: *Screen) void {
        self.frame.deinit(self.gpa);
    }

    pub fn take(self: *Screen, title: []const u8, style: ?termio.CursorStyle) Written {
        termio.enableRaw() catch |err| {
            std.debug.print("Calyx terminal initialization failed: {s}\n", .{@errorName(err)});
            self.lost = true;
            return .lost;
        };
        termio.setTitle(title) catch return self.fail();
        if (style) |value| self.setStyle(value) catch return self.fail();
        termio.enhanceKeyboard() catch return self.fail();
        return .shown;
    }

    pub fn giveBack(self: *Screen) Written {
        const left = self.leaveQuiet();
        const erased = self.frameEntries(&.{}, null);
        termio.plainKeyboard() catch {};
        self.setStyle(.default) catch {};
        _ = self.writeSlice("\x1b[?25h");
        termio.disableRaw() catch {};
        return worst(worst(left, erased), if (self.lost) .lost else .shown);
    }

    pub fn isQuiet(self: *const Screen) bool {
        _ = self;
        return termio.isAlternate();
    }

    pub fn cut(self: *Screen) void {
        self.lost = true;
    }

    pub fn frameEntries(
        self: *Screen,
        entries: []const write.Entry,
        live: ?write.Live,
    ) Written {
        if (self.lost or self.isQuiet()) return .shown;
        self.render(entries, live, termio.size()) catch |reason| {
            if (self.refused) |prior| if (prior == reason) return .shown;
            self.refused = reason;
            const erase = self.erase;
            self.erase = 0;
            return self.writeRefused(erase, reason);
        };
        return self.flushFrame();
    }

    fn render(
        self: *Screen,
        entries: []const write.Entry,
        live: ?write.Live,
        size: termio.Size,
    ) Refused!void {
        var scene = write.Scene.inlineFrame(self.gpa, .{
            .columns = size.columns,
            .rows = size.rows,
            .erase = self.erase,
            .previous = self.previous,
            .entries = entries,
            .live = live,
        }) catch return error.too_long;
        defer scene.deinit(self.gpa);
        const drawn = try drawScene(self.gpa, scene.bytes.items, &self.frame);
        self.erase = std.math.cast(u16, drawn.cursor_row) orelse std.math.maxInt(u16);
        self.refused = null;
        if (entries.len > 0) self.previous = entries[entries.len - 1].tag();
    }

    pub fn quiet(self: *Screen, lines: QuietLines) Written {
        const entered: Written = if (termio.isAlternate())
            .shown
        else blk: {
            const erased = self.frameEntries(&.{}, null);
            termio.enterAlternate() catch break :blk self.fail();
            self.erase = 0;
            break :blk erased;
        };
        const size = termio.size();
        var scene = write.Scene.quietFrame(self.gpa, .{
            .columns = size.columns,
            .rows = size.rows,
            .url = lines.url,
            .code = lines.code,
            .key = lines.key,
            .transient = lines.transient,
        }) catch return entered;
        defer scene.deinit(self.gpa);
        _ = drawScene(self.gpa, scene.bytes.items, &self.frame) catch return entered;
        return worst(entered, self.flushFrame());
    }

    pub fn leaveQuiet(self: *Screen) Written {
        if (!termio.isAlternate()) return .shown;
        termio.leaveAlternate() catch return self.fail();
        return .shown;
    }

    pub fn report(self: *Screen, ansi: []const u8) Written {
        return self.writeSlice(ansi);
    }

    fn writeRefused(self: *Screen, erase: u16, reason: Refused) Written {
        self.writeAll("\r") catch return self.fail();
        if (erase > 0) {
            var buf: [32]u8 = undefined;
            const seq = std.fmt.bufPrint(&buf, "\x1b[{d}A", .{erase}) catch return self.fail();
            self.writeAll(seq) catch return self.fail();
        }
        self.writeAll("\x1b[J") catch return self.fail();
        self.writeAll(refusedMessage(reason)) catch return self.fail();
        return .shown;
    }

    fn setStyle(self: *Screen, style: termio.CursorStyle) termio.Error!void {
        _ = self;
        try termio.setCursorStyle(style);
    }

    fn writeAll(self: *Screen, bytes: []const u8) termio.Error!void {
        if (self.lost) return;
        try termio.writeAll(bytes);
    }

    fn flushFrame(self: *Screen) Written {
        return self.writeSlice(self.frame.items);
    }

    fn writeSlice(self: *Screen, bytes: []const u8) Written {
        self.writeAll(bytes) catch return self.fail();
        return .shown;
    }

    fn fail(self: *Screen) Written {
        self.lost = true;
        return .lost;
    }
};

fn drawScene(gpa: std.mem.Allocator, bytes: []const u8, into: *std.ArrayList(u8)) Refused!leaf.Drawn {
    var capacity = @max(into.capacity, first_capacity);
    while (true) {
        into.resize(gpa, capacity) catch return error.too_long;
        var drawn: leaf.Drawn = undefined;
        const status = leaf.sprawling_console_draw(
            bytes.ptr,
            bytes.len,
            into.items.ptr,
            into.items.len,
            &drawn,
        );
        if (status == @backingInt(part.Status.Drawn)) {
            into.shrinkRetainingCapacity(drawn.written);
            return drawn;
        }
        if (status == @backingInt(part.Status.Full) and capacity < longest) {
            capacity = @min(capacity *| 2, longest);
            continue;
        }
        into.clearRetainingCapacity();
        return if (status == @backingInt(part.Status.Full)) error.too_long else error.malformed;
    }
}

fn worst(first: Written, second: Written) Written {
    if (first == .lost or second == .lost) return .lost;
    return .shown;
}

test "render updates erase and previous from a drawn inline scene" {
    const gpa = std.testing.allocator;
    var screen: Screen = .{ .gpa = gpa };
    defer screen.deinit();

    const entries = [_]write.Entry{.{ .reply = "ok" }};
    try screen.render(&entries, null, .{ .columns = 40, .rows = 12 });
    try std.testing.expect(screen.frame.items.len > 0);
    try std.testing.expectEqual(part.Part.Reply, screen.previous.?);
    try std.testing.expect(screen.refused == null);
}

test "city draws a long workspace banner and anchors the composer in the visible window" {
    var screen: Screen = .{ .gpa = std.testing.allocator };
    defer screen.deinit();
    const banner = [_]write.Entry{.{ .banner = .{
        .city = "Z:\\Profiles\\terminal-operator\\.sprawling\\w\\0123456789abcdef0123456789abcdef",
        .address = "127.0.0.1:2501",
        .url = "http://127.0.0.1:2501",
    } }};
    const live: write.Live = .{ .composer = .{
        .typed = "",
        .cursor = 0,
        .placeholder = "to the city…",
        .room = "city",
        .offer = "",
    } };
    try screen.render(&banner, live, .{ .columns = 140, .rows = 49 });
    try std.testing.expect(std.mem.indexOf(u8, screen.frame.items, "/web") != null);
    try std.testing.expect(std.mem.indexOf(u8, screen.frame.items, "\x1b[46;1H") != null);

    screen.erase = 0;
    screen.previous = null;
    try screen.render(&banner, live, .{ .columns = 80, .rows = 24 });
    try std.testing.expect(std.mem.indexOf(u8, screen.frame.items, "/web") == null);
    try std.testing.expect(std.mem.indexOf(u8, screen.frame.items, "\x1b[21;1H") != null);
}

test "a refused oversized scene does not set previous" {
    const gpa = std.testing.allocator;
    var screen: Screen = .{ .gpa = gpa };
    defer screen.deinit();

    const huge = try gpa.alloc(u8, 24 * 1024 * 1024);
    defer gpa.free(huge);
    @memset(huge, 'x');
    const entries = [_]write.Entry{.{ .reply = huge }};
    try std.testing.expectError(error.too_long, screen.render(&entries, null, .{ .columns = 2, .rows = 24 }));
    try std.testing.expect(screen.previous == null);
}

test "screen redraw GPA leak stress" {
    var gpa_state: std.heap.DebugAllocator(.{}) = .init;
    defer std.testing.expect(gpa_state.deinit() == .ok) catch @panic("screen GPA leak");
    const gpa = gpa_state.allocator();
    var screen: Screen = .{ .gpa = gpa };
    defer screen.deinit();
    var round: usize = 0;
    while (round < 256) : (round += 1) {
        var note_buf: [32]u8 = undefined;
        const note = try std.fmt.bufPrint(&note_buf, "line-{d}", .{round});
        const entries = [_]write.Entry{.{ .note = note }};
        const live: ?write.Live = if (round % 3 == 0) .{
            .waiting = null,
            .working = .{ .resident = "city", .since = @intCast(@min(round, std.math.maxInt(u32))) },
            .calling = &.{},
            .asking = if (round % 7 == 0) @as(?u64, 2) else null,
            .composer = .{
                .typed = "hello",
                .cursor = 2,
                .placeholder = "say",
                .room = "city",
                .offer = "model · unknown",
            },
            .menu = null,
        } else null;
        try screen.render(&entries, live, .{ .columns = 80, .rows = 24 });
    }
}
