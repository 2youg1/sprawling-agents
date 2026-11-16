// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

const std = @import("std");

const off_the_socket = [_][]const u8{ "PutSecret", "Auth" };

pub const command_names = [_][]const u8{
    "Dispatch",          "ProbeEndpoint",   "ConfigureBuilding",  "AttachEndpoint",
    "SelectModel",       "OpenSession",     "CreateBuilding",     "RemoveBuilding",
    "PutSecret",         "Steer",           "Cancel",             "Halt",
    "Reveal",            "RestoreDiscard",  "DoctorInstall",      "DoctorRefresh",
    "Release",           "BatchByBuilding", "Approve",            "HandOff",
    "SetAutonomy",       "Pursue",          "Wake",               "PutDocument",
    "PutIdentity",       "PutRules",        "SetWorkspaceChoice", "RestoreFile",
    "PutGuide",          "ConfigureCity",   "PutSpine",           "PutRange",
    "DecideProposals",   "ConnectToolkit",  "PutPreferences",     "PutShelved",
    "NameSession",       "ChangeRunPolicy", "OpenRemoteDoor",     "ReplaceCityKey",
    "ConfirmRemoteDoor", "CloseRemoteDoor", "PrivacyOperation",   "ForgetSecret",
    "CloseCity",         "AddAgent",        "AgentLogin",         "ForgetDevice",
    "PutPrompt",         "Auth",
};

pub const query_names = [_][]const u8{
    "History",         "RunHistory",    "HistoryRange",  "Sessions",
    "Changes",         "Hunks",         "Commit",        "RunView",
    "CityView",        "ApprovalQueue", "InboxView",     "Metrics",
    "CostView",        "ArchiveSearch", "RegistryView",  "DiscardView",
    "EndpointView",    "KnownHosts",    "AgentCatalog",  "ParseAgentSpec",
    "Devices",         "BuildingView",  "Identity",      "Automation",
    "GithubLogin",     "Guide",         "Governance",    "Rounds",
    "Evidence",        "CostOf",        "Listing",       "Find",
    "Document",        "Proposals",     "OpenProposals", "Range",
    "Versions",        "Bytes",         "Export",        "Preview",
    "Reply",           "Commits",       "Doctor",        "Prefix",
    "PromptShape",     "Content",       "Skills",        "Template",
    "GitStatus",       "McpHealth",     "Toolkits",      "NewestRelease",
    "Privacy",         "Preferences",   "Config",        "RunCosts",
    "UpstreamVersion", "SkillUsage",    "McpUsage",      "UsageExport",
    "Shells",          "ModelFigures",  "Layers",        "Catalog",
};

pub const Offered = enum {
    cli,
    web_ui,
    both,

    pub fn inTheCli(self: Offered) bool {
        return switch (self) {
            .cli, .both => true,
            .web_ui => false,
        };
    }
};

pub const Slash = enum {
    help,
    room,
    new,
    stop,
    halt,
    release,
    model,
    effort,
    approve,
    deny,
    web,
    quit,
    serving,
    remote,
    acp,
    wire,

    pub const all = [_]Slash{
        .help,    .room, .new, .stop, .halt,    .release, .model, .effort,
        .approve, .deny, .web, .quit, .serving, .remote,  .acp,   .wire,
    };

    pub fn spelling(self: Slash) []const u8 {
        return switch (self) {
            .help => "/help",
            .room => "/room",
            .new => "/new",
            .stop => "/stop",
            .halt => "/halt",
            .release => "/release",
            .model => "/model",
            .effort => "/effort",
            .approve => "/approve",
            .deny => "/deny",
            .web => "/web",
            .quit => "/quit",
            .serving => "/serving",
            .remote => "/remote",
            .acp => "/acp",
            .wire => "/wire",
        };
    }

    pub fn takes(self: Slash) []const u8 {
        return switch (self) {
            .help, .new, .stop, .web, .quit, .serving => "",
            .room => "<addr>",
            .halt, .release => "[--all]",
            .model, .approve, .deny => "[<id>]",
            .effort => "[<level>]",
            .remote => "<verb>",
            .acp => "[<text>]",
            .wire => "<verb> [<json>]",
        };
    }

    pub fn offered(self: Slash) Offered {
        return switch (self) {
            .help, .room, .new, .stop, .halt, .release, .model, .effort, .quit, .acp => .both,
            .approve, .deny, .web, .serving, .remote, .wire => .cli,
        };
    }

    pub fn says(self: Slash) []const u8 {
        return switch (self) {
            .help => "list these verbs",
            .room => "choose the room plain lines go to",
            .new => "start a new session in this room",
            .stop => "stop the run working in this room",
            .halt => "hold this room's building, or with --all the whole city",
            .release => "let this room's building work again, or with --all the whole city",
            .model => "show the model, or choose one the city's endpoints offer",
            .effort => "show the thinking level, or choose one the model offers",
            .approve => "approve the waiting request, or the one with this id",
            .deny => "deny the waiting request, or the one with this id",
            .web => "open the WebUI and quiet this terminal; Esc comes back",
            .quit => "close the city",
            .serving => "where this city listens, and what is running in it",
            .remote => "open|pair|close|devices|revoke: the door a device reaches the city through",
            .acp => "find an ACP agent, or read a pasted launch line",
            .wire => "send any wire command or question, its body as JSON",
        };
    }

    pub fn parse(word: []const u8) ?Slash {
        const bare = if (std.mem.startsWith(u8, word, "/")) word[1..] else word;
        for (all) |verb| {
            const spelling_bare = verb.spelling()[1..];
            if (std.mem.eql(u8, spelling_bare, bare)) return verb;
        }
        return null;
    }
};

pub const Reach = enum { building, city };

pub const Address = struct {
    bytes: []const u8,

    pub fn parse(raw: []const u8) error{InvalidAddress}!Address {
        if (raw.len == 0) return error.InvalidAddress;
        if (std.mem.startsWith(u8, raw, "/")) return error.InvalidAddress;
        if (std.mem.indexOfScalar(u8, raw, '\\') != null) return error.InvalidAddress;
        if (std.mem.indexOfScalar(u8, raw, ':') != null) return error.InvalidAddress;
        for (raw) |b| if (std.ascii.isControl(b)) return error.InvalidAddress;
        var parts = std.mem.splitScalar(u8, raw, '/');
        while (parts.next()) |segment| {
            if (segment.len == 0) return error.InvalidAddress;
            if (std.mem.eql(u8, segment, ".") or std.mem.eql(u8, segment, "..")) return error.InvalidAddress;
            if (segment[segment.len - 1] == '.') return error.InvalidAddress;
            if (std.ascii.isWhitespace(segment[segment.len - 1])) return error.InvalidAddress;
        }
        return .{ .bytes = raw };
    }

    pub fn asStr(self: Address) []const u8 {
        return self.bytes;
    }
};

pub const Frame = union(enum) {
    command: struct { name: []const u8, body: []u8 },
    ask: struct { query: []u8 },
};

pub const Line = union(enum) {
    nothing,
    help,
    open_web,
    serving,
    quit,
    select: Address,
    new,
    stop,
    halt: Reach,
    release: Reach,
    model: ?[]const u8,
    effort: ?[]const u8,
    approve: ?[]const u8,
    deny: ?[]const u8,
    acp: ?[]const u8,
    remote: []const u8,
    frame: Frame,
    work: []const u8,
    unknown: struct { verb: []const u8, nearest: []const []const u8 },
    malformed: struct { verb: []const u8, reason: []const u8 },
};

pub fn carriedCommands(out: [][]const u8) [][]const u8 {
    var n: usize = 0;
    for (command_names) |name| {
        var skip = false;
        for (off_the_socket) |blocked| {
            if (std.mem.eql(u8, name, blocked)) {
                skip = true;
                break;
            }
        }
        if (!skip) {
            out[n] = name;
            n += 1;
        }
    }
    return out[0..n];
}

pub fn snake(allocator: std.mem.Allocator, camel: []const u8) ![]u8 {
    var out: std.ArrayList(u8) = .empty;
    errdefer out.deinit(allocator);
    for (camel, 0..) |ch, index| {
        if (std.ascii.isUpper(ch) and index > 0) try out.append(allocator, '_');
        try out.append(allocator, std.ascii.toLower(ch));
    }
    return try out.toOwnedSlice(allocator);
}

pub fn slashVerbs(out: []Slash) []Slash {
    var n: usize = 0;
    for (Slash.all) |verb| {
        if (verb.offered().inTheCli()) {
            out[n] = verb;
            n += 1;
        }
    }
    return out[0..n];
}

fn argument(tail: []const u8) ?[]const u8 {
    if (tail.len == 0) return null;
    return tail;
}

fn splitVerb(rest: []const u8) struct { []const u8, []const u8 } {
    for (rest, 0..) |ch, i| {
        if (std.ascii.isWhitespace(ch)) {
            return .{ rest[0..i], std.mem.trim(u8, rest[i..], &std.ascii.whitespace) };
        }
    }
    return .{ rest, "" };
}

fn nearest(allocator: std.mem.Allocator, typed: []const u8, known: []const []const u8) ![]const []const u8 {
    const max_len = @min(typed.len, 4);
    if (max_len == 0) return &.{};
    var length = max_len;
    while (length >= 1) : (length -= 1) {
        const head = typed[0..length];
        var close: std.ArrayList([]const u8) = .empty;
        errdefer {
            for (close.items) |owned| allocator.free(owned);
            close.deinit(allocator);
        }
        for (known) |name| {
            if (std.mem.startsWith(u8, name, head)) {
                try close.append(allocator, try allocator.dupe(u8, name));
            }
        }
        if (close.items.len != 0) {
            if (close.items.len > 6) {
                var i: usize = 6;
                while (i < close.items.len) : (i += 1) allocator.free(close.items[i]);
                close.shrinkRetainingCapacity(6);
            }
            return try close.toOwnedSlice(allocator);
        }
        close.deinit(allocator);
        if (length == 1) break;
    }
    return try allocator.alloc([]const u8, 0);
}

pub fn parse(allocator: std.mem.Allocator, line: []const u8, idem: []const u8) !Line {
    const trimmed = std.mem.trim(u8, line, &std.ascii.whitespace);
    if (trimmed.len == 0) return .nothing;
    if (!std.mem.startsWith(u8, trimmed, "/")) {
        return .{ .work = try allocator.dupe(u8, trimmed) };
    }
    const rest = trimmed[1..];
    const verb_tail = splitVerb(rest);
    const verb = verb_tail[0];
    const tail = verb_tail[1];
    const slash = Slash.parse(verb);
    if (slash == null or !slash.?.offered().inTheCli()) {
        var buf: [Slash.all.len]Slash = undefined;
        const verbs = slashVerbs(&buf);
        var known_buf: [Slash.all.len][]const u8 = undefined;
        var k: usize = 0;
        for (verbs) |v| {
            known_buf[k] = v.spelling()[1..];
            k += 1;
        }
        return .{ .unknown = .{
            .verb = try allocator.dupe(u8, verb),
            .nearest = try nearest(allocator, verb, known_buf[0..k]),
        } };
    }
    const reach: Reach = if (std.mem.eql(u8, tail, "--all")) .city else .building;
    return switch (slash.?) {
        .help => .help,
        .web => .open_web,
        .serving => .serving,
        .quit => .quit,
        .new => .new,
        .stop => .stop,
        .halt => .{ .halt = reach },
        .release => .{ .release = reach },
        .model => .{ .model = if (argument(tail)) |a| try allocator.dupe(u8, a) else null },
        .effort => .{ .effort = if (argument(tail)) |a| try allocator.dupe(u8, a) else null },
        .approve => .{ .approve = if (argument(tail)) |a| try allocator.dupe(u8, a) else null },
        .deny => .{ .deny = if (argument(tail)) |a| try allocator.dupe(u8, a) else null },
        .acp => .{ .acp = if (argument(tail)) |a| try allocator.dupe(u8, a) else null },
        .remote => .{ .remote = try allocator.dupe(u8, tail) },
        .room => blk: {
            const addr = Address.parse(tail) catch {
                const hint = try allocator.dupe(u8, "room <building>/<room>");
                const nearest_list = try allocator.alloc([]const u8, 1);
                nearest_list[0] = hint;
                break :blk Line{ .unknown = .{
                    .verb = try std.fmt.allocPrint(allocator, "room {s}", .{tail}),
                    .nearest = nearest_list,
                } };
            };
            break :blk .{ .select = .{ .bytes = try allocator.dupe(u8, addr.bytes) } };
        },
        .wire => blk: {
            const parts = splitVerb(tail);
            break :blk try wireFrame(allocator, parts[0], parts[1], idem);
        },
    };
}

fn wireFrame(allocator: std.mem.Allocator, verb: []const u8, tail: []const u8, idem: []const u8) !Line {
    var carried_buf: [command_names.len][]const u8 = undefined;
    const carried = carriedCommands(&carried_buf);
    var known_command: ?[]const u8 = null;
    for (carried) |name| {
        const snaked = try snake(allocator, name);
        defer allocator.free(snaked);
        if (std.mem.eql(u8, snaked, verb)) {
            known_command = name;
            break;
        }
    }
    var known_query: ?[]const u8 = null;
    for (query_names) |name| {
        const snaked = try snake(allocator, name);
        defer allocator.free(snaked);
        if (std.mem.eql(u8, snaked, verb)) {
            known_query = name;
            break;
        }
    }
    if (known_command == null and known_query == null) {
        var verbs_buf: [command_names.len + query_names.len][]u8 = undefined;
        var n: usize = 0;
        for (carried) |name| {
            verbs_buf[n] = try snake(allocator, name);
            n += 1;
        }
        for (query_names) |name| {
            verbs_buf[n] = try snake(allocator, name);
            n += 1;
        }
        defer for (verbs_buf[0..n]) |s| allocator.free(s);
        var known: [command_names.len + query_names.len][]const u8 = undefined;
        for (verbs_buf[0..n], 0..) |s, i| known[i] = s;
        return .{ .unknown = .{
            .verb = try std.fmt.allocPrint(allocator, "wire {s}", .{verb}),
            .nearest = try nearest(allocator, verb, known[0..n]),
        } };
    }
    if (tail.len != 0) {
        var parsed = std.json.parseFromSlice(std.json.Value, allocator, tail, .{}) catch |err| {
            return .{ .malformed = .{
                .verb = try allocator.dupe(u8, verb),
                .reason = try std.fmt.allocPrint(allocator, "{s}", .{@errorName(err)}),
            } };
        };
        defer parsed.deinit();
    }
    if (known_command) |name| {
        const snaked = try snake(allocator, name);
        errdefer allocator.free(snaked);
        const body = if (tail.len == 0)
            try allocator.dupe(u8, "null")
        else blk: {
            var parsed = try std.json.parseFromSlice(std.json.Value, allocator, tail, .{});
            defer parsed.deinit();
            if (parsed.value == .object) {
                const gop = try parsed.value.object.getOrPut(allocator, "idem");
                if (!gop.found_existing) {
                    gop.value_ptr.* = .{ .string = idem };
                }
            }
            break :blk try std.json.Stringify.valueAlloc(allocator, parsed.value, .{});
        };
        return .{ .frame = .{ .command = .{ .name = snaked, .body = body } } };
    }
    const qname = try snake(allocator, known_query.?);
    errdefer allocator.free(qname);
    const query_json = if (tail.len == 0)
        try std.fmt.allocPrint(allocator, "\"{s}\"", .{qname})
    else blk: {
        const nested = try std.fmt.allocPrint(allocator, "{{\"{s}\":{s}}}", .{ qname, tail });
        allocator.free(qname);
        break :blk nested;
    };
    if (tail.len == 0) allocator.free(qname);
    const ask = try std.fmt.allocPrint(allocator, "{{\"ask_id\":0,\"query\":{s}}}", .{query_json});
    allocator.free(query_json);
    return .{ .frame = .{ .ask = .{ .query = ask } } };
}

test "snake spelling" {
    const a = try snake(std.testing.allocator, "AttachEndpoint");
    defer std.testing.allocator.free(a);
    try std.testing.expectEqualStrings("attach_endpoint", a);
    const b = try snake(std.testing.allocator, "Fork");
    defer std.testing.allocator.free(b);
    try std.testing.expectEqualStrings("fork", b);
}

test "slash verbs are the tables" {
    const help = try parse(std.testing.allocator, "/help", "k");
    try std.testing.expect(help == .help);
    const web = try parse(std.testing.allocator, "/web", "k");
    try std.testing.expect(web == .open_web);
    const work = try parse(std.testing.allocator, "  measure the beam  ", "k");
    defer std.testing.allocator.free(work.work);
    try std.testing.expectEqualStrings("measure the beam", work.work);
    const empty = try parse(std.testing.allocator, "   ", "k");
    try std.testing.expect(empty == .nothing);
}

test "room select and unknown wire verb" {
    const room = try parse(std.testing.allocator, "/room lab/room1", "k");
    defer std.testing.allocator.free(room.select.bytes);
    try std.testing.expectEqualStrings("lab/room1", room.select.bytes);
    const unknown = try parse(std.testing.allocator, "/wire carn", "k");
    defer {
        std.testing.allocator.free(unknown.unknown.verb);
        for (unknown.unknown.nearest) |n| std.testing.allocator.free(n);
        std.testing.allocator.free(unknown.unknown.nearest);
    }
    try std.testing.expectEqualStrings("wire carn", unknown.unknown.verb);
    var found = false;
    for (unknown.unknown.nearest) |n| {
        if (std.mem.eql(u8, n, "cancel")) found = true;
    }
    try std.testing.expect(found);
}
