// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

const std = @import("std");

const symbols = std.StaticStringMap([]const u8).initComptime(.{
    .{ "alpha", "α" },
    .{ "beta", "β" },
    .{ "gamma", "γ" },
    .{ "delta", "δ" },
    .{ "epsilon", "ε" },
    .{ "theta", "θ" },
    .{ "lambda", "λ" },
    .{ "mu", "μ" },
    .{ "pi", "π" },
    .{ "sigma", "σ" },
    .{ "phi", "φ" },
    .{ "omega", "ω" },
    .{ "Delta", "Δ" },
    .{ "Gamma", "Γ" },
    .{ "Sigma", "Σ" },
    .{ "Omega", "Ω" },
    .{ "sum", "∑" },
    .{ "prod", "∏" },
    .{ "int", "∫" },
    .{ "infty", "∞" },
    .{ "times", "×" },
    .{ "cdot", "·" },
    .{ "pm", "±" },
    .{ "div", "÷" },
    .{ "le", "≤" },
    .{ "leq", "≤" },
    .{ "ge", "≥" },
    .{ "geq", "≥" },
    .{ "ne", "≠" },
    .{ "approx", "≈" },
    .{ "to", "→" },
    .{ "rightarrow", "→" },
    .{ "in", "∈" },
    .{ "left", "" },
    .{ "right", "" },
    .{ "quad", "  " },
    .{ "qquad", "    " },
});

pub fn line(buffer: []u8, source: []const u8) ?[]const u8 {
    if (source.len > buffer.len) return null;
    var out: std.Io.Writer = .fixed(buffer);
    var at: usize = 0;
    var changed = false;
    var in_code = false;
    while (at < source.len) {
        const byte = source[at];
        if (byte == '`') in_code = !in_code;
        const rest = source[at..];
        if (!in_code and byte == '$' and (at == 0 or source[at - 1] != '\\')) {
            const marker = if (std.mem.startsWith(u8, rest, "$$")) "$$" else "$";
            const body = rest[marker.len..];
            if (std.mem.indexOf(u8, body, marker)) |close| {
                const formula = std.mem.trim(u8, body[0..close], " \t");
                if (formula.len > 0 and (marker.len == 2 or formula.len == close)) {
                    var rendered: [1024]u8 = undefined;
                    if (expression(&rendered, formula)) |converted| {
                        out.writeAll(converted) catch return null;
                        at += marker.len * 2 + close;
                        changed = true;
                        continue;
                    }
                }
            }
        }
        out.writeByte(byte) catch return null;
        at += 1;
    }
    return if (changed) out.buffered() else null;
}

pub fn expression(buffer: []u8, source: []const u8) ?[]const u8 {
    var out: std.Io.Writer = .fixed(buffer);
    convert(&out, source, 0) catch return null;
    return out.buffered();
}

const Error = std.Io.Writer.Error || error{Unsupported};

fn convert(out: *std.Io.Writer, source: []const u8, depth: usize) Error!void {
    if (depth >= 16) return error.Unsupported;
    var at: usize = 0;
    while (at < source.len) {
        const byte = source[at];
        at += 1;
        switch (byte) {
            '\\' => {
                const start = at;
                while (at < source.len and std.ascii.isAlphabetic(source[at])) : (at += 1) {}
                const name = source[start..at];
                if (symbols.get(name)) |symbol| {
                    try out.writeAll(symbol);
                } else if (std.mem.eql(u8, name, "frac")) {
                    try group(out, source, &at, depth, "(", ")⁄");
                    try group(out, source, &at, depth, "(", ")");
                } else if (std.mem.eql(u8, name, "sqrt")) {
                    try group(out, source, &at, depth, "√(", ")");
                } else return error.Unsupported;
            },
            '^', '_' => {
                if (at == source.len) return error.Unsupported;
                const top = byte == '^';
                if (source[at] == '{') {
                    const value = try takeGroup(source, &at);
                    try script(out, value, top);
                } else {
                    try script(out, source[at .. at + 1], top);
                    at += 1;
                }
            },
            '{' => {
                at -= 1;
                try group(out, source, &at, depth, "", "");
            },
            '}', '$' => return error.Unsupported,
            else => try out.writeByte(byte),
        }
    }
}

fn script(out: *std.Io.Writer, value: []const u8, top: bool) Error!void {
    const upper = [_][]const u8{ "⁰", "¹", "²", "³", "⁴", "⁵", "⁶", "⁷", "⁸", "⁹" };
    const lower = [_][]const u8{ "₀", "₁", "₂", "₃", "₄", "₅", "₆", "₇", "₈", "₉" };
    for (value) |digit| if (!std.ascii.isDigit(digit)) return error.Unsupported;
    for (value) |digit| try out.writeAll(if (top) upper[digit - '0'] else lower[digit - '0']);
}

fn group(out: *std.Io.Writer, source: []const u8, at: *usize, depth: usize, start: []const u8, end: []const u8) Error!void {
    const body = try takeGroup(source, at);
    try out.writeAll(start);
    try convert(out, body, depth + 1);
    try out.writeAll(end);
}

fn takeGroup(source: []const u8, at: *usize) Error![]const u8 {
    if (at.* >= source.len or source[at.*] != '{') return error.Unsupported;
    at.* += 1;
    const start = at.*;
    var nested: usize = 1;
    while (at.* < source.len) {
        const byte = source[at.*];
        at.* += 1;
        if (byte == '{') nested += 1;
        if (byte == '}') {
            nested -= 1;
            if (nested == 0) return source[start .. at.* - 1];
        }
    }
    return error.Unsupported;
}

test "basic inline and display math retain the visible source of unsupported formulas" {
    var buffer: [256]u8 = undefined;
    const rendered = line(&buffer, "Area $\\pi r^2$ and $$\\frac{1}{2}$$ beside $\\opaque{x}$").?;
    try std.testing.expectEqualStrings("Area π r² and (1)⁄(2) beside $\\opaque{x}$", rendered);
}
