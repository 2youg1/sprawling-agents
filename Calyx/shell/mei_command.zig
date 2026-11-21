// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

const std = @import("std");

pub const Command = union(enum) {
    task,
    new_session,
    resume_session: ?[]const u8,
    change_effort,
    quit,
    unknown,
};

fn resumed(typed: []const u8, long: []const u8, short: []const u8) ?Command {
    if (std.mem.eql(u8, typed, long) or std.mem.eql(u8, typed, short)) {
        return .{ .resume_session = null };
    }
    for ([_][]const u8{ long, short }) |verb| {
        if (std.mem.startsWith(u8, typed, verb) and typed.len > verb.len and typed[verb.len] == ' ') {
            const path = std.mem.trim(u8, typed[verb.len + 1 ..], &std.ascii.whitespace);
            return .{ .resume_session = if (path.len == 0) null else path };
        }
    }
    return null;
}

pub fn parse(typed: []const u8) Command {
    if (std.mem.eql(u8, typed, "/new") or std.mem.eql(u8, typed, "/n")) return .new_session;
    if (resumed(typed, "/resume", "/r")) |command| return command;
    if (std.mem.eql(u8, typed, "/e") or std.mem.eql(u8, typed, "/effect")) return .change_effort;
    if (std.mem.eql(u8, typed, "/quit") or std.mem.eql(u8, typed, "/q")) return .quit;
    if (std.mem.startsWith(u8, typed, "/")) return .unknown;
    return .task;
}

test "Mei commands include aliases and an optional resume path" {
    try std.testing.expect(parse("hello") == .task);
    try std.testing.expect(parse("/new") == .new_session);
    try std.testing.expect(parse("/n") == .new_session);
    try std.testing.expect(parse("/e") == .change_effort);
    try std.testing.expect(parse("/effect") == .change_effort);
    try std.testing.expect(parse("/quit") == .quit);
    try std.testing.expect(parse("/q") == .quit);
    try std.testing.expect(parse("/resume").resume_session == null);
    try std.testing.expect(parse("/r").resume_session == null);
    try std.testing.expectEqualStrings("old.jsonl", parse("/resume old.jsonl").resume_session.?);
    try std.testing.expectEqualStrings("old.jsonl", parse("/r old.jsonl").resume_session.?);
    try std.testing.expect(parse("/m") == .unknown);
}
