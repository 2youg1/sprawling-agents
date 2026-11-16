// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// POSIX terminal I/O for the Calyx shell: raw mode, alternate screen,
// cursor style, and parsing of key / paste / resize escape bytes. Unit
// tests exercise the parser only; enable/disable need a real TTY.

const std = @import("std");
const builtin = @import("builtin");
const posix = std.posix;

pub const Modifiers = packed struct(u8) {
    control: bool = false,
    alt: bool = false,
    shift: bool = false,
    _pad: u5 = 0,
};

pub const Code = union(enum) {
    char: u21,
    enter,
    esc,
    backspace,
    delete,
    tab,
    back_tab,
    left,
    right,
    up,
    down,
    home,
    end,
    unknown,
};

pub const Key = struct {
    code: Code,
    modifiers: Modifiers = .{},
};

pub const Size = struct { columns: u16, rows: u16 };

pub const Event = union(enum) {
    key: Key,
    paste: []const u8,
    resize: Size,
    lost,
};

pub const CursorStyle = enum {
    default,
    steady_bar,
};

pub const Error = error{
    NotATerminal,
    ProcessOrphaned,
    Unexpected,
    WriteFailed,
};

var saved: ?posix.termios = null;
var alternate: bool = false;

pub fn enableRaw() Error!void {
    if (builtin.os.tag == .windows) return error.NotATerminal;
    if (saved != null) return;
    const fd = posix.STDIN_FILENO;
    var term = posix.tcgetattr(fd) catch |err| return mapTermios(err);
    saved = term;
    term.lflag.ECHO = false;
    term.lflag.ICANON = false;
    term.lflag.ISIG = false;
    term.lflag.IEXTEN = false;
    term.iflag.IXON = false;
    term.iflag.ICRNL = false;
    term.iflag.BRKINT = false;
    term.iflag.INPCK = false;
    term.iflag.ISTRIP = false;
    term.oflag.OPOST = false;
    term.cc[@intFromEnum(posix.V.MIN)] = 1;
    term.cc[@intFromEnum(posix.V.TIME)] = 0;
    posix.tcsetattr(fd, .NOW, term) catch |err| return mapTermios(err);
}

pub fn disableRaw() Error!void {
    if (builtin.os.tag == .windows) return;
    const previous = saved orelse return;
    posix.tcsetattr(posix.STDIN_FILENO, .NOW, previous) catch |err| return mapTermios(err);
    saved = null;
}

pub fn enterAlternate() Error!void {
    try writeAll("\x1b[?1049h\x1b[?25l");
    alternate = true;
}

pub fn leaveAlternate() Error!void {
    if (!alternate) return;
    try writeAll("\x1b[2J\x1b[H\x1b[?1049l\x1b[?25h");
    alternate = false;
}

pub fn isAlternate() bool {
    return alternate;
}

pub fn setCursorStyle(style: CursorStyle) Error!void {
    try writeAll(switch (style) {
        .default => "\x1b[0 q",
        .steady_bar => "\x1b[6 q",
    });
}

pub fn enhanceKeyboard() Error!void {
    if (builtin.os.tag == .windows) return;
    try writeAll("\x1b[>1u\x1b[?2004h");
}

pub fn plainKeyboard() Error!void {
    if (builtin.os.tag == .windows) return;
    try writeAll("\x1b[<u\x1b[?2004l");
}

pub fn setTitle(title: []const u8) Error!void {
    try writeAll("\x1b]2;");
    try writeAll(title);
    try writeAll("\x07");
}

pub fn writeAll(bytes: []const u8) Error!void {
    if (builtin.os.tag == .windows) return error.WriteFailed;
    var written: usize = 0;
    while (written < bytes.len) {
        const n = std.c.write(posix.STDOUT_FILENO, bytes[written..].ptr, bytes.len - written);
        if (n < 0) return error.WriteFailed;
        if (n == 0) return error.WriteFailed;
        written += @intCast(n);
    }
}

pub fn size() Size {
    if (builtin.os.tag == .windows) return .{ .columns = 80, .rows = 24 };
    var wsz: posix.winsize = .{ .row = 0, .col = 0, .xpixel = 0, .ypixel = 0 };
    const rc = posix.system.ioctl(posix.STDOUT_FILENO, posix.T.IOCGWINSZ, @intFromPtr(&wsz));
    if (posix.errno(rc) != .SUCCESS or wsz.col == 0 or wsz.row == 0) {
        return .{ .columns = 80, .rows = 24 };
    }
    return .{ .columns = wsz.col, .rows = wsz.row };
}

/// Read available stdin bytes into `buf` and parse one event. Returns
/// `null` when more bytes are needed; `lost` when the stream ends.
pub fn readEvent(buf: []u8, filled: *usize) Error!?Event {
    if (builtin.os.tag == .windows) return .lost;
    if (filled.* == 0) {
        const n = posix.read(posix.STDIN_FILENO, buf) catch return .lost;
        if (n == 0) return .lost;
        filled.* = n;
    }
    var used: usize = 0;
    const event = parseOne(buf[0..filled.*], &used) orelse {
        if (filled.* == buf.len) {
            @memmove(buf[0 .. filled.* - 1], buf[1..filled.*]);
            filled.* -= 1;
        }
        const n = posix.read(posix.STDIN_FILENO, buf[filled.*..]) catch return .lost;
        if (n == 0) return .lost;
        filled.* += n;
        return null;
    };
    const rest = filled.* - used;
    if (rest > 0) @memmove(buf[0..rest], buf[used..filled.*]);
    filled.* = rest;
    return event;
}

/// Pure escape-sequence parser: consumes one event from `input`, writes
/// how many bytes it took into `used`, and returns null when incomplete.
pub fn parseOne(input: []const u8, used: *usize) ?Event {
    if (input.len == 0) return null;
    if (input[0] == 0x1b) return parseEsc(input, used);
    used.* = 1;
    const b = input[0];
    if (b == 0x7f or b == 0x08) return .{ .key = .{ .code = .backspace } };
    if (b == '\r' or b == '\n') return .{ .key = .{ .code = .enter } };
    if (b == '\t') return .{ .key = .{ .code = .tab } };
    if (b < 0x20) {
        return .{ .key = .{
            .code = .{ .char = @as(u21, b) | 0x60 },
            .modifiers = .{ .control = true },
        } };
    }
    if (b < 0x80) return .{ .key = .{ .code = .{ .char = b } } };
    const len = std.unicode.utf8ByteSequenceLength(b) catch {
        return .{ .key = .{ .code = .unknown } };
    };
    if (input.len < len) {
        used.* = 0;
        return null;
    }
    const cp = std.unicode.utf8Decode(input[0..len]) catch {
        used.* = 1;
        return .{ .key = .{ .code = .unknown } };
    };
    used.* = len;
    return .{ .key = .{ .code = .{ .char = cp } } };
}

fn parseEsc(input: []const u8, used: *usize) ?Event {
    if (input.len == 1) {
        used.* = 0;
        return null;
    }
    if (input[1] == '[') return parseCsi(input, used);
    if (input[1] == 'O' and input.len >= 3) {
        used.* = 3;
        return .{ .key = .{ .code = switch (input[2]) {
            'A' => .up,
            'B' => .down,
            'C' => .right,
            'D' => .left,
            'H' => .home,
            'F' => .end,
            else => .unknown,
        } } };
    }
    used.* = 2;
    return .{ .key = .{
        .code = .{ .char = input[1] },
        .modifiers = .{ .alt = true },
    } };
}

fn parseCsi(input: []const u8, used: *usize) ?Event {
    if (input.len < 3) {
        used.* = 0;
        return null;
    }
    if (std.mem.startsWith(u8, input, "\x1b[200~")) {
        const start = "\x1b[200~".len;
        const end_mark = "\x1b[201~";
        const end = std.mem.indexOf(u8, input[start..], end_mark) orelse {
            used.* = 0;
            return null;
        };
        used.* = start + end + end_mark.len;
        return .{ .paste = input[start .. start + end] };
    }
    var i: usize = 2;
    var params: [4]u16 = .{ 0, 0, 0, 0 };
    var param_i: usize = 0;
    var current: u16 = 0;
    var saw_digit = false;
    while (i < input.len) : (i += 1) {
        const c = input[i];
        if (c >= '0' and c <= '9') {
            saw_digit = true;
            current = current *% 10 +% (c - '0');
            continue;
        }
        if (c == ';' or c == ':') {
            if (param_i < params.len) {
                params[param_i] = current;
                param_i += 1;
            }
            current = 0;
            saw_digit = false;
            continue;
        }
        if (saw_digit and param_i < params.len) {
            params[param_i] = current;
            param_i += 1;
        }
        used.* = i + 1;
        const mods = modifiersFromParam(if (param_i >= 2) params[1] else if (param_i == 1 and c == 'u') params[0] else 0);
        return switch (c) {
            'A' => .{ .key = .{ .code = .up, .modifiers = mods } },
            'B' => .{ .key = .{ .code = .down, .modifiers = mods } },
            'C' => .{ .key = .{ .code = .right, .modifiers = mods } },
            'D' => .{ .key = .{ .code = .left, .modifiers = mods } },
            'H' => .{ .key = .{ .code = .home, .modifiers = mods } },
            'F' => .{ .key = .{ .code = .end, .modifiers = mods } },
            'Z' => .{ .key = .{ .code = .back_tab, .modifiers = .{ .shift = true } } },
            '~' => .{ .key = .{ .code = codeFromTilde(params[0]), .modifiers = mods } },
            't' => blk: {
                if (params[0] == 8 and param_i >= 3) {
                    break :blk .{ .resize = .{ .rows = params[1], .columns = params[2] } };
                }
                break :blk .{ .key = .{ .code = .unknown } };
            },
            'u' => .{ .key = .{
                .code = .{ .char = params[0] },
                .modifiers = if (param_i >= 2) modifiersFromParam(params[1]) else .{},
            } },
            else => .{ .key = .{ .code = .unknown } },
        };
    }
    used.* = 0;
    return null;
}

fn codeFromTilde(n: u16) Code {
    return switch (n) {
        1, 7 => .home,
        4, 8 => .end,
        3 => .delete,
        else => .unknown,
    };
}

fn modifiersFromParam(value: u16) Modifiers {
    if (value < 2) return .{};
    const bits = value - 1;
    return .{
        .shift = bits & 1 != 0,
        .alt = bits & 2 != 0,
        .control = bits & 4 != 0,
    };
}

fn mapTermios(err: anyerror) Error {
    return switch (err) {
        error.NotATerminal => error.NotATerminal,
        error.ProcessOrphaned => error.ProcessOrphaned,
        else => error.Unexpected,
    };
}

test "parser reads plain and control characters" {
    var used: usize = 0;
    const a = parseOne("a", &used).?;
    try std.testing.expectEqual(@as(usize, 1), used);
    try std.testing.expectEqual(Code{ .char = 'a' }, a.key.code);

    const ctrl = parseOne(&.{0x03}, &used).?;
    try std.testing.expectEqual(Code{ .char = 'c' }, ctrl.key.code);
    try std.testing.expect(ctrl.key.modifiers.control);

    const enter = parseOne("\r", &used).?;
    try std.testing.expectEqual(Code.enter, enter.key.code);
}

test "parser reads arrows, delete, and back-tab" {
    var used: usize = 0;
    const up = parseOne("\x1b[A", &used).?;
    try std.testing.expectEqual(@as(usize, 3), used);
    try std.testing.expectEqual(Code.up, up.key.code);

    const del = parseOne("\x1b[3~", &used).?;
    try std.testing.expectEqual(Code.delete, del.key.code);

    const back = parseOne("\x1b[Z", &used).?;
    try std.testing.expectEqual(Code.back_tab, back.key.code);
    try std.testing.expect(back.key.modifiers.shift);
}

test "parser reads bracketed paste and resize" {
    var used: usize = 0;
    const paste = parseOne("\x1b[200~hello\x1b[201~", &used).?;
    try std.testing.expectEqualStrings("hello", paste.paste);

    const incomplete = parseOne("\x1b[200~hel", &used);
    try std.testing.expect(incomplete == null);
    try std.testing.expectEqual(@as(usize, 0), used);

    const resize = parseOne("\x1b[8;24;80t", &used).?;
    try std.testing.expectEqual(@as(u16, 24), resize.resize.rows);
    try std.testing.expectEqual(@as(u16, 80), resize.resize.columns);
}

test "parser reads shifted arrow modifiers" {
    var used: usize = 0;
    const key = parseOne("\x1b[1;2C", &used).?;
    try std.testing.expectEqual(Code.right, key.key.code);
    try std.testing.expect(key.key.modifiers.shift);
}

test "seeded termio fuzz never traps" {
    var prng = std.Random.DefaultPrng.init(0x7e12_0c01);
    const random = prng.random();
    var buf: [96]u8 = undefined;
    var round: usize = 0;
    while (round < 4096) : (round += 1) {
        const n = random.intRangeAtMost(usize, 0, buf.len);
        random.bytes(buf[0..n]);
        if (n >= 2 and random.boolean()) {
            buf[0] = 0x1b;
            buf[1] = '[';
        }
        if (n >= 6 and random.boolean()) {
            @memcpy(buf[0..6], "\x1b[200~");
        }
        var used: usize = 0;
        const event = parseOne(buf[0..n], &used);
        if (event) |ev| {
            try std.testing.expect(used > 0);
            try std.testing.expect(used <= n);
            switch (ev) {
                .key, .paste, .resize, .lost => {},
            }
        } else {
            try std.testing.expectEqual(@as(usize, 0), used);
        }
    }
}
