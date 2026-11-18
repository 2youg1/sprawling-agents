// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! One Mei message line, read whole whatever its length, and the single
//! ceiling every reader of those lines shares.

const std = @import("std");

/// A message line is a tool result, so it can carry pictures as base64: one
/// 5 MiB picture is 6.7 MiB of text and one python call may return several.
/// 32 MiB carries four of them beside the text and the JSON envelope; past
/// that the line is refused rather than truncated.
pub const MAX_BYTES: u64 = 32 * 1024 * 1024;

pub const Error = error{ MessageLineTooLong, OutOfMemory, ReadFailed };

/// Reads one line and its delimiter from `reader` and owns the answer, with a
/// Windows producer's trailing carriage return dropped. Answers null only
/// when the stream ended before a byte arrived, so an empty line and end of
/// stream stay distinguishable. A line of exactly `cap` bytes is read; one
/// byte more is an error and never a shortened line.
pub fn read(gpa: std.mem.Allocator, reader: *std.Io.Reader, cap: u64) Error!?[]u8 {
    var allocating: std.Io.Writer.Allocating = .init(gpa);
    errdefer allocating.deinit();
    _ = reader.streamDelimiterLimit(&allocating.writer, '\n', .limited(cap +| 1)) catch |err| switch (err) {
        error.StreamTooLong => return error.MessageLineTooLong,
        error.WriteFailed => return error.OutOfMemory,
        error.ReadFailed => return error.ReadFailed,
    };
    if (allocating.written().len == 0) {
        _ = reader.takeByte() catch {
            allocating.deinit();
            return null;
        };
        return @as(?[]u8, try allocating.toOwnedSlice());
    }
    _ = reader.takeByte() catch {};
    const owned = try allocating.toOwnedSlice();
    const kept = std.mem.trimEnd(u8, owned, "\r").len;
    if (kept == owned.len) return @as(?[]u8, owned);
    return @as(?[]u8, gpa.realloc(owned, kept) catch {
        gpa.free(owned);
        return error.OutOfMemory;
    });
}

/// The one message a reader of these lines gets when a line outgrows `cap`.
pub fn tooLong(gpa: std.mem.Allocator, cap: u64, path: ?[]const u8) ![]u8 {
    const mib = cap / (1024 * 1024);
    if (path) |named| {
        return try std.fmt.allocPrint(gpa,
            \\```toml
            \\code = "E_MEI_LINE"
            \\description = "read one Mei message line: it is larger than {d} MiB"
            \\cause = "one JSONL message holds a whole tool result, and a result may carry pictures as base64
            \\path = {s}
            \\value = {d} MiB, the largest line this shell reads"
            \\resolution = "return fewer or smaller pictures in one python call, or read the file instead of printing it; a session log written that way is read again after the change"
            \\```
        , .{ mib, named, mib });
    }
    return try std.fmt.allocPrint(gpa,
        \\```toml
        \\code = "E_MEI_LINE"
        \\description = "read one Mei message line: it is larger than {d} MiB"
        \\cause = "one JSONL message holds a whole tool result, and a result may carry pictures as base64
        \\value = {d} MiB, the largest line this shell reads"
        \\resolution = "return fewer or smaller pictures in one python call, or read the file instead of printing it"
        \\```
    , .{ mib, mib });
}

/// The text a failed read reports; every other error keeps its own name.
pub fn failureText(gpa: std.mem.Allocator, failure: anyerror) ![]u8 {
    if (failure == error.MessageLineTooLong) return tooLong(gpa, MAX_BYTES, null);
    return try std.fmt.allocPrint(gpa, "{s}", .{@errorName(failure)});
}

const Fixture = struct {
    dir: std.testing.TmpDir,
    path: []u8,

    fn init(gpa: std.mem.Allocator, io: std.Io, body: []const u8) !Fixture {
        const dir = std.testing.tmpDir(.{});
        const path = try std.fmt.allocPrint(gpa, ".zig-cache/tmp/{s}/lines.txt", .{dir.sub_path});
        errdefer gpa.free(path);
        try std.Io.Dir.cwd().writeFile(io, .{ .sub_path = path, .data = body });
        return .{ .dir = dir, .path = path };
    }

    fn deinit(self: *Fixture, gpa: std.mem.Allocator) void {
        gpa.free(self.path);
        self.dir.cleanup();
    }

    fn reader(self: *Fixture, io: std.Io, buf: []u8) !struct { file: std.Io.File, stream: std.Io.File.Reader } {
        const file = try std.Io.Dir.cwd().openFile(io, self.path, .{});
        return .{ .file = file, .stream = file.readerStreaming(io, buf) };
    }
};

test "a line longer than the streaming buffer is read whole" {
    const gpa = std.testing.allocator;
    const io = std.testing.io;
    const body = try gpa.alloc(u8, 20_006);
    defer gpa.free(body);
    @memset(body, 'a');
    @memcpy(body[20_000..], "\nshort");
    var fixture = try Fixture.init(gpa, io, body);
    defer fixture.deinit(gpa);

    var buf: [4096]u8 = undefined;
    var opened = try fixture.reader(io, &buf);
    defer opened.file.close(io);

    const first = (try read(gpa, &opened.stream.interface, MAX_BYTES)).?;
    defer gpa.free(first);
    try std.testing.expectEqual(@as(usize, 20_000), first.len);
    const second = (try read(gpa, &opened.stream.interface, MAX_BYTES)).?;
    defer gpa.free(second);
    try std.testing.expectEqualStrings("short", second);
    try std.testing.expect((try read(gpa, &opened.stream.interface, MAX_BYTES)) == null);
}

test "a line past the cap is a reported failure" {
    const gpa = std.testing.allocator;
    const io = std.testing.io;
    const body = try gpa.alloc(u8, 20_000);
    defer gpa.free(body);
    @memset(body, 'a');
    var fixture = try Fixture.init(gpa, io, body);
    defer fixture.deinit(gpa);

    var buf: [4096]u8 = undefined;
    var opened = try fixture.reader(io, &buf);
    defer opened.file.close(io);
    try std.testing.expectError(error.MessageLineTooLong, read(gpa, &opened.stream.interface, 4096));
}

test "the last line without a delimiter still answers once" {
    const gpa = std.testing.allocator;
    const io = std.testing.io;
    var fixture = try Fixture.init(gpa, io, "only");
    defer fixture.deinit(gpa);

    var buf: [4096]u8 = undefined;
    var opened = try fixture.reader(io, &buf);
    defer opened.file.close(io);
    const line = (try read(gpa, &opened.stream.interface, MAX_BYTES)).?;
    defer gpa.free(line);
    try std.testing.expectEqualStrings("only", line);
    try std.testing.expect((try read(gpa, &opened.stream.interface, MAX_BYTES)) == null);
}

test "an empty stream, an empty line and a carriage return are told apart" {
    const gpa = std.testing.allocator;
    const io = std.testing.io;
    var empty = try Fixture.init(gpa, io, "");
    defer empty.deinit(gpa);
    var empty_buf: [4096]u8 = undefined;
    var empty_opened = try empty.reader(io, &empty_buf);
    defer empty_opened.file.close(io);
    try std.testing.expect((try read(gpa, &empty_opened.stream.interface, MAX_BYTES)) == null);

    var blank = try Fixture.init(gpa, io, "\n\r\nx\n");
    defer blank.deinit(gpa);
    var blank_buf: [4096]u8 = undefined;
    var blank_opened = try blank.reader(io, &blank_buf);
    defer blank_opened.file.close(io);
    const one = (try read(gpa, &blank_opened.stream.interface, MAX_BYTES)).?;
    defer gpa.free(one);
    try std.testing.expectEqualStrings("", one);
    const two = (try read(gpa, &blank_opened.stream.interface, MAX_BYTES)).?;
    defer gpa.free(two);
    try std.testing.expectEqualStrings("", two);
    const three = (try read(gpa, &blank_opened.stream.interface, MAX_BYTES)).?;
    defer gpa.free(three);
    try std.testing.expectEqualStrings("x", three);
    try std.testing.expect((try read(gpa, &blank_opened.stream.interface, MAX_BYTES)) == null);
}

test "a line of exactly the cap is read with and without a delimiter" {
    const gpa = std.testing.allocator;
    const io = std.testing.io;
    var fixture = try Fixture.init(gpa, io, "abcd\nabcd");
    defer fixture.deinit(gpa);

    var buf: [8]u8 = undefined;
    var opened = try fixture.reader(io, &buf);
    defer opened.file.close(io);
    const first = (try read(gpa, &opened.stream.interface, 4)).?;
    defer gpa.free(first);
    try std.testing.expectEqualStrings("abcd", first);
    const second = (try read(gpa, &opened.stream.interface, 4)).?;
    defer gpa.free(second);
    try std.testing.expectEqualStrings("abcd", second);
    try std.testing.expect((try read(gpa, &opened.stream.interface, 4)) == null);
}

test "a line one byte past the cap is refused" {
    const gpa = std.testing.allocator;
    const io = std.testing.io;
    var fixture = try Fixture.init(gpa, io, "abcde");
    defer fixture.deinit(gpa);

    var buf: [8]u8 = undefined;
    var opened = try fixture.reader(io, &buf);
    defer opened.file.close(io);
    try std.testing.expectError(error.MessageLineTooLong, read(gpa, &opened.stream.interface, 4));
}
