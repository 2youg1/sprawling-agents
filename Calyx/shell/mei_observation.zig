// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

const std = @import("std");
const leaf = @import("leaf");
const write = leaf.write;
const part = leaf.part;
const local_time = @import("local_time.zig");
const program_status = @import("program_status.zig");
const mei_process = @import("mei_process.zig");

pub const PythonBody = struct {
    code: []u8,
    output: []u8,
    took_us: ?u64,
    outcome: part.Outcome,
    at: ?u32,
};

const Pending = struct { id: []u8, code: []u8 };

const empty_code: [0]u8 = .{};

pub const Seen = struct {
    gpa: std.mem.Allocator,
    model: []u8,
    effort: []u8,
    effort_actual: bool = false,
    efforts: std.ArrayList([]u8) = .empty,
    context_tokens: ?u64 = null,
    used_tokens: ?u64 = null,
    step: u64 = 0,
    since: ?u64 = null,
    calling: bool = false,
    rest: program_status.Rest = .idle,
    headed: bool = false,
    last_text: []u8 = "",
    draft: []u8 = "",
    echo: []u8 = "",
    wave_ms: ?u64 = null,
    pending: std.ArrayList(Pending) = .empty,
    bodies: std.ArrayList(PythonBody) = .empty,
    run: bool = false,

    pub fn init(gpa: std.mem.Allocator, model: []const u8) !Seen {
        return .{
            .gpa = gpa,
            .model = try gpa.dupe(u8, model),
            .effort = try gpa.dupe(u8, "high"),
            .last_text = try gpa.dupe(u8, ""),
            .draft = try gpa.dupe(u8, ""),
            .echo = try gpa.dupe(u8, ""),
        };
    }

    pub fn deinit(self: *Seen) void {
        self.gpa.free(self.model);
        self.gpa.free(self.effort);
        self.clearEfforts();
        self.efforts.deinit(self.gpa);
        self.gpa.free(self.last_text);
        self.gpa.free(self.draft);
        self.gpa.free(self.echo);
        self.clearPending();
        self.pending.deinit(self.gpa);
        self.clearBodies();
        self.bodies.deinit(self.gpa);
        self.* = undefined;
    }

    pub fn suppressEcho(self: *Seen, text: []const u8) void {
        self.replace(&self.echo, text);
    }

    pub fn takeBodies(self: *Seen) ![]PythonBody {
        if (self.bodies.items.len == 0) return &.{};
        return try self.bodies.toOwnedSlice(self.gpa);
    }

    pub fn draftText(self: *const Seen) []const u8 {
        return self.draft;
    }

    fn replace(self: *Seen, field: *[]u8, value: []const u8) void {
        const owned = self.gpa.dupe(u8, value) catch return;
        self.gpa.free(field.*);
        field.* = owned;
    }

    fn clearEfforts(self: *Seen) void {
        for (self.efforts.items) |level| self.gpa.free(level);
        self.efforts.clearRetainingCapacity();
    }

    pub fn reset(self: *Seen) void {
        self.since = null;
        self.calling = false;
        self.rest = .idle;
        self.headed = false;
        self.used_tokens = null;
        self.step = 0;
        self.run = false;
        self.effort_actual = false;
        self.setLast("");
        self.clearDraft();
        self.clearPending();
        self.clearBodies();
        self.wave_ms = null;
        self.replace(&self.echo, "");
    }

    pub fn begin(self: *Seen, at: u64) void {
        self.run = true;
        self.since = at;
        self.calling = false;
        self.rest = .idle;
        self.headed = false;
        self.used_tokens = null;
        self.step = 0;
        self.effort_actual = false;
        self.setLast("");
        self.clearDraft();
        self.clearPending();
        self.wave_ms = null;
    }

    pub fn counts(self: *const Seen) program_status.Counts {
        return .{ .runs = if (self.run) 1 else 0, .approvals = 0 };
    }

    pub fn effortLevels(self: *const Seen) []const []const u8 {
        return self.efforts.items;
    }

    pub fn currentEffort(self: *const Seen) []const u8 {
        return self.effort;
    }

    pub fn requestEffort(self: *Seen, effort: []const u8) void {
        self.replace(&self.effort, effort);
        self.effort_actual = false;
    }

    pub fn offer(self: *Seen, buf: []u8) []const u8 {
        const source = if (self.effort_actual) "effort" else "requested";
        var base_buf: [384]u8 = undefined;
        const base = std.fmt.bufPrint(&base_buf, "{s} · {s} {s} · Agent Step {d}", .{
            self.model,
            source,
            self.effort,
            self.step,
        }) catch return self.model;
        if (self.used_tokens) |used| {
            if (self.context_tokens) |window| {
                const scaled = @as(u128, used) * 10000 + window / 2;
                const hundredths = scaled / window;
                return std.fmt.bufPrint(buf, "{s} · {d}.{d}{d}% · {d}/{d}", .{
                    base,
                    hundredths / 100,
                    (hundredths % 100) / 10,
                    hundredths % 10,
                    used,
                    window,
                }) catch base;
            }
        }
        if (self.context_tokens) |window| {
            return std.fmt.bufPrint(buf, "{s} · max {d}", .{ base, window }) catch base;
        }
        return std.fmt.bufPrint(buf, "{s} · context unknown", .{base}) catch base;
    }

    pub fn line(self: *Seen, arena: std.mem.Allocator, raw: []const u8, at: u64) ![]write.Entry {
        var parsed = std.json.parseFromSlice(std.json.Value, arena, raw, .{}) catch {
            return try noteEntries(arena, raw);
        };
        defer parsed.deinit();
        const value = parsed.value;
        if (value != .object) return try noteEntries(arena, raw);
        if (value.object.get("type")) |ty| {
            if (ty == .string and std.mem.eql(u8, ty.string, "mei.obs")) {
                return try self.observation(arena, value);
            }
            if (ty == .string and std.mem.eql(u8, ty.string, "mei.stream")) {
                return try self.stream(arena, value, at);
            }
        }
        const role = value.object.get("role") orelse return try noteEntries(arena, raw);
        const content = value.object.get("content") orelse return try noteEntries(arena, raw);
        if (role != .string) return try noteEntries(arena, raw);
        if (std.mem.eql(u8, role.string, "assistant")) {
            self.clearDraft();
            return try self.assistant(arena, content, at);
        }
        if (std.mem.eql(u8, role.string, "user")) return try self.userMessage(arena, content, at);
        return try noteEntries(arena, raw);
    }

    fn stream(self: *Seen, arena: std.mem.Allocator, value: std.json.Value, at: u64) ![]write.Entry {
        const kind = if (value.object.get("kind")) |k| (if (k == .string) k.string else "") else "";
        if (std.mem.eql(u8, kind, "delta")) {
            const text = if (value.object.get("text")) |tx| (if (tx == .string) tx.string else "") else "";
            if (text.len > 0) self.appendDraft(text);
            return &.{};
        }
        if (std.mem.eql(u8, kind, "commit")) {
            return try self.commitDraft(arena, at);
        }
        return &.{};
    }

    pub fn flushDraft(self: *Seen, arena: std.mem.Allocator, at: u64) ![]write.Entry {
        return try self.commitDraft(arena, at);
    }

    fn commitDraft(self: *Seen, arena: std.mem.Allocator, at: u64) ![]write.Entry {
        if (self.draft.len == 0) return &.{};
        var entries: std.ArrayList(write.Entry) = .empty;
        if (!self.headed) try entries.append(arena, try self.headEntry(arena, at));
        const said = try arena.dupe(u8, self.draft);
        self.setLast(said);
        self.clearDraft();
        try entries.append(arena, .{ .reply = said });
        return try entries.toOwnedSlice(arena);
    }

    fn headEntry(self: *Seen, arena: std.mem.Allocator, at: u64) !write.Entry {
        self.headed = true;
        const facts: []const []const u8 = if (self.model.len == 0) &.{} else blk: {
            const owned = try arena.dupe(u8, self.model);
            const list = try arena.alloc([]const u8, 1);
            list[0] = owned;
            break :blk list;
        };
        return .{ .head = .{
            .at = tod(at),
            .resident = "Mei",
            .facts = facts,
        } };
    }

    fn appendDraft(self: *Seen, text: []const u8) void {
        const next = std.fmt.allocPrint(self.gpa, "{s}{s}", .{ self.draft, text }) catch return;
        self.gpa.free(self.draft);
        self.draft = next;
    }

    fn clearDraft(self: *Seen) void {
        if (self.draft.len == 0) return;
        const empty = self.gpa.alloc(u8, 0) catch return;
        self.gpa.free(self.draft);
        self.draft = empty;
    }

    fn observation(self: *Seen, arena: std.mem.Allocator, value: std.json.Value) ![]write.Entry {
        _ = arena;
        const kind = if (value.object.get("kind")) |k| (if (k == .string) k.string else "") else "";
        if (std.mem.eql(u8, kind, "model")) {
            if (value.object.get("model")) |model| {
                if (model == .string) self.replace(&self.model, model.string);
            }
            self.context_tokens = if (value.object.get("context_tokens")) |tokens|
                (if (tokens == .integer and tokens.integer > 0) @intCast(tokens.integer) else null)
            else
                null;
            if (value.object.get("effort")) |effort| {
                if (effort == .string) self.requestEffort(effort.string);
            }
            if (value.object.get("efforts")) |levels| {
                if (levels == .array) {
                    self.clearEfforts();
                    for (levels.array.items) |level| {
                        if (level != .string) continue;
                        const owned = self.gpa.dupe(u8, level.string) catch continue;
                        self.efforts.append(self.gpa, owned) catch {
                            self.gpa.free(owned);
                            continue;
                        };
                    }
                }
            }
        } else if (std.mem.eql(u8, kind, "route")) {
            const available = if (value.object.get("available")) |flag| flag == .bool and flag.bool else false;
            if (available) {
                if (value.object.get("model")) |model| {
                    if (model == .string) self.replace(&self.model, model.string);
                }
                if (value.object.get("effort")) |effort| {
                    if (effort == .string) {
                        self.replace(&self.effort, effort.string);
                        self.effort_actual = true;
                    }
                }
            } else {
                self.effort_actual = false;
            }
        } else if (std.mem.eql(u8, kind, "usage")) {
            if (value.object.get("usage")) |usage| {
                if (usage == .object) {
                    var used: u64 = 0;
                    for ([_][]const u8{ "input_tokens", "cache_creation_input_tokens", "cache_read_input_tokens" }) |name| {
                        if (usage.object.get(name)) |tokens| {
                            if (tokens == .integer and tokens.integer >= 0) used +|= @intCast(tokens.integer);
                        }
                    }
                    self.used_tokens = used;
                    self.step +|= 1;
                }
            }
        }
        return &.{};
    }

    fn assistant(self: *Seen, arena: std.mem.Allocator, content: std.json.Value, at: u64) ![]write.Entry {
        var entries: std.ArrayList(write.Entry) = .empty;
        if (!self.headed) try entries.append(arena, try self.headEntry(arena, at));
        if (content == .string) {
            const text = try arena.dupe(u8, content.string);
            self.setLast(text);
            if (text.len > 0) try entries.append(arena, .{ .reply = text });
            return try entries.toOwnedSlice(arena);
        }
        if (content != .array) return try entries.toOwnedSlice(arena);
        var reasoning: u64 = 0;
        var text: std.ArrayList(u8) = .empty;
        defer text.deinit(arena);
        for (content.array.items) |block| {
            if (block != .object) continue;
            const ty = if (block.object.get("type")) |t| (if (t == .string) t.string else "") else "";
            if (std.mem.eql(u8, ty, "thinking")) {
                if (block.object.get("thinking")) |th| {
                    if (th == .string) reasoning +|= th.string.len;
                }
            } else if (std.mem.eql(u8, ty, "text")) {
                if (block.object.get("text")) |tx| {
                    if (tx == .string) try text.appendSlice(arena, tx.string);
                }
            } else if (std.mem.eql(u8, ty, "tool_use")) {
                self.noteCall(block, at);
                self.calling = true;
            }
        }
        if (reasoning > 0) try entries.append(arena, .{ .reasoning = reasoning });
        const said = try text.toOwnedSlice(arena);
        self.setLast(said);
        if (said.len > 0) try entries.append(arena, .{ .reply = said });
        return try entries.toOwnedSlice(arena);
    }

    fn userMessage(self: *Seen, arena: std.mem.Allocator, content: std.json.Value, at: u64) ![]write.Entry {
        if (content == .string) return try self.youUnlessEcho(arena, content.string, at);
        var entries: std.ArrayList(write.Entry) = .empty;
        if (content != .array) return try entries.toOwnedSlice(arena);
        var said: std.ArrayList(u8) = .empty;
        defer said.deinit(arena);
        for (content.array.items) |block| {
            if (block != .object) continue;
            const ty = if (block.object.get("type")) |t| (if (t == .string) t.string else "") else "";
            if (std.mem.eql(u8, ty, "tool_result")) {
                try self.oneResult(&entries, arena, block, at);
            } else if (std.mem.eql(u8, ty, "text")) {
                if (block.object.get("text")) |text| {
                    if (text == .string and text.string.len > 0) {
                        if (said.items.len > 0) try said.append(arena, '\n');
                        try said.appendSlice(arena, text.string);
                    }
                }
            }
        }
        if (said.items.len > 0) {
            if (!(self.echo.len > 0 and std.mem.eql(u8, self.echo, said.items))) {
                try entries.append(arena, .{ .you = .{ .at = tod(at), .said = try arena.dupe(u8, said.items) } });
            } else self.replace(&self.echo, "");
        }
        self.calling = false;
        self.wave_ms = null;
        return try entries.toOwnedSlice(arena);
    }

    fn youUnlessEcho(self: *Seen, arena: std.mem.Allocator, said: []const u8, at: u64) ![]write.Entry {
        if (said.len == 0) return &.{};
        if (self.echo.len > 0 and std.mem.eql(u8, self.echo, said)) {
            self.replace(&self.echo, "");
            return &.{};
        }
        const entries = try arena.alloc(write.Entry, 1);
        entries[0] = .{ .you = .{ .at = tod(at), .said = try arena.dupe(u8, said) } };
        return entries;
    }

    fn noteCall(self: *Seen, block: std.json.Value, at: u64) void {
        if (self.wave_ms == null) self.wave_ms = at;
        const id = if (block.object.get("id")) |value| (if (value == .string) value.string else "") else "";
        const code = if (block.object.get("input")) |input|
            (if (input == .object) (if (input.object.get("code")) |value| (if (value == .string) value.string else "") else "") else "")
        else
            "";
        const owned_id = self.gpa.dupe(u8, id) catch return;
        const owned_code = self.gpa.dupe(u8, code) catch {
            self.gpa.free(owned_id);
            return;
        };
        self.pending.append(self.gpa, .{ .id = owned_id, .code = owned_code }) catch {
            self.gpa.free(owned_id);
            self.gpa.free(owned_code);
        };
    }

    fn oneResult(self: *Seen, entries: *std.ArrayList(write.Entry), arena: std.mem.Allocator, block: std.json.Value, at: u64) !void {
        const id = if (block.object.get("tool_use_id")) |value| (if (value == .string) value.string else "") else "";
        const failed = if (block.object.get("is_error")) |value| value == .bool and value.bool else false;
        const code = self.takeCode(id);
        const output = if (block.object.get("content")) |result| try self.ownResult(result) else try self.gpa.dupe(u8, "");
        const took_us = if (self.wave_ms) |start| if (at > start) (at - start) * 1000 else null else null;
        self.bodies.append(self.gpa, .{
            .code = code,
            .output = output,
            .took_us = took_us,
            .outcome = if (failed) .Failed else .Answered,
            .at = tod(at),
        }) catch {
            self.release(code);
            self.release(output);
            return error.OutOfMemory;
        };
        errdefer {
            const dropped = self.bodies.pop().?;
            self.release(dropped.code);
            self.release(dropped.output);
        }
        try entries.append(arena, .{ .tool = .{
            .at = tod(at),
            .name = "python",
            .subject = "",
            .took_us = took_us,
            .outcome = if (failed) .Failed else .Answered,
        } });
    }

    fn takeCode(self: *Seen, id: []const u8) []u8 {
        var index: usize = 0;
        while (index < self.pending.items.len) : (index += 1) {
            if (id.len == 0 or std.mem.eql(u8, self.pending.items[index].id, id)) {
                const item = self.pending.orderedRemove(index);
                self.gpa.free(item.id);
                return item.code;
            }
        }
        if (self.pending.items.len > 0) {
            const item = self.pending.orderedRemove(0);
            self.gpa.free(item.id);
            return item.code;
        }
        return self.gpa.dupe(u8, "") catch @constCast(&empty_code);
    }

    fn ownResult(self: *Seen, result: std.json.Value) ![]u8 {
        switch (result) {
            .string => |text| return try self.gpa.dupe(u8, text),
            .array => |parts| {
                var out: std.ArrayList(u8) = .empty;
                errdefer out.deinit(self.gpa);
                for (parts.items) |item| {
                    if (item != .object) continue;
                    const kind = if (item.object.get("type")) |t| (if (t == .string) t.string else "") else "";
                    const piece: []const u8 = if (std.mem.eql(u8, kind, "text")) blk: {
                        if (item.object.get("text")) |text| {
                            if (text == .string) break :blk text.string;
                        }
                        break :blk "";
                    } else if (std.mem.eql(u8, kind, "image"))
                        "[image attached]"
                    else
                        "";
                    if (piece.len == 0) continue;
                    if (out.items.len > 0) try out.append(self.gpa, '\n');
                    try out.appendSlice(self.gpa, piece);
                }
                return try out.toOwnedSlice(self.gpa);
            },
            else => return try std.json.Stringify.valueAlloc(self.gpa, result, .{}),
        }
    }

    fn clearPending(self: *Seen) void {
        for (self.pending.items) |item| {
            self.gpa.free(item.id);
            self.gpa.free(item.code);
        }
        self.pending.clearRetainingCapacity();
    }

    fn clearBodies(self: *Seen) void {
        for (self.bodies.items) |body| {
            self.release(body.code);
            self.release(body.output);
        }
        self.bodies.clearRetainingCapacity();
    }

    fn release(self: *Seen, text: []u8) void {
        if (text.ptr == &empty_code) return;
        self.gpa.free(text);
    }

    pub fn finished(self: *Seen, arena: std.mem.Allocator, result: mei_process.Result, at: u64) ![]write.Entry {
        const ending: part.Ending = switch (result) {
            .ok => blk: {
                self.rest = .done;
                break :blk .Done;
            },
            .err => blk: {
                self.rest = .failed;
                break :blk .Limit;
            },
        };
        const answer: ?[]const u8 = switch (result) {
            .ok => |s| s,
            .err => |s| s,
        };
        const took_s = if (self.since) |since| if (at >= since) (at - since) / 1000 else null else null;
        self.run = false;
        self.since = null;
        self.calling = false;
        var entries: std.ArrayList(write.Entry) = .empty;
        const flushed = try self.commitDraft(arena, at);
        try entries.appendSlice(arena, flushed);
        if (answer) |a| {
            if (a.len > 0) {
                if (ending == .Done and !std.mem.eql(u8, a, self.last_text)) {
                    try entries.append(arena, .{ .reply = try arena.dupe(u8, a) });
                } else if (ending == .Limit) {
                    try entries.append(arena, .{ .note = try arena.dupe(u8, a) });
                }
            }
        }
        try entries.append(arena, .{ .ended = .{ .at = tod(at), .ending = ending, .took_s = took_s } });
        return try entries.toOwnedSlice(arena);
    }

    pub fn cancelled(self: *Seen, arena: std.mem.Allocator, at: u64) ![]write.Entry {
        self.rest = .idle;
        const took_s = if (self.since) |since| if (at >= since) (at - since) / 1000 else null else null;
        self.run = false;
        self.since = null;
        self.calling = false;
        var entries: std.ArrayList(write.Entry) = .empty;
        const flushed = try self.commitDraft(arena, at);
        try entries.appendSlice(arena, flushed);
        try entries.append(arena, .{ .ended = .{ .at = tod(at), .ending = .Cancelled, .took_s = took_s } });
        return try entries.toOwnedSlice(arena);
    }

    fn setLast(self: *Seen, text: []const u8) void {
        self.replace(&self.last_text, text);
    }
};

pub fn noteEntries(arena: std.mem.Allocator, said: []const u8) ![]write.Entry {
    const entries = try arena.alloc(write.Entry, 1);
    entries[0] = .{ .note = try arena.dupe(u8, said) };
    return entries;
}

pub fn tod(at: u64) ?u32 {
    return if (local_time.local(at)) |t| t.seconds else null;
}

test "mei.obs pulse model route usage never become notes" {
    const gpa = std.testing.allocator;
    var seen = try Seen.init(gpa, "auto");
    defer seen.deinit();
    var arena_state = std.heap.ArenaAllocator.init(gpa);
    defer arena_state.deinit();
    const arena = arena_state.allocator();
    const lines = [_][]const u8{
        "{\"type\":\"mei.obs\",\"kind\":\"pulse\"}",
        "{\"type\":\"mei.obs\",\"kind\":\"model\",\"model\":\"m\",\"context_tokens\":100}",
        "{\"type\":\"mei.obs\",\"kind\":\"route\",\"available\":true,\"model\":\"m\",\"effort\":\"high\"}",
        "{\"type\":\"mei.obs\",\"kind\":\"usage\",\"usage\":{\"input_tokens\":1,\"cache_creation_input_tokens\":0,\"cache_read_input_tokens\":0}}",
    };
    for (lines) |raw| {
        const entries = try seen.line(arena, raw, 1);
        try std.testing.expectEqual(@as(usize, 0), entries.len);
    }
}

test "observations drive actual effort step and two-decimal context" {
    const gpa = std.testing.allocator;
    var seen = try Seen.init(gpa, "auto");
    defer seen.deinit();
    var arena_state = std.heap.ArenaAllocator.init(gpa);
    defer arena_state.deinit();
    const arena = arena_state.allocator();
    var offer_buf: [512]u8 = undefined;
    _ = try seen.line(arena, "{\"type\":\"mei.obs\",\"kind\":\"model\",\"model\":\"group/plan-first\",\"context_tokens\":1048576,\"efforts\":[\"none\",\"low\",\"high\",\"max\"],\"effort\":\"high\"}", 1);
    _ = try seen.line(arena, "{\"type\":\"mei.obs\",\"kind\":\"route\",\"available\":true,\"model\":\"provider/actual\",\"effort\":\"high\"}", 2);
    _ = try seen.line(arena, "{\"type\":\"mei.obs\",\"kind\":\"usage\",\"usage\":{\"input_tokens\":231,\"cache_creation_input_tokens\":0,\"cache_read_input_tokens\":256}}", 3);
    const offer = seen.offer(&offer_buf);
    try std.testing.expect(std.mem.indexOf(u8, offer, "provider/actual") != null);
    try std.testing.expect(std.mem.indexOf(u8, offer, "effort high") != null);
    try std.testing.expect(std.mem.indexOf(u8, offer, "Agent Step 1") != null);
    try std.testing.expect(std.mem.indexOf(u8, offer, "0.05%") != null);
    try std.testing.expect(std.mem.indexOf(u8, offer, "487/1048576") != null);
}

test "image results never print base64" {
    const gpa = std.testing.allocator;
    var seen = try Seen.init(gpa, "model");
    defer seen.deinit();
    var arena_state = std.heap.ArenaAllocator.init(gpa);
    defer arena_state.deinit();
    const entries = try seen.line(arena_state.allocator(), "{\"role\":\"user\",\"content\":[{\"type\":\"tool_result\",\"tool_use_id\":\"a\",\"content\":[{\"type\":\"image\",\"source\":{\"data\":\"secret-base64\"}}]}]}", 1);
    try std.testing.expectEqual(@as(usize, 1), entries.len);
    try std.testing.expectEqualStrings("python", entries[0].tool.name);
    try std.testing.expectEqual(@as(usize, 1), seen.bodies.items.len);
    try std.testing.expectEqualStrings("[image attached]", seen.bodies.items[0].output);
    try std.testing.expect(std.mem.indexOf(u8, seen.bodies.items[0].output, "secret") == null);
}

test "a user prompt is shown once and a python call keeps code output and elapsed time" {
    const gpa = std.testing.allocator;
    var seen = try Seen.init(gpa, "model");
    defer seen.deinit();
    var arena_state = std.heap.ArenaAllocator.init(gpa);
    defer arena_state.deinit();
    const arena = arena_state.allocator();
    seen.suppressEcho("hello");
    const echo = try seen.line(arena, "{\"role\":\"user\",\"content\":\"hello\"}", 1);
    try std.testing.expectEqual(@as(usize, 0), echo.len);
    const again = try seen.line(arena, "{\"role\":\"user\",\"content\":\"再次\"}", 2);
    try std.testing.expectEqualStrings("再次", again[0].you.said);
    const assistant = try seen.line(arena, "{\"role\":\"assistant\",\"content\":[{\"type\":\"text\",\"text\":\"running\"},{\"type\":\"tool_use\",\"id\":\"a\",\"name\":\"python\",\"input\":{\"code\":\"print(1)\"}}]}", 1000);
    try std.testing.expectEqualStrings("running", assistant[assistant.len - 1].reply);
    try std.testing.expect(seen.calling);
    const result = try seen.line(arena, "{\"role\":\"user\",\"content\":[{\"type\":\"tool_result\",\"tool_use_id\":\"a\",\"content\":\"1\\n\"}]}", 2500);
    try std.testing.expectEqual(@as(usize, 1), result.len);
    try std.testing.expectEqual(@as(?u64, 1_500_000), result[0].tool.took_us);
    try std.testing.expectEqualStrings("print(1)", seen.bodies.items[seen.bodies.items.len - 1].code);
    try std.testing.expectEqualStrings("1\n", seen.bodies.items[seen.bodies.items.len - 1].output);
}

test "stream deltas stay in the draft until commit" {
    const gpa = std.testing.allocator;
    var seen = try Seen.init(gpa, "model");
    defer seen.deinit();
    seen.begin(10);
    var arena_state = std.heap.ArenaAllocator.init(gpa);
    defer arena_state.deinit();
    const arena = arena_state.allocator();
    const first = try seen.line(arena, "{\"type\":\"mei.stream\",\"kind\":\"delta\",\"text\":\"Hel\"}", 11);
    try std.testing.expectEqual(@as(usize, 0), first.len);
    try std.testing.expectEqualStrings("Hel", seen.draftText());
    const second = try seen.line(arena, "{\"type\":\"mei.stream\",\"kind\":\"delta\",\"text\":\"lo\"}", 12);
    try std.testing.expectEqual(@as(usize, 0), second.len);
    try std.testing.expectEqualStrings("Hello", seen.draftText());
    const committed = try seen.line(arena, "{\"type\":\"mei.stream\",\"kind\":\"commit\"}", 13);
    try std.testing.expect(committed.len >= 1);
    try std.testing.expectEqualStrings("Hello", committed[committed.len - 1].reply);
    try std.testing.expectEqualStrings("", seen.draftText());
}

test "a complete assistant line drops an open draft" {
    const gpa = std.testing.allocator;
    var seen = try Seen.init(gpa, "model");
    defer seen.deinit();
    seen.begin(1);
    var arena_state = std.heap.ArenaAllocator.init(gpa);
    defer arena_state.deinit();
    const arena = arena_state.allocator();
    _ = try seen.line(arena, "{\"type\":\"mei.stream\",\"kind\":\"delta\",\"text\":\"partial\"}", 2);
    try std.testing.expectEqualStrings("partial", seen.draftText());
    const entries = try seen.line(arena, "{\"role\":\"assistant\",\"content\":\"final\"}", 3);
    try std.testing.expectEqualStrings("", seen.draftText());
    try std.testing.expectEqualStrings("final", entries[entries.len - 1].reply);
}

test "head facts keep a copy of the model after it is replaced" {
    const gpa = std.testing.allocator;
    var seen = try Seen.init(gpa, "provider/old");
    defer seen.deinit();
    seen.begin(1);
    var arena_state = std.heap.ArenaAllocator.init(gpa);
    defer arena_state.deinit();
    const arena = arena_state.allocator();
    const entries = try seen.line(arena, "{\"role\":\"assistant\",\"content\":\"hi\"}", 2);
    try std.testing.expect(entries.len >= 2);
    try std.testing.expectEqualStrings("provider/old", entries[0].head.facts[0]);
    _ = try seen.line(arena, "{\"type\":\"mei.obs\",\"kind\":\"model\",\"model\":\"provider/new\"}", 3);
    try std.testing.expectEqualStrings("provider/old", entries[0].head.facts[0]);
    try std.testing.expectEqualStrings("provider/new", seen.model);
}
