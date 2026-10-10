// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// Characters as a terminal shows them: how many columns each one takes,
// and which ones never reach the terminal as themselves. A record's text
// is a model's or a person's words, and a control character inside it
// would be a command to the terminal, so every C0 and C1 control, DEL,
// and every byte that is not UTF-8 is drawn as U+FFFD instead.

const std = @import("std");

pub const replacement = "\u{FFFD}";

/// One character of a text: what it is, how many bytes it took, and
/// whether it is drawn as itself.
pub const Char = struct {
    cp: u21,
    len: usize,
    /// The bytes to write: the character itself, or U+FFFD.
    shown: []const u8,
    columns: usize,
};

/// The character starting at `at`, which must be inside `bytes`.
pub fn char(bytes: []const u8, at: usize) Char {
    const n = sequenceLength(bytes[at]) orelse return unreadable();
    if (at + n > bytes.len) return unreadable();
    const slice = bytes[at .. at + n];
    const cp = std.unicode.utf8Decode(slice) catch return unreadable();
    if (isControl(cp)) return .{ .cp = 0xFFFD, .len = n, .shown = replacement, .columns = 1 };
    return .{ .cp = cp, .len = n, .shown = slice, .columns = columns(cp) };
}

/// How many bytes a UTF-8 sequence takes, read from its first byte;
/// nothing for a byte no sequence starts with. Whether the sequence is
/// well formed is `utf8Decode`'s to judge.
fn sequenceLength(lead: u8) ?usize {
    if (lead < 0x80) return 1;
    if (lead & 0xE0 == 0xC0) return 2;
    if (lead & 0xF0 == 0xE0) return 3;
    if (lead & 0xF8 == 0xF0) return 4;
    return null;
}

fn unreadable() Char {
    return .{ .cp = 0xFFFD, .len = 1, .shown = replacement, .columns = 1 };
}

fn isControl(cp: u21) bool {
    return cp < 0x20 or cp == 0x7F or (cp >= 0x80 and cp < 0xA0);
}

/// Columns one character takes: none for a combining mark, a joiner or
/// a variation selector; two for the East Asian wide and fullwidth
/// ranges and the emoji blocks; one otherwise.
pub fn columns(cp: u21) usize {
    return switch (cp) {
        0x0300...0x036F, 0x1AB0...0x1AFF, 0x1DC0...0x1DFF, 0x200B...0x200F, 0x20D0...0x20FF, 0xFE00...0xFE0F, 0xFE20...0xFE2F => 0,
        0x1100...0x115F, 0x2E80...0x303E, 0x3041...0x33FF, 0x3400...0x4DBF, 0x4E00...0x9FFF, 0xA000...0xA4CF, 0xAC00...0xD7A3, 0xF900...0xFAFF, 0xFE30...0xFE4F, 0xFF00...0xFF60, 0xFFE0...0xFFE6, 0x1F300...0x1F64F, 0x1F680...0x1F6FF, 0x1F900...0x1F9FF, 0x20000...0x3FFFD => 2,
        else => 1,
    };
}

/// Columns a whole text takes on one line.
pub fn width(bytes: []const u8) usize {
    var at: usize = 0;
    var total: usize = 0;
    while (at < bytes.len) {
        const c = char(bytes, at);
        total += c.columns;
        at += c.len;
    }
    return total;
}

test "a control character is drawn as the replacement character" {
    const c = char("\x1b[2J", 0);
    try std.testing.expectEqualStrings(replacement, c.shown);
}

test "Chinese takes two columns a character, a combining mark none" {
    try std.testing.expectEqual(@as(usize, 4), width("中文"));
    try std.testing.expectEqual(@as(usize, 1), width("e\u{0301}"));
}

test "a cut UTF-8 sequence is one replacement character" {
    const c = char("\xe4\xb8", 0);
    try std.testing.expectEqual(@as(usize, 1), c.len);
    try std.testing.expectEqualStrings(replacement, c.shown);
}
