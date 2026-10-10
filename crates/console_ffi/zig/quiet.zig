// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// The quiet host on the alternate screen: the address and the pairing
// code, and the key minted for a serve beyond this machine beside the
// code when there is one, as one block in the middle of the window, with
// at most one transient line two rows under it. Nothing else is drawn.

const std = @import("std");
const scene = @import("scene.zig");
const paint = @import("paint.zig");
const text = @import("text.zig");

pub const Error = scene.Error || paint.Error;

pub fn quiet(out: *std.Io.Writer, r: *scene.Reader) Error!void {
    const columns = try r.short();
    const rows = try r.short();
    const url = try r.text();
    const code = try r.text();
    const key = try r.text();
    const transient = try r.text();
    const label = "pairing code   ";
    const key_label = "   key  ";
    const second = label.len + text.width(code) + if (key.len > 0) key_label.len + text.width(key) else 0;
    const block = @max(@max(text.width(url), second), text.width(transient));
    const left = (@as(usize, columns) -| block) / 2;
    const top = (@as(usize, rows) -| 2) / 2;
    try out.writeAll("\x1b[?2026h\x1b[H\x1b[2J");
    var p = paint.Painter.init(out, columns);
    try place(out, top, left);
    p.column = left;
    try p.link(.plain, url, url);
    try p.close();
    try place(out, top + 1, left);
    p.column = left;
    try p.fit(.faint, label, label.len);
    try p.fit(.bold, code, p.room());
    if (key.len > 0) {
        try p.fit(.faint, key_label, key_label.len);
        try p.fit(.bold, key, p.room());
    }
    if (transient.len > 0) {
        try place(out, top + 3, left);
        p.column = left;
        try p.fit(.faint, transient, p.room());
    }
    try p.close();
    try out.writeAll("\x1b[?2026l");
}

/// Moves the cursor to a row and column counted from zero.
fn place(out: *std.Io.Writer, row: usize, column: usize) paint.Error!void {
    try out.print("\x1b[{d};{d}H", .{ row + 1, column + 1 });
}
