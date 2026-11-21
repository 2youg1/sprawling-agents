// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

const std = @import("std");

pub fn next(current: []const u8, levels: []const []const u8) ?[]const u8 {
    if (levels.len == 0) return null;
    for (levels, 0..) |level, index| {
        if (std.mem.eql(u8, level, current)) return levels[(index + 1) % levels.len];
    }
    return levels[0];
}

pub fn updatedExtra(gpa: std.mem.Allocator, existing: ?[]const u8, effort: []const u8) ![]u8 {
    var arena_state = std.heap.ArenaAllocator.init(gpa);
    defer arena_state.deinit();
    const arena = arena_state.allocator();
    var parsed = try std.json.parseFromSlice(std.json.Value, arena, existing orelse "{}", .{});
    defer parsed.deinit();
    if (parsed.value != .object) return error.InvalidExtra;
    const thinking_type = if (std.mem.eql(u8, effort, "none")) "disabled" else "adaptive";
    var thinking: std.json.ObjectMap = .empty;
    try thinking.put(arena, "type", .{ .string = thinking_type });
    try parsed.value.object.put(arena, "thinking", .{ .object = thinking });
    if (parsed.value.object.getPtr("output_config")) |output| {
        if (output.* != .object) return error.InvalidExtra;
        try output.object.put(arena, "effort", .{ .string = effort });
    } else {
        var output: std.json.ObjectMap = .empty;
        try output.put(arena, "effort", .{ .string = effort });
        try parsed.value.object.put(arena, "output_config", .{ .object = output });
    }
    return std.json.Stringify.valueAlloc(gpa, parsed.value, .{});
}

test "effort cycles only through Magpie's offered levels" {
    const levels = [_][]const u8{ "none", "low", "high", "max" };
    try std.testing.expectEqualStrings("max", next("high", &levels).?);
    try std.testing.expectEqualStrings("none", next("max", &levels).?);
    try std.testing.expectEqualStrings("none", next("unknown", &levels).?);
    try std.testing.expect(next("high", &.{}) == null);
}

test "switching effort preserves the other request fields" {
    const gpa = std.testing.allocator;
    const encoded = try updatedExtra(gpa, "{\"max_tokens\":123,\"output_config\":{\"format\":{\"type\":\"text\"}}}", "max");
    defer gpa.free(encoded);
    var parsed = try std.json.parseFromSlice(std.json.Value, gpa, encoded, .{});
    defer parsed.deinit();
    try std.testing.expectEqual(@as(i64, 123), parsed.value.object.get("max_tokens").?.integer);
    try std.testing.expectEqualStrings("adaptive", parsed.value.object.get("thinking").?.object.get("type").?.string);
    const output = parsed.value.object.get("output_config").?.object;
    try std.testing.expectEqualStrings("max", output.get("effort").?.string);
    try std.testing.expect(output.get("format") != null);
}

test "none disables thinking" {
    const gpa = std.testing.allocator;
    const encoded = try updatedExtra(gpa, null, "none");
    defer gpa.free(encoded);
    var parsed = try std.json.parseFromSlice(std.json.Value, gpa, encoded, .{});
    defer parsed.deinit();
    try std.testing.expectEqualStrings("disabled", parsed.value.object.get("thinking").?.object.get("type").?.string);
}
