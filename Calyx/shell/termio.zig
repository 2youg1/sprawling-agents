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
var resize_armed: bool = false;
var pending_unit: u16 = 0;
var sgr_drop_until: i64 = 0;
var winch_read: if (is_windows) void else posix.fd_t = if (is_windows) {} else -1;
var winch_write: if (is_windows) void else posix.fd_t = if (is_windows) {} else -1;

const WinModes = struct {
    in_mode: u32,
    out_mode: u32,
};

const ENABLE_PROCESSED_INPUT: u32 = 0x0001;
const ENABLE_LINE_INPUT: u32 = 0x0002;
const ENABLE_ECHO_INPUT: u32 = 0x0004;
const ENABLE_WINDOW_INPUT: u32 = 0x0008;
const ENABLE_QUICK_EDIT_MODE: u32 = 0x0040;
const ENABLE_EXTENDED_FLAGS: u32 = 0x0080;
const ENABLE_VIRTUAL_TERMINAL_INPUT: u32 = 0x0200;
const ENABLE_VIRTUAL_TERMINAL_PROCESSING: u32 = 0x0004;
const ENABLE_MOUSE_INPUT: u32 = 0x0010;

const InputBackend = enum { virtual_terminal, windows_console };

const InputProtocols = struct {
    kitty_keyboard: bool,
    bracketed_paste: bool,
    sgr_mouse: bool,
};

fn inputProtocols(backend: InputBackend) InputProtocols {
    return switch (backend) {
        .virtual_terminal => .{ .kitty_keyboard = true, .bracketed_paste = true, .sgr_mouse = true },
        .windows_console => .{ .kitty_keyboard = false, .bracketed_paste = false, .sgr_mouse = false },
    };
}

fn activeInputBackend() InputBackend {
    return if (is_windows) .windows_console else .virtual_terminal;
}

fn windowsRawInputMode(mode: u32) u32 {
    return (mode & ~(ENABLE_ECHO_INPUT | ENABLE_LINE_INPUT | ENABLE_PROCESSED_INPUT | ENABLE_QUICK_EDIT_MODE | ENABLE_VIRTUAL_TERMINAL_INPUT)) |
        ENABLE_WINDOW_INPUT | ENABLE_MOUSE_INPUT | ENABLE_EXTENDED_FLAGS;
}

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
    installResize();
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
    const protocols = inputProtocols(activeInputBackend());
    if (protocols.kitty_keyboard) try writeAll("\x1b[>1u");
    if (protocols.bracketed_paste) try writeAll("\x1b[?2004h");
}

pub fn plainKeyboard() Error!void {
    const protocols = inputProtocols(activeInputBackend());
    if (protocols.kitty_keyboard) try writeAll("\x1b[<u");
    if (protocols.bracketed_paste) try writeAll("\x1b[?2004l");
}

pub fn trackPointer() Error!void {
    if (inputProtocols(activeInputBackend()).sgr_mouse) try writeAll("\x1b[?1000h\x1b[?1006h");
}

pub fn untrackPointer() Error!void {
    if (inputProtocols(activeInputBackend()).sgr_mouse) try writeAll("\x1b[?1000l\x1b[?1006l");
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

pub fn resizeArmed() bool {
    return resize_armed;
}

pub fn readEvent(buf: []u8, filled: *usize) Error!?Event {
    if (is_windows and resize_armed) return readEventWindows();
    if (drainWinch()) return .{ .resize = size() };
    if (filled.* == 0) {
        const n = readStdin(buf) catch return .lost;
        if (n == 0) return .lost;
        filled.* = n;
    }
    dropPendingSgr(buf, filled);
    if (filled.* == 0) return null;
    return finishParse(buf, filled);
}

fn finishParse(buf: []u8, filled: *usize) Error!?Event {
    if (takeParsed(buf, filled)) |event| return event;
    const short = buf[0] != 0x1b and filled.* < 4;
    switch (try continueRead(buf, filled)) {
        .timeout => {
            if (short) {
                filled.* = 0;
                return null;
            }
            const lone = filled.* == 1 and buf[0] == 0x1b;
            if (!lone and std.mem.startsWith(u8, buf[0..filled.*], "\x1b[<")) {
                sgr_drop_until = nowMs() + 50;
            }
            filled.* = 0;
            if (lone) return .{ .key = .{ .code = .esc } };
            return null;
        },
        .resize => return .{ .resize = size() },
        .grew => {},
    }
    if (filled.* == 0) return null;
    if (takeParsed(buf, filled)) |event| return event;
    return null;
}

const ReadStep = enum { timeout, resize, grew };

fn continueRead(buf: []u8, filled: *usize) Error!ReadStep {
    switch (wake(50)) {
        .none => return .timeout,
        .resize => {
            _ = drainWinch();
            return .resize;
        },
        .stdin => {
            if (filled.* == buf.len) {
                @memmove(buf[0 .. filled.* - 1], buf[1..filled.*]);
                filled.* -= 1;
            }
            const n = try readStdin(buf[filled.*..]);
            if (n == 0) return error.Unexpected;
            filled.* += n;
            dropPendingSgr(buf, filled);
            return .grew;
        },
    }
}

fn takeParsed(buf: []u8, filled: *usize) ?Event {
    var used: usize = 0;
    const event = parseOne(buf[0..filled.*], &used) orelse return null;
    const rest = filled.* - used;
    if (rest > 0) @memmove(buf[0..rest], buf[used..filled.*]);
    filled.* = rest;
    return event;
}

fn dropPendingSgr(buf: []u8, filled: *usize) void {
    if (sgr_drop_until == 0) return;
    if (nowMs() > sgr_drop_until) {
        sgr_drop_until = 0;
        return;
    }
    var i: usize = 0;
    var closed = false;
    while (i < filled.*) {
        const b = buf[i];
        if ((b >= '0' and b <= '9') or b == ';') {
            i += 1;
            continue;
        }
        if (b == 'M' or b == 'm') {
            i += 1;
            closed = true;
        }
        break;
    }
    if (i == 0) return;
    const rest = filled.* - i;
    if (rest > 0) @memmove(buf[0..rest], buf[i..filled.*]);
    filled.* = rest;
    if (closed) sgr_drop_until = 0;
}

fn partialEscape(buf: []const u8) enum { esc, discard } {
    if (buf.len == 1 and buf[0] == 0x1b) return .esc;
    return .discard;
}

fn installResize() void {
    if (is_windows or resize_armed) return;
    var fds: [2]posix.fd_t = undefined;
    if (std.c.pipe(&fds) != 0) return;
    setNonblock(fds[0]);
    setNonblock(fds[1]);
    winch_read = fds[0];
    winch_write = fds[1];
    const mask = posix.sigemptyset();
    const act: posix.Sigaction = .{
        .handler = .{ .handler = wakeWinch },
        .mask = mask,
        .flags = 0,
    };
    posix.sigaction(.WINCH, &act, null);
    resize_armed = true;
}

fn setNonblock(fd: posix.fd_t) void {
    if (builtin.os.tag == .windows) return;
    const get: c_int = 3;
    const set: c_int = 4;
    const nonblock: c_uint = if (builtin.os.tag == .macos) 0x0004 else 0o4000;
    const cur = std.c.fcntl(fd, get, @as(c_uint, 0));
    if (cur < 0) return;
    _ = std.c.fcntl(fd, set, @as(c_uint, @intCast(cur)) | nonblock);
}

fn wakeWinch(_: posix.SIG) callconv(.c) void {
    if (is_windows or winch_write < 0) return;
    var byte = [_]u8{1};
    _ = std.c.write(winch_write, &byte, 1);
}

fn drainWinch() bool {
    if (is_windows or winch_read < 0) return false;
    var junk: [32]u8 = undefined;
    var saw = false;
    while (true) {
        const n = std.c.read(winch_read, &junk, junk.len);
        if (n <= 0) break;
        saw = true;
        if (n < junk.len) break;
    }
    return saw;
}

fn readEventWindows() Error!?Event {
    if (comptime !is_windows) return null;
    const KEY_EVENT: u16 = 1;
    const MOUSE_EVENT: u16 = 2;
    const WINDOW_BUFFER_SIZE_EVENT: u16 = 4;
    const KeyRecord = extern struct {
        down: i32,
        repeat: u16,
        virtual_key: u16,
        scan: u16,
        unicode: u16,
        control: u32,
    };
    const MouseRecord = extern struct {
        x: i16,
        y: i16,
        button: u32,
        control: u32,
        flags: u32,
    };
    const Record = extern struct {
        kind: u16,
        _pad: u16 = 0,
        event: extern union {
            key: KeyRecord,
            mouse: MouseRecord,
            size: extern struct { x: i16, y: i16 },
        },
    };
    const ReadConsoleInputW = struct {
        extern "kernel32" fn ReadConsoleInputW(handle: WinApi.HANDLE, buffer: *Record, length: u32, read: *u32) callconv(.winapi) WinApi.BOOL;
    }.ReadConsoleInputW;
    const in_h = WinApi.stdHandle(WinApi.STD_INPUT_HANDLE) catch return .lost;
    var record: Record = undefined;
    var count: u32 = 0;
    if (!ReadConsoleInputW(in_h, &record, 1, &count).toBool() or count == 0) return null;
    switch (record.kind) {
        WINDOW_BUFFER_SIZE_EVENT => return .{ .resize = size() },
        MOUSE_EVENT => {
            if (record.event.mouse.flags != 0 or record.event.mouse.button & 1 == 0) return null;
            return .{ .key = .{ .code = .{ .click = .{
                .column = @intCast(record.event.mouse.x + 1),
                .row = @intCast(record.event.mouse.y + 1),
                .button = 0,
                .release = false,
            } } } };
        },
        KEY_EVENT => {
            if (record.event.key.down == 0) return null;
            return windowsKeyEvent(
                record.event.key.virtual_key,
                record.event.key.unicode,
                record.event.key.control,
            );
        },
        else => return null,
    }
}

fn readStdin(buf: []u8) Error!usize {
    if (is_windows) return readStdinWindows(buf);
    return posix.read(posix.STDIN_FILENO, buf) catch return error.Unexpected;
}

const Wake = enum { stdin, resize, none };

fn wake(timeout_ms: i32) Wake {
    if (is_windows) return if (waitReadableWindows(timeout_ms)) .stdin else .none;
    var fds = [_]posix.pollfd{
        .{ .fd = posix.STDIN_FILENO, .events = posix.POLL.IN, .revents = 0 },
        .{ .fd = winch_read, .events = posix.POLL.IN, .revents = 0 },
    };
    const nfd: usize = if (winch_read >= 0) 2 else 1;
    const n = posix.poll(fds[0..nfd], timeout_ms) catch return .none;
    if (n <= 0) return .none;
    if (fds[0].revents & posix.POLL.IN != 0) return .stdin;
    return .resize;
}

pub fn waitReadable(timeout_ms: i32) bool {
    if (is_windows) return waitReadableWindows(timeout_ms);
    return wake(timeout_ms) != .none;
}

fn windowsKey(unit: u16, mods: Modifiers) ?Event {
    if (utf16Unit(&pending_unit, unit)) |cp| {
        if (cp < 32) {
            const letter = controlLetter(unit) orelse return .{ .key = .{ .code = .unknown, .modifiers = mods } };
            var marked = mods;
            marked.control = true;
            return .{ .key = .{ .code = .{ .char = letter }, .modifiers = marked } };
        }
        return .{ .key = .{ .code = .{ .char = cp }, .modifiers = mods } };
    }
    return null;
}

fn windowsKeyEvent(virtual_key: u16, unit: u16, control: u32) ?Event {
    const mods: Modifiers = .{
        .shift = control & 0x0010 != 0,
        .alt = control & 0x0003 != 0,
        .control = control & 0x000c != 0,
    };
    const code: Code = switch (virtual_key) {
        0x0d => .enter,
        0x1b => .esc,
        0x08 => .backspace,
        0x09 => if (mods.shift) .back_tab else .tab,
        0x2e => .delete,
        0x24 => .home,
        0x23 => .end,
        0x25 => .left,
        0x26 => .up,
        0x27 => .right,
        0x28 => .down,
        else => return windowsKey(unit, mods),
    };
    return .{ .key = .{ .code = code, .modifiers = mods } };
}

fn utf16Unit(pending: *u16, unit: u16) ?u21 {
    if (unit >= 0xD800 and unit <= 0xDBFF) {
        pending.* = unit;
        return null;
    }
    if (unit >= 0xDC00 and unit <= 0xDFFF) {
        if (pending.* < 0xD800 or pending.* > 0xDBFF) {
            pending.* = 0;
            return null;
        }
        const high = pending.*;
        pending.* = 0;
        return 0x10000 + (@as(u21, high - 0xD800) << 10) + @as(u21, unit - 0xDC00);
    }
    pending.* = 0;
    return unit;
}

fn controlLetter(unit: u16) ?u21 {
    if (unit == 0 or unit >= 32) return null;
    return @as(u21, unit) | 0x60;
}

fn nowMs() i64 {
    if (is_windows) return 0;
    var ts: posix.timespec = undefined;
    if (posix.errno(posix.system.clock_gettime(posix.CLOCK.REALTIME, &ts)) != .SUCCESS) return 0;
    return @as(i64, ts.sec) * 1000 + @divTrunc(@as(i64, ts.nsec), std.time.ns_per_ms);
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
        const new_in = windowsRawInputMode(in_mode);
        const new_out = out_mode | ENABLE_VIRTUAL_TERMINAL_PROCESSING;
        if (!WinApi.SetConsoleMode(in_h, new_in).toBool()) return error.NotATerminal;
        if (!WinApi.SetConsoleMode(out_h, new_out).toBool()) return error.NotATerminal;
        resize_armed = true;
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

test "a surrogate pair is one scalar and a control unit keeps its letter" {
    var pending: u16 = 0;
    try std.testing.expect(utf16Unit(&pending, 0xD83D) == null);
    try std.testing.expectEqual(@as(u21, 0x1F600), utf16Unit(&pending, 0xDE00).?);
    pending = 0;
    try std.testing.expect(utf16Unit(&pending, 0xDE00) == null);
    try std.testing.expectEqual(@as(?u21, 'o'), controlLetter(0x0f));
}

test "Windows key records preserve navigation and committed Unicode" {
    const pending = pending_unit;
    pending_unit = 0;
    defer pending_unit = pending;
    const up = windowsKeyEvent(0x26, 0, 0).?;
    try std.testing.expectEqual(Code.up, up.key.code);
    const letter = windowsKeyEvent(0x41, 'a', 0).?;
    try std.testing.expectEqual(Code{ .char = 'a' }, letter.key.code);
    _ = windowsKeyEvent(0xe7, 0xd83d, 0);
    const emoji = windowsKeyEvent(0xe7, 0xde00, 0).?;
    try std.testing.expectEqual(Code{ .char = 0x1f600 }, emoji.key.code);
}

test "seeded Windows keyboard record fuzz emits only Unicode scalars" {
    const pending = pending_unit;
    pending_unit = 0;
    defer pending_unit = pending;
    var prng = std.Random.DefaultPrng.init(0x7110_0b1e);
    const random = prng.random();
    var round: usize = 0;
    while (round < 8192) : (round += 1) {
        const pair = round % 64;
        const virtual_key: u16 = if (pair < 2) 0xe7 else random.int(u16);
        const unit: u16 = if (pair == 0) 0xd83d else if (pair == 1) 0xde00 else random.int(u16);
        if (windowsKeyEvent(virtual_key, unit, random.int(u32))) |event| {
            switch (event) {
                .key => |key| switch (key.code) {
                    .char => |scalar| try std.testing.expect(scalar < 0xd800 or scalar > 0xdfff),
                    else => {},
                },
                else => {},
            }
        }
    }
}

test "Windows input records do not negotiate VT key, paste, or pointer streams" {
    try std.testing.expectEqual(
        InputProtocols{ .kitty_keyboard = false, .bracketed_paste = false, .sgr_mouse = false },
        inputProtocols(.windows_console),
    );
}

test "Windows raw input mode reads native window and mouse records" {
    const original = ENABLE_ECHO_INPUT | ENABLE_LINE_INPUT | ENABLE_PROCESSED_INPUT | ENABLE_QUICK_EDIT_MODE | 0x0020;
    const mode = windowsRawInputMode(original);
    try std.testing.expectEqual(0x0020 | ENABLE_WINDOW_INPUT | ENABLE_MOUSE_INPUT | ENABLE_EXTENDED_FLAGS, mode);
    try std.testing.expectEqual(@as(u32, 0), mode & ENABLE_QUICK_EDIT_MODE);
}

test "a discarded sgr tail stops at its terminator" {
    sgr_drop_until = nowMs() + 1000;
    defer sgr_drop_until = 0;
    var buf = [_]u8{ ';', '6', 'M', 'a' };
    var filled: usize = buf.len;
    dropPendingSgr(&buf, &filled);
    try std.testing.expectEqual(@as(usize, 1), filled);
    try std.testing.expectEqual(@as(u8, 'a'), buf[0]);
    try std.testing.expectEqual(@as(i64, 0), sgr_drop_until);
}

test "a partial escape becomes esc only when it is a lone escape" {
    try std.testing.expect(partialEscape("\x1b") == .esc);
    try std.testing.expect(partialEscape("\x1b[<0;5") == .discard);
    try std.testing.expect(partialEscape("\x1b[") == .discard);
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
