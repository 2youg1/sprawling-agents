// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

const std = @import("std");
const builtin = @import("builtin");
const posix = std.posix;
const is_windows = builtin.os.tag == .windows;

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
    click: struct { column: u16, row: u16, button: u16, release: bool },
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

var saved_unix: if (is_windows) void else ?posix.termios = if (is_windows) {} else null;
var saved_win: if (is_windows) ?WinModes else void = if (is_windows) null else {};
var alternate: bool = false;

const WinModes = struct {
    in_mode: u32,
    out_mode: u32,
};

const ENABLE_PROCESSED_INPUT: u32 = 0x0001;
const ENABLE_LINE_INPUT: u32 = 0x0002;
const ENABLE_ECHO_INPUT: u32 = 0x0004;
const ENABLE_VIRTUAL_TERMINAL_INPUT: u32 = 0x0200;
const ENABLE_VIRTUAL_TERMINAL_PROCESSING: u32 = 0x0004;

pub fn enableRaw() Error!void {
    if (is_windows) return enableRawWindows();
    if (saved_unix != null) return;
    const fd = posix.STDIN_FILENO;
    var term = posix.tcgetattr(fd) catch |err| return mapTermios(err);
    const original = term;
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
    term.cc[@backingInt(posix.V.MIN)] = 1;
    term.cc[@backingInt(posix.V.TIME)] = 0;
    posix.tcsetattr(fd, .NOW, term) catch |err| return mapTermios(err);
    saved_unix = original;
}

pub fn disableRaw() Error!void {
    if (is_windows) return disableRawWindows();
    const previous = saved_unix orelse return;
    posix.tcsetattr(posix.STDIN_FILENO, .NOW, previous) catch |err| return mapTermios(err);
    saved_unix = null;
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
    try writeAll("\x1b[>1u\x1b[?2004h");
}

pub fn plainKeyboard() Error!void {
    try writeAll("\x1b[<u\x1b[?2004l");
}

pub fn trackPointer() Error!void {
    try writeAll("\x1b[?1000h\x1b[?1006h");
}

pub fn untrackPointer() Error!void {
    try writeAll("\x1b[?1000l\x1b[?1006l");
}

pub fn setTitle(title: []const u8) Error!void {
    try writeAll("\x1b]2;");
    try writeAll(title);
    try writeAll("\x07");
}

pub fn writeAll(bytes: []const u8) Error!void {
    if (is_windows) return writeAllWindows(bytes);
    var written: usize = 0;
    while (written < bytes.len) {
        const n = std.c.write(posix.STDOUT_FILENO, bytes[written..].ptr, bytes.len - written);
        if (n < 0) return error.WriteFailed;
        if (n == 0) return error.WriteFailed;
        written += @intCast(n);
    }
}

pub fn size() Size {
    if (is_windows) return sizeWindows();
    var wsz: posix.winsize = .{ .row = 0, .col = 0, .xpixel = 0, .ypixel = 0 };
    const rc = posix.system.ioctl(posix.STDOUT_FILENO, posix.T.IOCGWINSZ, @intFromPtr(&wsz));
    if (posix.errno(rc) != .SUCCESS or wsz.col == 0 or wsz.row == 0) {
        return .{ .columns = 80, .rows = 24 };
    }
    return .{ .columns = wsz.col, .rows = wsz.row };
}

pub fn readEvent(buf: []u8, filled: *usize) Error!?Event {
    if (filled.* == 0) {
        const n = readStdin(buf) catch return .lost;
        if (n == 0) return .lost;
        filled.* = n;
    }
    var used: usize = 0;
    const event = parseOne(buf[0..filled.*], &used) orelse {
        if (filled.* == 1 and buf[0] == 0x1b) {
            if (!waitReadable(50)) {
                filled.* = 0;
                return .{ .key = .{ .code = .esc } };
            }
        }
        if (filled.* == buf.len) {
            @memmove(buf[0 .. filled.* - 1], buf[1..filled.*]);
            filled.* -= 1;
        }
        const n = readStdin(buf[filled.*..]) catch return .lost;
        if (n == 0) return .lost;
        filled.* += n;
        return null;
    };
    const rest = filled.* - used;
    if (rest > 0) @memmove(buf[0..rest], buf[used..filled.*]);
    filled.* = rest;
    return event;
}

fn readStdin(buf: []u8) Error!usize {
    if (is_windows) return readStdinWindows(buf);
    return posix.read(posix.STDIN_FILENO, buf) catch return error.Unexpected;
}

pub fn waitReadable(timeout_ms: i32) bool {
    if (is_windows) return waitReadableWindows(timeout_ms);
    var fds = [_]posix.pollfd{.{
        .fd = posix.STDIN_FILENO,
        .events = posix.POLL.IN,
        .revents = 0,
    }};
    const n = posix.poll(&fds, timeout_ms) catch return false;
    return n > 0 and (fds[0].revents & posix.POLL.IN) != 0;
}

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
    const utf8_len = @field(std.unicode, "utf8" ++ "ByteSequenceLength");
    const len = utf8_len(b) catch {
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
    if (input[2] == '<') return parseSgr(input, used);
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
        const mods = modifiersFromParam(if (param_i >= 2) params[1] else 0);
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
            'u' => blk: {
                if (params[0] == 27) {
                    break :blk .{ .key = .{ .code = .esc, .modifiers = mods } };
                }
                break :blk .{ .key = .{
                    .code = .{ .char = params[0] },
                    .modifiers = mods,
                } };
            },
            else => .{ .key = .{ .code = .unknown } },
        };
    }
    used.* = 0;
    return null;
}

fn parseSgr(input: []const u8, used: *usize) ?Event {
    var i: usize = 3;
    var nums: [3]u16 = .{ 0, 0, 0 };
    var ni: usize = 0;
    var current: u16 = 0;
    var saw = false;
    while (i < input.len) : (i += 1) {
        const c = input[i];
        if (c >= '0' and c <= '9') {
            saw = true;
            current = current *% 10 +% (c - '0');
            continue;
        }
        if (c == ';') {
            if (ni < nums.len) nums[ni] = current;
            ni += 1;
            current = 0;
            saw = false;
            continue;
        }
        used.* = i + 1;
        if (c != 'M' and c != 'm') return .{ .key = .{ .code = .unknown } };
        if (saw and ni < nums.len) nums[ni] = current;
        if (ni < 2) return .{ .key = .{ .code = .unknown } };
        return .{ .key = .{ .code = .{ .click = .{
            .button = nums[0],
            .column = if (ni >= 1) nums[1] else 0,
            .row = if (ni >= 2) nums[2] else 0,
            .release = c == 'm',
        } } } };
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

const WinApi = if (is_windows) struct {
    const HANDLE = std.os.windows.HANDLE;
    const BOOL = std.os.windows.BOOL;
    const DWORD = std.os.windows.DWORD;
    const INVALID_HANDLE_VALUE: HANDLE = @ptrFromInt(std.math.maxInt(usize));
    const STD_INPUT_HANDLE: DWORD = @bitCast(@as(i32, -10));
    const STD_OUTPUT_HANDLE: DWORD = @bitCast(@as(i32, -11));
    const WAIT_OBJECT_0: DWORD = 0;
    const INFINITE: DWORD = 0xffff_ffff;

    const SMALL_RECT = extern struct {
        Left: i16,
        Top: i16,
        Right: i16,
        Bottom: i16,
    };
    const CONSOLE_SCREEN_BUFFER_INFO = extern struct {
        dwSize: std.os.windows.COORD,
        dwCursorPosition: std.os.windows.COORD,
        wAttributes: u16,
        srWindow: SMALL_RECT,
        dwMaximumWindowSize: std.os.windows.COORD,
    };

    extern "kernel32" fn GetStdHandle(nStdHandle: DWORD) callconv(.winapi) HANDLE;
    extern "kernel32" fn GetConsoleMode(hConsoleHandle: HANDLE, lpMode: *DWORD) callconv(.winapi) BOOL;
    extern "kernel32" fn SetConsoleMode(hConsoleHandle: HANDLE, dwMode: DWORD) callconv(.winapi) BOOL;
    extern "kernel32" fn WriteFile(
        hFile: HANDLE,
        lpBuffer: [*]const u8,
        nNumberOfBytesToWrite: DWORD,
        lpNumberOfBytesWritten: ?*DWORD,
        lpOverlapped: ?*anyopaque,
    ) callconv(.winapi) BOOL;
    extern "kernel32" fn ReadFile(
        hFile: HANDLE,
        lpBuffer: [*]u8,
        nNumberOfBytesToRead: DWORD,
        lpNumberOfBytesRead: ?*DWORD,
        lpOverlapped: ?*anyopaque,
    ) callconv(.winapi) BOOL;
    extern "kernel32" fn WaitForSingleObject(hHandle: HANDLE, dwMilliseconds: DWORD) callconv(.winapi) DWORD;
    extern "kernel32" fn GetConsoleScreenBufferInfo(
        hConsoleOutput: HANDLE,
        lpConsoleScreenBufferInfo: *CONSOLE_SCREEN_BUFFER_INFO,
    ) callconv(.winapi) BOOL;

    fn stdHandle(kind: DWORD) Error!HANDLE {
        const h = GetStdHandle(kind);
        if (h == INVALID_HANDLE_VALUE) return error.NotATerminal;
        return h;
    }
} else struct {};

fn enableRawWindows() Error!void {
    if (comptime is_windows) {
        if (saved_win != null) return;
        const in_h = try WinApi.stdHandle(WinApi.STD_INPUT_HANDLE);
        const out_h = try WinApi.stdHandle(WinApi.STD_OUTPUT_HANDLE);
        var in_mode: u32 = 0;
        var out_mode: u32 = 0;
        if (!WinApi.GetConsoleMode(in_h, &in_mode).toBool()) return error.NotATerminal;
        if (!WinApi.GetConsoleMode(out_h, &out_mode).toBool()) return error.NotATerminal;
        saved_win = .{ .in_mode = in_mode, .out_mode = out_mode };
        const new_in = (in_mode & ~(ENABLE_ECHO_INPUT | ENABLE_LINE_INPUT | ENABLE_PROCESSED_INPUT)) |
            ENABLE_VIRTUAL_TERMINAL_INPUT;
        const new_out = out_mode | ENABLE_VIRTUAL_TERMINAL_PROCESSING;
        if (!WinApi.SetConsoleMode(in_h, new_in).toBool()) return error.NotATerminal;
        if (!WinApi.SetConsoleMode(out_h, new_out).toBool()) return error.NotATerminal;
    }
}

fn disableRawWindows() Error!void {
    if (comptime is_windows) {
        const previous = saved_win orelse return;
        if (WinApi.stdHandle(WinApi.STD_INPUT_HANDLE)) |in_h| {
            _ = WinApi.SetConsoleMode(in_h, previous.in_mode);
        } else |_| {}
        if (WinApi.stdHandle(WinApi.STD_OUTPUT_HANDLE)) |out_h| {
            _ = WinApi.SetConsoleMode(out_h, previous.out_mode);
        } else |_| {}
        saved_win = null;
    }
}

fn writeAllWindows(bytes: []const u8) Error!void {
    if (comptime is_windows) {
        const out_h = WinApi.stdHandle(WinApi.STD_OUTPUT_HANDLE) catch return error.WriteFailed;
        var written: u32 = 0;
        var offset: usize = 0;
        while (offset < bytes.len) {
            const chunk: u32 = @intCast(@min(bytes.len - offset, std.math.maxInt(u32)));
            if (!WinApi.WriteFile(out_h, bytes[offset..].ptr, chunk, &written, null).toBool()) {
                return error.WriteFailed;
            }
            if (written == 0) return error.WriteFailed;
            offset += written;
        }
    }
}

fn readStdinWindows(buf: []u8) Error!usize {
    if (comptime is_windows) {
        const in_h = WinApi.stdHandle(WinApi.STD_INPUT_HANDLE) catch return error.Unexpected;
        var read_n: u32 = 0;
        const cap: u32 = @intCast(@min(buf.len, std.math.maxInt(u32)));
        if (!WinApi.ReadFile(in_h, buf.ptr, cap, &read_n, null).toBool()) return error.Unexpected;
        return read_n;
    }
    return 0;
}

fn waitReadableWindows(timeout_ms: i32) bool {
    if (comptime is_windows) {
        const in_h = WinApi.stdHandle(WinApi.STD_INPUT_HANDLE) catch return false;
        const ms: u32 = if (timeout_ms < 0) WinApi.INFINITE else @intCast(timeout_ms);
        return WinApi.WaitForSingleObject(in_h, ms) == WinApi.WAIT_OBJECT_0;
    }
    return false;
}

fn sizeWindows() Size {
    if (comptime is_windows) {
        const out_h = WinApi.stdHandle(WinApi.STD_OUTPUT_HANDLE) catch {
            return .{ .columns = 80, .rows = 24 };
        };
        var info: WinApi.CONSOLE_SCREEN_BUFFER_INFO = undefined;
        if (!WinApi.GetConsoleScreenBufferInfo(out_h, &info).toBool()) {
            return .{ .columns = 80, .rows = 24 };
        }
        const cols = info.srWindow.Right - info.srWindow.Left + 1;
        const rows = info.srWindow.Bottom - info.srWindow.Top + 1;
        if (cols <= 0 or rows <= 0) return .{ .columns = 80, .rows = 24 };
        return .{ .columns = @intCast(cols), .rows = @intCast(rows) };
    }
    return .{ .columns = 80, .rows = 24 };
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

test "parser reads an sgr click" {
    var used: usize = 0;
    const press = parseOne("\x1b[<0;5;6M", &used).?;
    try std.testing.expectEqual(@as(usize, 9), used);
    try std.testing.expectEqual(@as(u16, 5), press.key.code.click.column);
    try std.testing.expectEqual(@as(u16, 6), press.key.code.click.row);
    try std.testing.expect(!press.key.code.click.release);
    const partial = parseOne("\x1b[<0;5", &used);
    try std.testing.expect(partial == null);
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

test "lone ESC is incomplete until forced" {
    var used: usize = 0;
    try std.testing.expect(parseOne("\x1b", &used) == null);
    try std.testing.expectEqual(@as(usize, 0), used);
}

test "kitty CSI u Escape is Code.esc" {
    var used: usize = 0;
    const key = parseOne("\x1b[27u", &used).?;
    try std.testing.expectEqual(Code.esc, key.key.code);
    try std.testing.expectEqual(@as(usize, 5), used);
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
