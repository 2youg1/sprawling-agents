//! The Zig performance kernel: byte-level leaves behind the thin FFI
//! adapter `crates/mem`. Input is `(ptr, len)`, output is one flat struct
//! plus an integer code. There is no domain model here, no error wording,
//! no canonical semantics, and no clock call of any kind (mem-SPEC.md
//! section 1).
//!
//! The envelope span scanner is the first leaf (implementation design
//! section 8.8): it locates the raw value of each envelope key inside one
//! JSON object without allocating. The key table is spelled here and in
//! Rust, and the proptest equivalence suite is what keeps the two
//! spellings one meaning (mem-SPEC.md section 12, decision 2).

const std = @import("std");

/// One located value: `[start, start + len)` in the caller's bytes.
pub const Span = extern struct {
    start: usize = 0,
    len: usize = 0,
};

/// The flat result: one presence bitmap and one fixed slot per key.
/// Slot `i` is valid when bit `i` of `found` is set.
pub const Spans = extern struct {
    found: u32 = 0,
    v: Span = .{ .start = 0, .len = 0 },
    seq: Span = .{ .start = 0, .len = 0 },
    prev: Span = .{ .start = 0, .len = 0 },
    kind: Span = .{ .start = 0, .len = 0 },
    ig: Span = .{ .start = 0, .len = 0 },
};

/// The integer codes crossing the boundary. Rust maps every one of them
/// at a single point and refuses any code this table does not name
/// (mem-SPEC.md section 8-1).
pub const Code = enum(c_int) {
    ok = 0,
    malformed = -1,
    duplicate_name = -2,
    too_deep = -3,
};

const ScanError = error{ Malformed, DuplicateName, TooDeep };

/// The slot each envelope key lands in, and its bit in `found`. An
/// exhaustive switch over this enum is how a span reaches its field.
const Slot = enum(u8) {
    v = 0,
    seq = 1,
    prev = 2,
    kind = 3,
    ig = 4,
};

/// The envelope key table: comptime perfect hash onto the slot.
const keys = std.StaticStringMap(Slot).initComptime(.{
    .{ "v", .v },
    .{ "seq", .seq },
    .{ "prev", .prev },
    .{ "kind", .kind },
    .{ "ig", .ig },
});

/// How many containers may be open at once, the envelope object included.
/// The scanner refuses past it rather than recursing without a floor; the
/// known divergence from the serde reference is recorded in mem-SPEC.md
/// section 3 with the condition that reopens it.
const MAX_DEPTH: usize = 1024;

/// Scans one JSON object and fills `out`. The preconditions the FFI call
/// in `crates/mem` states: `input` holds `len` readable bytes, `out` is
/// valid and writable, and neither is aliased for the duration.
export fn mem_envelope_spans(input: [*]const u8, len: usize, out: *Spans) c_int {
    const bytes = input[0..len];
    scanInto(bytes, out) catch |err| return codeOf(err);
    return @intFromEnum(Code.ok);
}

fn codeOf(err: ScanError) c_int {
    return switch (err) {
        error.Malformed => @intFromEnum(Code.malformed),
        error.DuplicateName => @intFromEnum(Code.duplicate_name),
        error.TooDeep => @intFromEnum(Code.too_deep),
    };
}

/// Zero-allocation scan. Duplicates among the five envelope names refuse
/// only after the whole object is walked, so a syntax error anywhere wins
/// over a duplicate name - the order serde parses in (mem-SPEC.md
/// section 8-1).
fn scanInto(input: []const u8, out: *Spans) ScanError!void {
    out.* = blank();
    var scanner = Scanner{ .bytes = input, .out = out };
    scanner.skipWs();
    try scanner.object(1);
    scanner.skipWs();
    if (scanner.at != input.len) return error.Malformed;
    if (scanner.duplicate) return error.DuplicateName;
}

/// One pass over the caller's bytes: the JSON syntax this leaf owns, and
/// the envelope policy the flat result carries. No allocation, no
/// recursion past `MAX_DEPTH`, nothing retained after the call.
const Scanner = struct {
    bytes: []const u8,
    at: usize = 0,
    out: *Spans,
    duplicate: bool = false,

    fn peek(self: *Scanner) ?u8 {
        return if (self.at < self.bytes.len) self.bytes[self.at] else null;
    }

    fn take(self: *Scanner) ?u8 {
        const byte = self.peek() orelse return null;
        self.at += 1;
        return byte;
    }

    /// RFC 8259 whitespace, the four bytes and no others.
    fn skipWs(self: *Scanner) void {
        while (self.at < self.bytes.len) : (self.at += 1) {
            switch (self.bytes[self.at]) {
                ' ', '\t', '\n', '\r' => {},
                else => return,
            }
        }
    }

    /// The envelope object is the depth-1 container; its members' names
    /// are the only ones read as keys. A deeper object's names are
    /// validated and skipped.
    fn object(self: *Scanner, depth: usize) ScanError!void {
        if (depth > MAX_DEPTH) return error.TooDeep;
        if (self.peek() != '{') return error.Malformed;
        self.at += 1;
        try self.members(depth);
    }

    fn members(self: *Scanner, depth: usize) ScanError!void {
        self.skipWs();
        if (self.peek() == '}') {
            self.at += 1;
            return;
        }
        while (true) {
            self.skipWs();
            if (self.peek() != '"') return error.Malformed;
            var name = Name{};
            try self.string(&name);
            self.skipWs();
            if (self.peek() != ':') return error.Malformed;
            self.at += 1;
            self.skipWs();
            const start = self.at;
            try self.value(depth);
            if (depth == 1) self.record(name.slot(), start, self.at);
            self.skipWs();
            switch (self.take() orelse return error.Malformed) {
                ',' => continue,
                '}' => return,
                else => return error.Malformed,
            }
        }
    }

    fn array(self: *Scanner, depth: usize) ScanError!void {
        if (depth > MAX_DEPTH) return error.TooDeep;
        self.at += 1;
        self.skipWs();
        if (self.peek() == ']') {
            self.at += 1;
            return;
        }
        while (true) {
            self.skipWs();
            try self.value(depth);
            self.skipWs();
            switch (self.take() orelse return error.Malformed) {
                ',' => continue,
                ']' => return,
                else => return error.Malformed,
            }
        }
    }

    /// Walks one value to its end, validating its syntax. One more
    /// container is open for every `{` or `[` met on the way, and the
    /// walk refuses past `MAX_DEPTH` rather than recursing without a
    /// floor.
    fn value(self: *Scanner, depth: usize) ScanError!void {
        switch (self.peek() orelse return error.Malformed) {
            '"' => try self.string(null),
            '{' => try self.object(depth + 1),
            '[' => try self.array(depth + 1),
            't' => try self.literal("true"),
            'f' => try self.literal("false"),
            'n' => try self.literal("null"),
            '-', '0'...'9' => try self.number(),
            else => return error.Malformed,
        }
    }

    fn literal(self: *Scanner, text: []const u8) ScanError!void {
        if (!std.mem.startsWith(u8, self.bytes[self.at..], text)) return error.Malformed;
        self.at += text.len;
    }

    /// Consumes one string, validating strict UTF-8 and escape syntax.
    /// When `name` is given the decoded bytes feed it, capped where the
    /// key table ends - a name is read only far enough to answer "is
    /// this one of the five" (mem-SPEC.md section 10).
    fn string(self: *Scanner, name: ?*Name) ScanError!void {
        self.at += 1; // the opening quote the caller saw
        while (true) {
            const byte = self.take() orelse return error.Malformed;
            switch (byte) {
                '"' => return,
                '\\' => try self.escape(name),
                0x00...0x1F => return error.Malformed,
                else => {
                    try self.utf8Tail(byte);
                    if (name) |captured| {
                        if (byte < 0x80) captured.push(byte) else captured.notKey();
                    }
                },
            }
        }
    }

    /// The continuation bytes of one UTF-8 sequence, strict: no overlong
    /// encoding, no surrogate half, nothing past U+10FFFF - the strings
    /// serde_json accepts (mem-SPEC.md section 3).
    fn utf8Tail(self: *Scanner, first: u8) ScanError!void {
        switch (first) {
            0x00...0x7F => {},
            0xC2...0xDF => try self.cont(0x80, 0xBF),
            0xE0 => {
                try self.cont(0xA0, 0xBF);
                try self.cont(0x80, 0xBF);
            },
            0xE1...0xEC, 0xEE...0xEF => {
                try self.cont(0x80, 0xBF);
                try self.cont(0x80, 0xBF);
            },
            0xED => {
                try self.cont(0x80, 0x9F);
                try self.cont(0x80, 0xBF);
            },
            0xF0 => {
                try self.cont(0x90, 0xBF);
                try self.cont(0x80, 0xBF);
                try self.cont(0x80, 0xBF);
            },
            0xF1...0xF3 => {
                try self.cont(0x80, 0xBF);
                try self.cont(0x80, 0xBF);
                try self.cont(0x80, 0xBF);
            },
            0xF4 => {
                try self.cont(0x80, 0x8F);
                try self.cont(0x80, 0xBF);
                try self.cont(0x80, 0xBF);
            },
            else => return error.Malformed,
        }
    }

    fn cont(self: *Scanner, lo: u8, hi: u8) ScanError!void {
        const byte = self.take() orelse return error.Malformed;
        if (byte < lo or byte > hi) return error.Malformed;
    }

    /// One escape, decoded into the name when there is one. A `\uXXXX`
    /// needs its four hex digits and no surrogate pairing - the check
    /// `RawValue` does not do either (mem-SPEC.md section 3).
    fn escape(self: *Scanner, name: ?*Name) ScanError!void {
        const byte = self.take() orelse return error.Malformed;
        if (byte == 'u') {
            const code = try self.hex4();
            if (name) |captured| {
                if (code < 0x80) captured.push(@intCast(code)) else captured.notKey();
            }
            return;
        }
        const plain: u8 = switch (byte) {
            '"', '\\', '/' => byte,
            'b' => 0x08,
            'f' => 0x0C,
            'n' => 0x0A,
            'r' => 0x0D,
            't' => 0x09,
            else => return error.Malformed,
        };
        if (name) |captured| captured.push(plain);
    }

    fn hex4(self: *Scanner) ScanError!u16 {
        var code: u16 = 0;
        for (0..4) |_| {
            const byte = self.take() orelse return error.Malformed;
            const digit: u16 = switch (byte) {
                '0'...'9' => byte - '0',
                'a'...'f' => byte - 'a' + 10,
                'A'...'F' => byte - 'A' + 10,
                else => return error.Malformed,
            };
            code = code * 16 + digit;
        }
        return code;
    }

    /// RFC 8259 strict, the shape serde_json accepts: no digit beside a
    /// leading zero, a fraction carries its digits, an exponent its own
    /// (mem-SPEC.md section 3).
    fn number(self: *Scanner) ScanError!void {
        var byte = self.take() orelse return error.Malformed;
        if (byte == '-') byte = self.take() orelse return error.Malformed;
        switch (byte) {
            '0' => {},
            '1'...'9' => self.digits(),
            else => return error.Malformed,
        }
        if (self.peek() == '.') {
            self.at += 1;
            try self.digits1();
        }
        if (self.peek() == 'e' or self.peek() == 'E') {
            self.at += 1;
            if (self.peek() == '+' or self.peek() == '-') self.at += 1;
            try self.digits1();
        }
        // The token ends where a delimiter begins: `1x` is not a number
        // followed by an unknown, it is not a number.
        switch (self.peek() orelse return) {
            ' ', '\t', '\n', '\r', ',', '}', ']' => {},
            else => return error.Malformed,
        }
    }

    fn digits(self: *Scanner) void {
        while (true) {
            const byte = self.peek() orelse return;
            if (byte < '0' or byte > '9') return;
            self.at += 1;
        }
    }

    fn digits1(self: *Scanner) ScanError!void {
        const byte = self.peek() orelse return error.Malformed;
        if (byte < '0' or byte > '9') return error.Malformed;
        self.digits();
    }

    /// A located envelope key keeps the first span and flags the
    /// duplicate; the call refuses only once the walk is done.
    fn record(self: *Scanner, slot: ?Slot, start: usize, end: usize) void {
        const key = slot orelse return;
        const shift: u5 = @intCast(@intFromEnum(key));
        const bit = @as(u32, 1) << shift;
        if (self.out.found & bit != 0) {
            self.duplicate = true;
            return;
        }
        self.out.found |= bit;
        const span = Span{ .start = start, .len = end - start };
        switch (key) {
            .v => self.out.v = span,
            .seq => self.out.seq = span,
            .prev => self.out.prev = span,
            .kind => self.out.kind = span,
            .ig => self.out.ig = span,
        }
    }
};

/// What a member's name decodes to, capped at the longest key: enough to
/// answer "is this one of the five", and nothing more. A name carrying a
/// non-ASCII byte or running past four decoded bytes is not a key, and
/// the bytes after that point are validated without being kept.
const Name = struct {
    text: [4]u8 = [_]u8{0} ** 4,
    len: u8 = 0,
    keyable: bool = true,

    fn push(self: *Name, byte: u8) void {
        if (!self.keyable) return;
        if (byte >= 0x80 or self.len == 4) {
            self.notKey();
            return;
        }
        self.text[self.len] = byte;
        self.len += 1;
    }

    fn notKey(self: *Name) void {
        self.keyable = false;
    }

    fn slot(self: *Name) ?Slot {
        if (!self.keyable) return null;
        return keys.get(self.text[0..self.len]);
    }
};

fn blank() Spans {
    return Spans{
        .found = 0,
        .v = .{ .start = 0, .len = 0 },
        .seq = .{ .start = 0, .len = 0 },
        .prev = .{ .start = 0, .len = 0 },
        .kind = .{ .start = 0, .len = 0 },
        .ig = .{ .start = 0, .len = 0 },
    };
}

fn sliceOf(input: []const u8, span: Span) []const u8 {
    return input[span.start..][0..span.len];
}

test "the envelope's five keys land in their slots" {
    const line =
        \\{"v":1,"seq":"2","prev":{"x":[1,2]},"kind":"run","ig":false,"later":9}
    ;
    var out = blank();
    try scanInto(line, &out);
    try std.testing.expectEqual(@as(u32, 0b11111), out.found);
    try std.testing.expectEqualStrings("1", sliceOf(line, out.v));
    try std.testing.expectEqualStrings("\"2\"", sliceOf(line, out.seq));
    try std.testing.expectEqualStrings("{\"x\":[1,2]}", sliceOf(line, out.prev));
    try std.testing.expectEqualStrings("\"run\"", sliceOf(line, out.kind));
    try std.testing.expectEqualStrings("false", sliceOf(line, out.ig));
}

test "absent keys stay absent and an empty object is one" {
    var out = blank();
    try scanInto("{\"kind\":\"run\"}", &out);
    try std.testing.expectEqual(@as(u32, 0b01000), out.found);
    var empty = blank();
    try scanInto("  {}  ", &empty);
    try std.testing.expectEqual(@as(u32, 0), empty.found);
}

test "a name's escapes do not hide a duplicate envelope key" {
    var out = blank();
    try std.testing.expectError(
        error.DuplicateName,
        scanInto("{\"v\":1,\"\\u0076\":2}", &out),
    );
    var unknown = blank();
    try scanInto("{\"x\":1,\"x\":2,\"v\":1}", &unknown);
    try std.testing.expectEqual(@as(u32, 0b00001), unknown.found);
}

test "malformed input is refused" {
    const cases = [_][]const u8{
        "",
        "null",
        "[1,2]",
        "{",
        "{\"v\":}",
        "{\"v\":01}",
        "{\"v\":1 trailing}",
        "{\"v\":1} x",
        "{\"k\":\"\xff\"}",
        "{\"k\":\"\\ud80\"}",
    };
    for (cases) |case| {
        var out = blank();
        try std.testing.expectError(error.Malformed, scanInto(case, &out));
    }
}

test "nesting deeper than the cap is refused rather than recursing out" {
    // The envelope object plus 1023 arrays fill the cap; one more is -3.
    var gpa: std.heap.DebugAllocator(.{}) = .init;
    defer std.debug.assert(gpa.deinit() == .ok);
    const allocator = gpa.allocator();

    const at_cap = try nest(allocator, 1023);
    defer allocator.free(at_cap);
    var out = blank();
    try scanInto(at_cap, &out);

    const past = try nest(allocator, 1024);
    defer allocator.free(past);
    var deep = blank();
    try std.testing.expectError(error.TooDeep, scanInto(past, &deep));
}

fn nest(allocator: std.mem.Allocator, depth: usize) ![]u8 {
    var text = std.ArrayList(u8).empty;
    errdefer text.deinit(allocator);
    try text.appendSlice(allocator, "{\"k\":");
    for (0..depth) |_| try text.append(allocator, '[');
    for (0..depth) |_| try text.append(allocator, ']');
    try text.append(allocator, '}');
    return text.toOwnedSlice(allocator);
}

test "fuzz envelope spans" {
    try std.testing.fuzz({}, fuzzSpans, .{});
}

fn fuzzSpans(_: void, smith: *std.testing.Smith) !void {
    // The scanner window: never escaped, never retained.
    var scratch: [2048]u8 = undefined;
    const n = smith.slice(&scratch);
    var out = blank();
    _ = scanInto(scratch[0..n], &out) catch return;
}
