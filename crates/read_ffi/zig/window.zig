// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The leaf that opens one file and answers everything the judgement
//! and the window need from the handle it opened
//! (`crates/read_ffi/Spec.lean` D1, `crates/runtime/Spec.lean` D12).
//!
//! One `CreateFileW` and two queries on that handle answer what the
//! safe path spends four opens and two canonicalisations on: where the
//! file really is, how big it is, when it last changed, and the bytes of
//! the window. Opening first and then asking the handle where it lives
//! is also the stronger shape — there is no moment between a judgement
//! and an open in which a link can be swapped, because the judgement is
//! made about the file already open.
//!
//! The leaf decides no policy: it answers facts. Whether the final path
//! lies in the city, and what address it names there, stays in
//! `runtime::tools::chosen_path`, where the one admission lives.

const std = @import("std");

const HANDLE = *anyopaque;
const INVALID_HANDLE_VALUE: HANDLE = @ptrFromInt(std.math.maxInt(usize));

const GENERIC_READ: u32 = 0x8000_0000;
const FILE_SHARE_READ: u32 = 0x0000_0001;
const FILE_SHARE_WRITE: u32 = 0x0000_0002;
const OPEN_EXISTING: u32 = 3;
const FILE_ATTRIBUTE_NORMAL: u32 = 0x0000_0080;
const FILE_ATTRIBUTE_DIRECTORY: u32 = 0x0000_0010;
/// Bypasses the page cache: the bytes an answer needs are read once and
/// are not worth the cache fill a buffered read pays for (D12).
const FILE_FLAG_NO_BUFFERING: u32 = 0x2000_0000;
const FILE_FLAG_SEQUENTIAL_SCAN: u32 = 0x0800_0000;
/// A directory opens only when this is asked for, and opening one is how
/// the caller learns the path names a directory rather than a file.
const FILE_FLAG_BACKUP_SEMANTICS: u32 = 0x0200_0000;
const FILE_BEGIN: u32 = 0;

const ERROR_FILE_NOT_FOUND: u32 = 2;
const ERROR_PATH_NOT_FOUND: u32 = 3;
const ERROR_ACCESS_DENIED: u32 = 5;
const ERROR_INVALID_PARAMETER: u32 = 87;
const ERROR_INSUFFICIENT_BUFFER: u32 = 122;

const FILETIME = extern struct {
    low: u32,
    high: u32,
};

/// `BY_HANDLE_FILE_INFORMATION`, in the order and size the system
/// writes it.
const FileInformation = extern struct {
    attributes: u32,
    creation: FILETIME,
    accessed: FILETIME,
    written: FILETIME,
    volume: u32,
    size_high: u32,
    size_low: u32,
    links: u32,
    index_high: u32,
    index_low: u32,
};

/// What the leaf answers with beside the bytes: the window's length,
/// the file's own facts, and where the final path was written.
pub const Drawn = extern struct {
    written: usize,
    size: u64,
    /// The file's last write time in nanoseconds since the Unix epoch,
    /// which is the value the cache compares.
    modified_nanos: u64,
    final_len: usize,
};

/// The leaf's answer. `ok` is the only one that carries bytes.
pub const Status = enum(u32) {
    ok = 0,
    missing = 1,
    directory = 2,
    unreadable = 3,
    read_failed = 4,
    path_too_long = 5,
};

extern "kernel32" fn CreateFileW(
    name: [*:0]const u16,
    access: u32,
    share: u32,
    security: ?*anyopaque,
    creation: u32,
    flags: u32,
    template: ?HANDLE,
) callconv(.winapi) HANDLE;

extern "kernel32" fn CloseHandle(handle: HANDLE) callconv(.winapi) i32;

extern "kernel32" fn GetFinalPathNameByHandleW(
    handle: HANDLE,
    into: [*]u16,
    capacity: u32,
    flags: u32,
) callconv(.winapi) u32;

extern "kernel32" fn GetFileInformationByHandle(
    handle: HANDLE,
    into: *FileInformation,
) callconv(.winapi) i32;

extern "kernel32" fn SetFilePointerEx(
    handle: HANDLE,
    distance: i64,
    moved: ?*i64,
    method: u32,
) callconv(.winapi) i32;

extern "kernel32" fn ReadFile(
    handle: HANDLE,
    into: [*]u8,
    want: u32,
    read: *u32,
    overlapped: ?*anyopaque,
) callconv(.winapi) i32;

extern "kernel32" fn GetLastError() callconv(.winapi) u32;

/// Opens `name`, reads the window from `start` into `into`, and answers
/// where the opened file really lives.
///
/// `start` and `into` are aligned to the volume's sector and `want` is a
/// whole number of sectors when the caller asks for a direct read;
/// `direct` false asks for the buffered fallback, which needs no
/// alignment. The caller gets `written` bytes at most.
export fn sprawling_read_window(
    name: [*:0]const u16,
    start: u64,
    want: u32,
    direct: u32,
    into: [*]u8,
    capacity: usize,
    final_into: [*]u16,
    final_capacity: u32,
    drawn: *Drawn,
) Status {
    drawn.* = .{ .written = 0, .size = 0, .modified_nanos = 0, .final_len = 0 };
    const flags = FILE_ATTRIBUTE_NORMAL | FILE_FLAG_SEQUENTIAL_SCAN |
        FILE_FLAG_BACKUP_SEMANTICS |
        (if (direct != 0) FILE_FLAG_NO_BUFFERING else 0);
    var handle = CreateFileW(
        name,
        GENERIC_READ,
        FILE_SHARE_READ | FILE_SHARE_WRITE,
        null,
        OPEN_EXISTING,
        flags,
        null,
    );
    if (handle == INVALID_HANDLE_VALUE and direct != 0) {
        // A volume whose sector is larger than the caller's alignment
        // refuses a direct read; the buffered one still answers.
        handle = CreateFileW(
            name,
            GENERIC_READ,
            FILE_SHARE_READ | FILE_SHARE_WRITE,
            null,
            OPEN_EXISTING,
            FILE_ATTRIBUTE_NORMAL | FILE_FLAG_SEQUENTIAL_SCAN | FILE_FLAG_BACKUP_SEMANTICS,
            null,
        );
    }
    if (handle == INVALID_HANDLE_VALUE) {
        return switch (GetLastError()) {
            ERROR_FILE_NOT_FOUND, ERROR_PATH_NOT_FOUND => .missing,
            ERROR_ACCESS_DENIED, ERROR_INVALID_PARAMETER => .unreadable,
            else => .unreadable,
        };
    }
    defer _ = CloseHandle(handle);

    var info: FileInformation = undefined;
    if (GetFileInformationByHandle(handle, &info) == 0) {
        return .unreadable;
    }
    const size = (@as(u64, info.size_high) << 32) | @as(u64, info.size_low);
    drawn.size = size;
    const stamp = (@as(u64, info.written.high) << 32) | @as(u64, info.written.low);
    // 100-nanosecond steps since 1601, moved to the Unix epoch.
    const epoch_delta: u64 = 116_444_736_000_000_000;
    drawn.modified_nanos = if (stamp > epoch_delta) (stamp - epoch_delta) * 100 else 0;
    if ((info.attributes & FILE_ATTRIBUTE_DIRECTORY) != 0) {
        return .directory;
    }

    const wanted = GetFinalPathNameByHandleW(handle, final_into, final_capacity, 0);
    if (wanted == 0) {
        return .unreadable;
    }
    if (wanted >= final_capacity) {
        return .path_too_long;
    }
    drawn.final_len = wanted;

    if (capacity == 0 or want == 0 or start >= size) {
        return .ok;
    }
    if (SetFilePointerEx(handle, @bitCast(start), null, FILE_BEGIN) == 0) {
        return .read_failed;
    }
    var read: u32 = 0;
    if (ReadFile(handle, into, want, &read, null) == 0) {
        return .read_failed;
    }
    const room = @min(@as(u64, read), size - start);
    drawn.written = @intCast(@min(room, capacity));
    return .ok;
}
