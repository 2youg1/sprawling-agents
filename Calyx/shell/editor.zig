// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

const std = @import("std");
const leaf = @import("leaf");

pub const Edit = union(enum) {
    insert: u21,
    newline,
    backspace,
    delete,
    left,
    right,
    home,
    end,
    word_left,
    word_right,
    delete_word,
    older,
    newer,
    line_up,
    line_down,
    clear,
};

pub const Editor = struct {
    allocator: std.mem.Allocator,
    text: std.ArrayList(u21) = .empty,
    cursor: usize = 0,
    history: std.ArrayList([]u8) = .empty,
    recalled: ?usize = null,
    draft: std.ArrayList(u21) = .empty,
    goal: ?usize = null,

    pub fn init(allocator: std.mem.Allocator) Editor {
        return .{ .allocator = allocator };
    }

    pub fn deinit(self: *Editor) void {
        self.text.deinit(self.allocator);
        self.draft.deinit(self.allocator);
        for (self.history.items) |line| self.allocator.free(line);
        self.history.deinit(self.allocator);
        self.* = undefined;
    }

    pub fn isEmpty(self: *const Editor) bool {
        return self.text.items.len == 0;
    }

    pub fn textOwned(self: *const Editor) ![]u8 {
        return try encode(self.allocator, self.text.items);
    }

    pub fn cursorAt(self: *const Editor) usize {
        return self.cursor;
    }

    pub fn paste(self: *Editor, pasted: []const u8) !void {
        self.goal = null;
        var i: usize = 0;
        while (i < pasted.len) {
            const utf8_len = @field(std.unicode, "utf8" ++ "ByteSequenceLength");
            const len = try utf8_len(pasted[i]);
            if (i + len > pasted.len) return error.InvalidUtf8;
            const ch = try std.unicode.utf8Decode(pasted[i..][0..len]);
            i += len;
            if (ch == '\r') continue;
            try self.insert(ch);
        }
    }

    pub fn set(self: *Editor, value: []const u8) !void {
        self.goal = null;
        self.text.clearRetainingCapacity();
        try self.paste(value);
        self.cursor = self.text.items.len;
    }

    pub fn apply(self: *Editor, edit: Edit) !void {
        switch (edit) {
            .older => {
                self.goal = null;
                try self.recall(if (self.recalled) |n| n +| 1 else 0);
                return;
            },
            .newer => {
                self.goal = null;
                if (self.recalled) |n| {
                    if (n == 0) {
                        self.recalled = null;
                        self.text.deinit(self.allocator);
                        self.text = self.draft;
                        self.draft = .empty;
                        self.cursor = self.text.items.len;
                    } else try self.recall(n - 1);
                }
                return;
            },
            .line_up => {
                self.lineUp();
                return;
            },
            .line_down => {
                self.lineDown();
                return;
            },
            else => self.goal = null,
        }
        switch (edit) {
            .insert => |ch| try self.insert(ch),
            .newline => try self.insert('\n'),
            .backspace => {
                if (self.cursor > 0) {
                    const before = self.cursor - 1;
                    _ = self.text.orderedRemove(before);
                    self.cursor = before;
                }
            },
            .delete => {
                if (self.cursor < self.text.items.len) {
                    _ = self.text.orderedRemove(self.cursor);
                }
            },
            .left => self.cursor = std.math.sub(usize, self.cursor, 1) catch 0,
            .right => self.cursor = @min(self.cursor +| 1, self.text.items.len),
            .home => self.cursor = 0,
            .end => self.cursor = self.text.items.len,
            .word_left => self.cursor = self.wordStart(),
            .word_right => self.cursor = self.wordEnd(),
            .delete_word => {
                const start = self.wordStart();
                const count = self.cursor - start;
                var n: usize = 0;
                while (n < count) : (n += 1) {
                    _ = self.text.orderedRemove(start);
                }
                self.cursor = start;
            },
            .clear => {
                self.text.clearRetainingCapacity();
                self.cursor = 0;
                self.recalled = null;
            },
            .older, .newer, .line_up, .line_down => unreachable,
        }
    }

    pub fn enter(self: *Editor) !?[]u8 {
        if (self.cursor == self.text.items.len and self.text.items.len > 0 and
            self.text.items[self.text.items.len - 1] == '\\')
        {
            _ = self.text.pop();
            self.cursor = self.text.items.len;
            try self.insert('\n');
            return null;
        }
        const line = try self.textOwned();
        try self.apply(.clear);
        const trimmed = std.mem.trim(u8, line, &std.ascii.whitespace);
        if (trimmed.len != 0) {
            const last = if (self.history.items.len == 0) null else self.history.items[self.history.items.len - 1];
            if (last == null or !std.mem.eql(u8, last.?, line)) {
                try self.history.append(self.allocator, try self.allocator.dupe(u8, line));
            }
        }
        return line;
    }

    fn insert(self: *Editor, ch: u21) !void {
        try self.text.insert(self.allocator, self.cursor, ch);
        self.cursor = self.cursor +| 1;
    }

    fn recall(self: *Editor, back: usize) !void {
        const index = std.math.sub(usize, self.history.items.len, back +| 1) catch return;
        if (index >= self.history.items.len) return;
        const line = self.history.items[index];
        if (self.recalled == null) {
            self.draft.deinit(self.allocator);
            self.draft = self.text;
            self.text = .empty;
        } else {
            self.text.clearRetainingCapacity();
        }
        self.cursor = 0;
        try self.paste(line);
        self.cursor = self.text.items.len;
        self.recalled = back;
        self.goal = null;
    }

    fn lineUp(self: *Editor) void {
        const goal = self.goal orelse self.columnOf(self.cursor);
        self.goal = goal;
        const start = self.lineStart(self.cursor);
        if (start == 0) return;
        self.cursor = self.indexAtColumn(self.lineStart(start - 1), goal);
    }

    fn lineDown(self: *Editor) void {
        const goal = self.goal orelse self.columnOf(self.cursor);
        self.goal = goal;
        const end = self.lineEnd(self.cursor);
        if (end >= self.text.items.len) return;
        self.cursor = self.indexAtColumn(end + 1, goal);
    }

    fn lineStart(self: *const Editor, index: usize) usize {
        var i = index;
        while (i > 0 and self.text.items[i - 1] != '\n') i -= 1;
        return i;
    }

    fn lineEnd(self: *const Editor, index: usize) usize {
        var i = index;
        while (i < self.text.items.len and self.text.items[i] != '\n') i += 1;
        return i;
    }

    fn columnOf(self: *const Editor, index: usize) usize {
        var col: usize = 0;
        var i = self.lineStart(index);
        while (i < index) : (i += 1) col += leaf.text.columns(self.text.items[i]);
        return col;
    }

    fn indexAtColumn(self: *const Editor, line_start: usize, goal: usize) usize {
        var col: usize = 0;
        var i = line_start;
        while (i < self.text.items.len and self.text.items[i] != '\n') {
            const width = leaf.text.columns(self.text.items[i]);
            if (col >= goal or col + width > goal) return i;
            col += width;
            i += 1;
        }
        return i;
    }

    fn wordStart(self: *const Editor) usize {
        const before = self.text.items[0..self.cursor];
        var last_non_ws: ?usize = null;
        var i = before.len;
        while (i > 0) {
            i -= 1;
            if (!isWhitespace(before[i])) {
                last_non_ws = i;
                break;
            }
        }
        const last = last_non_ws orelse return 0;
        var j = last;
        while (j > 0) {
            j -= 1;
            if (isWhitespace(before[j])) return j + 1;
        }
        return 0;
    }

    fn wordEnd(self: *const Editor) usize {
        const after = self.text.items[self.cursor..];
        var first_non_ws: ?usize = null;
        for (after, 0..) |c, i| {
            if (!isWhitespace(c)) {
                first_non_ws = i;
                break;
            }
        }
        const first = first_non_ws orelse return self.text.items.len;
        const rest = after[first..];
        var length: usize = rest.len;
        for (rest, 0..) |c, i| {
            if (isWhitespace(c)) {
                length = i;
                break;
            }
        }
        return self.cursor +| first +| length;
    }
};

fn isWhitespace(c: u21) bool {
    return switch (c) {
        ' ',
        '\t',
        '\n',
        '\r',
        0x0b,
        0x0c,
        0xa0,
        0x1680,
        0x2000...0x200a,
        0x2028,
        0x2029,
        0x202f,
        0x205f,
        0x3000,
        => true,
        else => false,
    };
}

fn encode(allocator: std.mem.Allocator, chars: []const u21) ![]u8 {
    var out: std.ArrayList(u8) = .empty;
    errdefer out.deinit(allocator);
    var buf: [4]u8 = undefined;
    for (chars) |ch| {
        const n = try std.unicode.utf8Encode(ch, &buf);
        try out.appendSlice(allocator, buf[0..n]);
    }
    return try out.toOwnedSlice(allocator);
}

test "insert paste and cursor motion" {
    var ed = Editor.init(std.testing.allocator);
    defer ed.deinit();
    try ed.paste("ab");
    try ed.apply(.{ .insert = 'c' });
    try ed.apply(.home);
    try ed.apply(.{ .insert = 'x' });
    const t = try ed.textOwned();
    defer std.testing.allocator.free(t);
    try std.testing.expectEqualStrings("xabc", t);
    try std.testing.expectEqual(@as(usize, 1), ed.cursorAt());
}

test "set replaces text when the cursor is not at the start" {
    var ed = Editor.init(std.testing.allocator);
    defer ed.deinit();
    try ed.set("/quit");
    try ed.set("/quit");
    const text = try ed.textOwned();
    defer std.testing.allocator.free(text);
    try std.testing.expectEqualStrings("/quit", text);
    try std.testing.expectEqual(@as(usize, 5), ed.cursorAt());
}

test "backslash at end continues the line" {
    var ed = Editor.init(std.testing.allocator);
    defer ed.deinit();
    try ed.set("hi\\");
    const cont = try ed.enter();
    try std.testing.expect(cont == null);
    const t = try ed.textOwned();
    defer std.testing.allocator.free(t);
    try std.testing.expectEqualStrings("hi\n", t);
}

test "enter records history once" {
    var ed = Editor.init(std.testing.allocator);
    defer ed.deinit();
    try ed.set("one");
    const a = (try ed.enter()).?;
    defer std.testing.allocator.free(a);
    try ed.set("one");
    const b = (try ed.enter()).?;
    defer std.testing.allocator.free(b);
    try std.testing.expectEqual(@as(usize, 1), ed.history.items.len);
}

test "older and newer recall draft" {
    var ed = Editor.init(std.testing.allocator);
    defer ed.deinit();
    try ed.set("first");
    const a = (try ed.enter()).?;
    defer std.testing.allocator.free(a);
    try ed.set("draft");
    try ed.apply(.older);
    const recalled = try ed.textOwned();
    defer std.testing.allocator.free(recalled);
    try std.testing.expectEqualStrings("first", recalled);
    try ed.apply(.newer);
    const draft = try ed.textOwned();
    defer std.testing.allocator.free(draft);
    try std.testing.expectEqualStrings("draft", draft);
}

test "paste clears the vertical column goal" {
    var ed = Editor.init(std.testing.allocator);
    defer ed.deinit();
    try ed.set("abcd\nwxyz");
    try ed.apply(.home);
    try ed.apply(.right);
    try ed.apply(.right);
    try ed.apply(.line_down);
    try ed.paste("X");
    try ed.apply(.line_up);
    try std.testing.expectEqual(@as(usize, 3), ed.cursorAt());
}

test "up and down move between lines until the ends of the text" {
    var ed = Editor.init(std.testing.allocator);
    defer ed.deinit();
    try ed.set("中文\nmore");
    try ed.apply(.line_up);
    try std.testing.expectEqual(@as(usize, 2), ed.cursorAt());
    try ed.apply(.home);
    try ed.apply(.line_down);
    try std.testing.expectEqual(@as(usize, 3), ed.cursorAt());
    try ed.apply(.home);
    try ed.apply(.line_up);
    try std.testing.expectEqual(@as(usize, 0), ed.cursorAt());
}

test "word left right and delete word" {
    var ed = Editor.init(std.testing.allocator);
    defer ed.deinit();
    try ed.set("aa bb cc");
    try ed.apply(.word_left);
    try std.testing.expectEqual(@as(usize, 6), ed.cursorAt());
    try ed.apply(.delete_word);
    const t = try ed.textOwned();
    defer std.testing.allocator.free(t);
    try std.testing.expectEqualStrings("aa cc", t);
    try ed.apply(.end);
    try ed.apply(.word_left);
    try std.testing.expectEqual(@as(usize, 3), ed.cursorAt());
    try ed.apply(.word_right);
    try std.testing.expectEqual(@as(usize, 5), ed.cursorAt());
}

test "seeded editor fuzz keeps cursor in range" {
    var prng = std.Random.DefaultPrng.init(0xe011_70f);
    const random = prng.random();
    var ed = Editor.init(std.testing.allocator);
    defer ed.deinit();
    var round: usize = 0;
    while (round < 2048) : (round += 1) {
        const edit: Edit = switch (random.intRangeAtMost(u8, 0, 12)) {
            0 => .{ .insert = random.intRangeAtMost(u21, 32, 126) },
            1 => .newline,
            2 => .backspace,
            3 => .delete,
            4 => .left,
            5 => .right,
            6 => .home,
            7 => .end,
            8 => .word_left,
            9 => .word_right,
            10 => .delete_word,
            11 => .older,
            else => .newer,
        };
        ed.apply(edit) catch {};
        try std.testing.expect(ed.cursorAt() <= ed.text.items.len);
    }
}

test "editor GPA leak stress" {
    var gpa_state: std.heap.DebugAllocator(.{}) = .init;
    defer std.testing.expect(gpa_state.deinit() == .ok) catch @panic("editor GPA leak");
    const gpa = gpa_state.allocator();
    var ed = Editor.init(gpa);
    defer ed.deinit();
    var round: usize = 0;
    while (round < 10_000) : (round += 1) {
        try ed.apply(.{ .insert = 'a' + @as(u21, @intCast(round % 26)) });
        if (round % 17 == 0) try ed.apply(.backspace);
        if (round % 41 == 0) {
            if (try ed.enter()) |line| gpa.free(line);
        }
        if (round % 53 == 0) try ed.paste("paste-utf8-字");
        if (round % 59 == 0) try ed.apply(.older);
        if (round % 61 == 0) try ed.apply(.newer);
        if (round % 67 == 0) try ed.apply(.clear);
    }
}
