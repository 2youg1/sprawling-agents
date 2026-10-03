// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// The Win32 call groups that have no admitted safe Rust interface,
// each as one whole operation behind a `(ptr, len)` boundary
// (`crates/desktop/Spec.lean` sections 8-12 and D12).
//
// What crosses the boundary is Rust's memory, lent for one call: a
// buffer and its length, an out-parameter for a count, an out-parameter
// for the Win32 error code. Nothing crosses the other way: no handle, no
// device context, no global block outlives the call that took it, so
// every resource below is released inside the export that acquired it,
// on every path, by the `defer` written beside the acquisition. Each
// export answers with a `Step` (`step.zig`): `Finished`, or the part of
// the operation that stopped. Rust turns a step and a code into a
// refusal; this file owns no wording, no scope and no policy.

const std = @import("std");
const windows = std.os.windows;
const boundary = @import("boundary.zig");
const Step = @import("step.zig").Step;

// A safety check that fails here traps rather than unwinding into Rust,
// which cannot catch it, and rather than linking Zig's stack-trace
// printer into a binary that never shows a Zig stack.
pub const panic = std.debug.no_panic;

const BOOL = windows.BOOL;
const HWND = windows.HWND;
const HANDLE = windows.HANDLE;
const HDC = *opaque {};
const HGDIOBJ = *opaque {};
const HRESULT = i32;
const LPARAM = windows.LPARAM;

const BITMAPINFOHEADER = extern struct {
    biSize: u32,
    biWidth: i32,
    biHeight: i32,
    biPlanes: u16,
    biBitCount: u16,
    biCompression: u32,
    biSizeImage: u32 = 0,
    biXPelsPerMeter: i32 = 0,
    biYPelsPerMeter: i32 = 0,
    biClrUsed: u32 = 0,
    biClrImportant: u32 = 0,
};

const BITMAPINFO = extern struct {
    bmiHeader: BITMAPINFOHEADER,
    bmiColors: [1]u32 = .{0},
};

// The SDK's values, each written once, here: no Rust code names them.
const BI_RGB: u32 = 0;
const DIB_RGB_COLORS: u32 = 0;
const PW_RENDERFULLCONTENT: u32 = 0x2;
const CF_UNICODETEXT: u32 = 13;
const GMEM_MOVEABLE: u32 = 0x2;
const HWND_MESSAGE: isize = -3;
const STATIC: [*:0]const u16 = &[_:0]u16{ 'S', 'T', 'A', 'T', 'I', 'C' };

const EnumProc = *const fn (HWND, LPARAM) callconv(.winapi) BOOL;

extern "user32" fn EnumWindows(proc: EnumProc, param: LPARAM) callconv(.winapi) BOOL;
extern "user32" fn GetDC(window: ?HWND) callconv(.winapi) ?HDC;
extern "user32" fn ReleaseDC(window: ?HWND, context: HDC) callconv(.winapi) i32;
extern "user32" fn PrintWindow(window: HWND, context: HDC, flags: u32) callconv(.winapi) BOOL;
extern "user32" fn CreateWindowExW(
    ex_style: u32,
    class: ?[*:0]const u16,
    name: ?[*:0]const u16,
    style: u32,
    x: i32,
    y: i32,
    width: i32,
    height: i32,
    parent: ?HWND,
    menu: ?*anyopaque,
    instance: ?*anyopaque,
    param: ?*anyopaque,
) callconv(.winapi) ?HWND;
extern "user32" fn DestroyWindow(window: HWND) callconv(.winapi) BOOL;
extern "user32" fn OpenClipboard(owner: ?HWND) callconv(.winapi) BOOL;
extern "user32" fn CloseClipboard() callconv(.winapi) BOOL;
extern "user32" fn EmptyClipboard() callconv(.winapi) BOOL;
extern "user32" fn IsClipboardFormatAvailable(format: u32) callconv(.winapi) BOOL;
extern "user32" fn GetClipboardData(format: u32) callconv(.winapi) ?HANDLE;
extern "user32" fn SetClipboardData(format: u32, block: ?HANDLE) callconv(.winapi) ?HANDLE;
extern "gdi32" fn CreateCompatibleDC(context: ?HDC) callconv(.winapi) ?HDC;
extern "gdi32" fn CreateCompatibleBitmap(context: HDC, width: i32, height: i32) callconv(.winapi) ?HGDIOBJ;
extern "gdi32" fn SelectObject(context: HDC, object: HGDIOBJ) callconv(.winapi) ?HGDIOBJ;
extern "gdi32" fn DeleteObject(object: HGDIOBJ) callconv(.winapi) BOOL;
extern "gdi32" fn DeleteDC(context: HDC) callconv(.winapi) BOOL;
extern "gdi32" fn GetDIBits(
    context: HDC,
    bitmap: HGDIOBJ,
    start: u32,
    lines: u32,
    bits: ?*anyopaque,
    info: *BITMAPINFO,
    usage: u32,
) callconv(.winapi) i32;
extern "kernel32" fn GlobalAlloc(flags: u32, bytes: usize) callconv(.winapi) ?HANDLE;
extern "kernel32" fn GlobalFree(block: ?HANDLE) callconv(.winapi) ?HANDLE;
extern "kernel32" fn GlobalLock(block: ?HANDLE) callconv(.winapi) ?*anyopaque;
extern "kernel32" fn GlobalUnlock(block: HANDLE) callconv(.winapi) BOOL;
extern "kernel32" fn GlobalSize(block: HANDLE) callconv(.winapi) usize;
extern "shcore" fn SetProcessDpiAwareness(awareness: u32) callconv(.winapi) HRESULT;
extern "shcore" fn GetProcessDpiAwareness(process: ?HANDLE, awareness: *u32) callconv(.winapi) HRESULT;

/// The calling thread's last error, read immediately after the call that
/// failed and before any other call can overwrite it.
fn lastError() u32 {
    return @intFromEnum(windows.GetLastError());
}

/// One operation's end: the step it stopped at and the code it read.
pub const Ended = struct {
    step: Step,
    code: u32 = 0,

    pub const finished: Ended = .{ .step = .Finished };

    pub fn failed(step: Step) Ended {
        return .{ .step = step, .code = lastError() };
    }

    pub fn answer(self: Ended, code: *u32) u32 {
        code.* = self.code;
        return @intFromEnum(self.step);
    }
};

// ---------------------------------------------------------------- windows

const Handles = boundary.Kept(?HWND);

fn collect(handle: HWND, param: LPARAM) callconv(.winapi) BOOL {
    const kept: *Handles = @ptrFromInt(@as(usize, @bitCast(param)));
    kept.keep(handle);
    return .TRUE;
}

/// Every top-level window `EnumWindows` reports, written into `into` up
/// to `capacity` and counted in full into `found`. `NoRoom` asks for a
/// longer buffer; nothing past `capacity` is written.
export fn sprawling_desktop_windows(into: [*]?HWND, capacity: usize, found: *usize, code: *u32) u32 {
    var kept: Handles = .{ .into = into[0..capacity] };
    const walked = EnumWindows(collect, @bitCast(@intFromPtr(&kept)));
    found.* = kept.count;
    if (walked == .FALSE) return Ended.failed(.Listing).answer(code);
    return (Ended{ .step = kept.step() }).answer(code);
}

// ---------------------------------------------------------------- capture

/// One window's pixels as top-down 32-bit BGRA, written into exactly
/// `len` bytes. The bitmap is unselected before it is read back, the
/// window's context goes back with the window it came from, and every
/// object is released on every path (`crates/desktop/Spec.lean` section 10, item 8).
export fn sprawling_desktop_capture(window: ?HWND, width: i32, height: i32, into: [*]u8, len: usize, code: *u32) u32 {
    return capture(window, width, height, into[0..len]).answer(code);
}

fn capture(window: ?HWND, width: i32, height: i32, into: []u8) Ended {
    // `GetDC` of no window is the context of the whole screen.
    const named = window orelse return .{ .step = .NoWindow };
    const bytes = boundary.bitmapBytes(width, height) orelse return .{ .step = .Measuring };
    if (bytes != into.len) return .{ .step = .Measuring };
    const screen = GetDC(named) orelse return Ended.failed(.Context);
    defer _ = ReleaseDC(named, screen);
    const memory = CreateCompatibleDC(screen) orelse return Ended.failed(.Bitmap);
    defer _ = DeleteDC(memory);
    const bitmap = CreateCompatibleBitmap(screen, width, height) orelse return Ended.failed(.Bitmap);
    defer _ = DeleteObject(bitmap);
    const drawn = draw(named, memory, bitmap);
    if (drawn.step != .Finished) return drawn;
    return readBack(memory, bitmap, width, height, into);
}

/// The window drawn into the bitmap while it is selected, and the old
/// object selected back before this returns, whichever way it returns.
fn draw(window: HWND, memory: HDC, bitmap: HGDIOBJ) Ended {
    const previous = SelectObject(memory, bitmap) orelse return Ended.failed(.Selecting);
    defer _ = SelectObject(memory, previous);
    // The reason is read before the deferred call can overwrite it.
    if (PrintWindow(window, memory, PW_RENDERFULLCONTENT) == .FALSE) return Ended.failed(.Drawing);
    return .finished;
}

fn readBack(memory: HDC, bitmap: HGDIOBJ, width: i32, height: i32, into: []u8) Ended {
    var info: BITMAPINFO = .{
        .bmiHeader = .{
            .biSize = @sizeOf(BITMAPINFOHEADER),
            .biWidth = width,
            // Negative is how GDI is told to hand back the top row first.
            .biHeight = -height,
            .biPlanes = 1,
            .biBitCount = 32,
            .biCompression = BI_RGB,
        },
    };
    const lines: u32 = @intCast(height);
    const rows = GetDIBits(memory, bitmap, 0, lines, into.ptr, &info, DIB_RGB_COLORS);
    if (rows == 0) return Ended.failed(.Reading);
    if (rows != height) return .{ .step = .ShortRows };
    return .finished;
}

// -------------------------------------------------------------- clipboard

/// The clipboard, open under a message-only window of this process's
/// own for as long as `open` holds it (`crates/desktop/Spec.lean` D8).
const Held = struct {
    owner: HWND,

    fn open() union(enum) { held: Held, ended: Ended } {
        const owner = CreateWindowExW(0, STATIC, null, 0, 0, 0, 0, 0, @ptrFromInt(@as(usize, @bitCast(HWND_MESSAGE))), null, null, null) orelse
            return .{ .ended = Ended.failed(.Owner) };
        if (OpenClipboard(owner) == .FALSE) {
            const ended = Ended.failed(.Opening);
            _ = DestroyWindow(owner);
            return .{ .ended = ended };
        }
        return .{ .held = .{ .owner = owner } };
    }

    /// Closes the clipboard first, then destroys the owner it was open
    /// under, which is the order section 12.8 fixes.
    fn close(self: Held) void {
        _ = CloseClipboard();
        _ = DestroyWindow(self.owner);
    }
};

/// The clipboard's text, copied into `into` up to its first zero unit
/// and never past the block `GlobalSize` measures. `Absent` is a
/// clipboard holding no text; `NoRoom` asks for `found` units of room.
export fn sprawling_desktop_clipboard_read(into: [*]u16, capacity: usize, found: *usize, code: *u32) u32 {
    return clipboardRead(into[0..capacity], found).answer(code);
}

fn clipboardRead(into: []u16, found: *usize) Ended {
    const held = switch (Held.open()) {
        .held => |held| held,
        .ended => |ended| return ended,
    };
    defer held.close();
    if (IsClipboardFormatAvailable(CF_UNICODETEXT) == .FALSE) return .{ .step = .Absent };
    const block = GetClipboardData(CF_UNICODETEXT) orelse return Ended.failed(.Fetching);
    return lockedCopy(block, into, found);
}

/// The text behind one global block. A block that will not lock is a
/// refusal, never an empty clipboard.
fn lockedCopy(block: ?HANDLE, into: []u16, found: *usize) Ended {
    const start = GlobalLock(block) orelse return Ended.failed(.Locking);
    // `GlobalLock` answered, so `block` is not null.
    const locked = block.?;
    defer _ = GlobalUnlock(locked);
    const units = GlobalSize(locked) / @sizeOf(u16);
    if (units == 0) return .{ .step = .EmptyBlock };
    const text: [*]const u16 = @ptrCast(@alignCast(start));
    const copied = boundary.textCopy(text[0..units], into);
    found.* = copied.count;
    return .{ .step = copied.step };
}

/// `len` units of text put on the clipboard. The block is allocated and
/// filled before the clipboard is emptied, and freed here unless the
/// clipboard took it. `Handing` means the clipboard was emptied and is
/// empty now.
export fn sprawling_desktop_clipboard_write(text: [*]const u16, len: usize, code: *u32) u32 {
    return clipboardWrite(text[0..len]).answer(code);
}

fn clipboardWrite(text: []const u16) Ended {
    const bytes = boundary.blockBytes(text.len) orelse return .{ .step = .Measuring };
    const block = GlobalAlloc(GMEM_MOVEABLE, bytes) orelse return Ended.failed(.Allocating);
    const handed = handOver(block, text);
    if (handed.step != .Finished) _ = GlobalFree(block);
    return handed;
}

fn handOver(block: HANDLE, text: []const u16) Ended {
    const filled = fill(block, text);
    if (filled.step != .Finished) return filled;
    const held = switch (Held.open()) {
        .held => |held| held,
        .ended => |ended| return ended,
    };
    defer held.close();
    if (EmptyClipboard() == .FALSE) return Ended.failed(.Emptying);
    if (SetClipboardData(CF_UNICODETEXT, block) == null) return Ended.failed(.Handing);
    return .finished;
}

fn fill(block: HANDLE, text: []const u16) Ended {
    const start = GlobalLock(block) orelse return Ended.failed(.Allocating);
    defer _ = GlobalUnlock(block);
    const units: [*]u16 = @ptrCast(@alignCast(start));
    return .{ .step = boundary.textFill(text, units[0 .. text.len + 1]) };
}

// -------------------------------------------------------------------- dpi

/// Declares the awareness Rust names, answering the call's HRESULT.
export fn sprawling_desktop_dpi_declare(awareness: u32) u32 {
    return @bitCast(SetProcessDpiAwareness(awareness));
}

/// This process's awareness, into `awareness`, answering the HRESULT.
export fn sprawling_desktop_dpi_awareness(awareness: *u32) u32 {
    return @bitCast(GetProcessDpiAwareness(null, awareness));
}

// --------------------------------------------------------------- boundary

// The rules of `boundary.zig`, callable on their own so the Rust
// reference and the fuzz target judge the very functions the operations
// above write with. They read and write only what they are lent.

export fn sprawling_desktop_keep(stream: [*]const usize, len: usize, into: [*]usize, capacity: usize, found: *usize) u32 {
    var kept: boundary.Kept(usize) = .{ .into = into[0..capacity] };
    for (stream[0..len]) |item| kept.keep(item);
    found.* = kept.count;
    return @intFromEnum(kept.step());
}

export fn sprawling_desktop_text_copy(block: [*]const u16, len: usize, into: [*]u16, capacity: usize, found: *usize) u32 {
    const copied = boundary.textCopy(block[0..len], into[0..capacity]);
    found.* = copied.count;
    return @intFromEnum(copied.step);
}

export fn sprawling_desktop_text_fill(units: [*]const u16, len: usize, block: [*]u16, block_len: usize) u32 {
    return @intFromEnum(boundary.textFill(units[0..len], block[0..block_len]));
}

export fn sprawling_desktop_bitmap_bytes(width: i32, height: i32, bytes: *usize) u32 {
    const measured = boundary.bitmapBytes(width, height) orelse return @intFromEnum(Step.Measuring);
    bytes.* = measured;
    return @intFromEnum(Step.Finished);
}

// The processor and job exports live in `cpu.zig`; naming it here is
// what makes this library emit them.
comptime {
    _ = @import("cpu.zig");
}

test {
    _ = boundary;
    _ = @import("cpu.zig");
}

test "a block that will not lock is a refusal, not an empty clipboard" {
    var into: [4]u16 = undefined;
    var found: usize = 0;
    const ended = lockedCopy(null, &into, &found);
    try std.testing.expectEqual(Step.Locking, ended.step);
}

test "a null window is refused before any context is taken" {
    var into: [16]u8 = undefined;
    try std.testing.expectEqual(Step.NoWindow, capture(null, 2, 2, &into).step);
}
