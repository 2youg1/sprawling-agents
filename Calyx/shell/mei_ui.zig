// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

const std = @import("std");
const leaf = @import("leaf");
const write = leaf.write;
const part = leaf.part;
const argv = @import("argv.zig");
const mei_path = @import("mei_path.zig");
const editor_mod = @import("editor.zig");
const local_time = @import("local_time.zig");
const program_status = @import("program_status.zig");
const screen_mod = @import("screen.zig");
const termio = @import("termio.zig");
const mei_process = @import("mei_process.zig");
const mei_session = @import("mei_session.zig");

const replay_batch: usize = 64;

pub fn run(init: std.process.Init) u8 {
    const gpa = init.gpa;
    const io = init.io;

    var arg_list: std.ArrayList([]const u8) = .empty;
    defer arg_list.deinit(gpa);
    var args_it = std.process.Args.Iterator.initAllocator(init.minimal.args, gpa) catch return 1;
    defer args_it.deinit();
    while (args_it.next()) |a| {
        arg_list.append(gpa, a) catch return 1;
    }

    const replay = switch (argv.replayFlag(arg_list.items)) {
        .none => null,
        .missing_value => {
            std.debug.print("missing value for --replay\n", .{});
            return 1;
        },
        .path => |path| path,
    };

    var ui = Ui.init(gpa, io, init.environ_map, replay) catch return 1;
    defer ui.deinit();
    return ui.loop();
}

const Seen = struct {
    gpa: std.mem.Allocator,
    model: []u8,
    usage: ?[]u8 = null,
    since: ?u64 = null,
    calling: bool = false,
    rest: program_status.Rest = .idle,
    headed: bool = false,
    last_text: []u8 = "",
    run: bool = false,

    fn init(gpa: std.mem.Allocator, model: []const u8) !Seen {
        return .{
            .gpa = gpa,
            .model = try gpa.dupe(u8, model),
            .last_text = try gpa.dupe(u8, ""),
        };
    }

    fn deinit(self: *Seen) void {
        self.gpa.free(self.model);
        if (self.usage) |u| self.gpa.free(u);
        self.gpa.free(self.last_text);
        self.* = undefined;
    }

    fn begin(self: *Seen, at: u64) void {
        self.run = true;
        self.since = at;
        self.calling = false;
        self.rest = .idle;
        self.headed = false;
        self.gpa.free(self.last_text);
        self.last_text = self.gpa.dupe(u8, "") catch "";
    }

    fn counts(self: *const Seen) program_status.Counts {
        return .{ .runs = if (self.run) 1 else 0, .approvals = 0 };
    }

    fn offer(self: *Seen, buf: []u8) []const u8 {
        const usage = self.usage orelse "unknown";
        if (self.model.len == 0) return usage;
        return std.fmt.bufPrint(buf, "{s} · {s}", .{ self.model, usage }) catch usage;
    }

    fn line(self: *Seen, arena: std.mem.Allocator, raw: []const u8, at: u64) ![]write.Entry {
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
        }
        const role = value.object.get("role") orelse return try noteEntries(arena, raw);
        const content = value.object.get("content") orelse return try noteEntries(arena, raw);
        if (role != .string) return try noteEntries(arena, raw);
        if (std.mem.eql(u8, role.string, "assistant")) return try self.assistant(arena, content, at);
        if (std.mem.eql(u8, role.string, "user")) return try self.toolResults(arena, content, at);
        return try noteEntries(arena, raw);
    }

    fn observation(self: *Seen, arena: std.mem.Allocator, value: std.json.Value) ![]write.Entry {
        _ = arena;
        const kind = if (value.object.get("kind")) |k| (if (k == .string) k.string else "") else "";
        if (std.mem.eql(u8, kind, "usage")) {
            if (value.object.get("usage")) |usage| {
                const text = std.json.Stringify.valueAlloc(self.gpa, usage, .{}) catch null;
                if (text) |t| {
                    if (self.usage) |old| self.gpa.free(old);
                    self.usage = t;
                }
            }
        }
        // Spec D5: pulse/usage obs are swallowed — no Note entries.
        return &.{};
    }

    fn assistant(self: *Seen, arena: std.mem.Allocator, content: std.json.Value, at: u64) ![]write.Entry {
        var entries: std.ArrayList(write.Entry) = .empty;
        if (!self.headed) {
            self.headed = true;
            const facts: []const []const u8 = if (self.model.len == 0) &.{} else &.{self.model};
            try entries.append(arena, .{ .head = .{
                .at = tod(at),
                .resident = "Mei",
                .facts = try arena.dupe([]const u8, facts),
            } });
        }
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
                self.calling = true;
            }
        }
        if (reasoning > 0) try entries.append(arena, .{ .reasoning = reasoning });
        const said = try text.toOwnedSlice(arena);
        self.setLast(said);
        if (said.len > 0) try entries.append(arena, .{ .reply = said });
        return try entries.toOwnedSlice(arena);
    }

    fn toolResults(self: *Seen, arena: std.mem.Allocator, content: std.json.Value, at: u64) ![]write.Entry {
        var entries: std.ArrayList(write.Entry) = .empty;
        if (content == .array) {
            for (content.array.items) |block| {
                if (block != .object) continue;
                const ty = if (block.object.get("type")) |t| (if (t == .string) t.string else "") else "";
                if (!std.mem.eql(u8, ty, "tool_result")) continue;
                try entries.append(arena, .{ .tool = .{
                    .at = tod(at),
                    .name = "python",
                    .subject = "",
                    .took_us = null,
                    .outcome = .Answered,
                } });
                if (block.object.get("content")) |c| {
                    const text = switch (c) {
                        .string => |s| try arena.dupe(u8, s),
                        else => try std.json.Stringify.valueAlloc(arena, c, .{}),
                    };
                    if (text.len > 0) try entries.append(arena, .{ .note = text });
                }
            }
        }
        self.calling = false;
        return try entries.toOwnedSlice(arena);
    }

    fn finished(self: *Seen, arena: std.mem.Allocator, result: mei_process.Result, at: u64) ![]write.Entry {
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
        const took_s = if (self.since) |since|
            if (at >= since) (at - since) / 1000 else null
        else
            null;
        self.run = false;
        self.since = null;
        self.calling = false;

        var entries: std.ArrayList(write.Entry) = .empty;
        if (answer) |a| {
            if (a.len > 0) {
                if (ending == .Done and !std.mem.eql(u8, a, self.last_text)) {
                    try entries.append(arena, .{ .reply = try arena.dupe(u8, a) });
                } else if (ending == .Limit) {
                    try entries.append(arena, .{ .note = try arena.dupe(u8, a) });
                }
            }
        }
        try entries.append(arena, .{ .ended = .{
            .at = tod(at),
            .ending = ending,
            .took_s = took_s,
        } });
        return try entries.toOwnedSlice(arena);
    }

    fn cancelled(self: *Seen, arena: std.mem.Allocator, at: u64) ![]write.Entry {
        self.rest = .idle;
        const took_s = if (self.since) |since|
            if (at >= since) (at - since) / 1000 else null
        else
            null;
        self.run = false;
        self.since = null;
        self.calling = false;
        var entries: std.ArrayList(write.Entry) = .empty;
        try entries.append(arena, .{ .ended = .{
            .at = tod(at),
            .ending = .Cancelled,
            .took_s = took_s,
        } });
        return try entries.toOwnedSlice(arena);
    }

    fn setLast(self: *Seen, text: []const u8) void {
        const owned = self.gpa.dupe(u8, text) catch return;
        self.gpa.free(self.last_text);
        self.last_text = owned;
    }
};

fn noteEntries(arena: std.mem.Allocator, said: []const u8) ![]write.Entry {
    const entries = try arena.alloc(write.Entry, 1);
    entries[0] = .{ .note = try arena.dupe(u8, said) };
    return entries;
}

fn tod(at: u64) ?u32 {
    return if (local_time.local(at)) |t| t.seconds else null;
}

const Ui = struct {
    gpa: std.mem.Allocator,
    io: std.Io,
    environ: *std.process.Environ.Map,
    editor: editor_mod.Editor,
    screen: screen_mod.Screen,
    seen: Seen,
    bus: mei_process.Bus,
    process: ?mei_process.Process = null,
    session: ?mei_session.Session = null,
    session_dir: []u8,
    pending: std.ArrayList([]u8) = .empty,
    replay_path: ?[]const u8,
    replay_file: ?std.Io.File = null,
    replay_done: bool,
    reporting: program_status.Reporting,
    held: program_status.Held = .opening,
    lost: bool = false,
    mei_program: []u8,
    frame_arena: std.heap.ArenaAllocator,

    fn init(
        gpa: std.mem.Allocator,
        io: std.Io,
        environ: *std.process.Environ.Map,
        replay: ?[]const u8,
    ) !Ui {
        const model = environ.get("HARNESS_MODEL") orelse "HARNESS_MODEL is not set";
        var cwd_buf: [std.fs.max_path_bytes]u8 = undefined;
        const session_cwd: []const u8 = blk: {
            const cwd_len = std.process.currentPath(io, &cwd_buf) catch break :blk ".";
            if (cwd_len == 0) break :blk ".";
            break :blk cwd_buf[0..cwd_len];
        };
        const session_dir = try gpa.dupe(u8, session_cwd);

        const mei_bin = environ.get("MEI_BIN");
        var sibling_owned: ?[]u8 = null;
        defer if (sibling_owned) |p| gpa.free(p);
        const sibling_exists = blk: {
            var exe_buf: [std.fs.max_path_bytes]u8 = undefined;
            const n = std.process.executablePath(io, &exe_buf) catch break :blk false;
            const dir = std.fs.path.dirname(exe_buf[0..n]) orelse break :blk false;
            var sib_buf: [std.fs.max_path_bytes]u8 = undefined;
            const sib = std.fmt.bufPrint(&sib_buf, "{s}{c}{s}", .{
                dir,
                std.fs.path.sep,
                mei_path.siblingName(),
            }) catch break :blk false;
            sibling_owned = try gpa.dupe(u8, sib);
            std.Io.Dir.cwd().access(io, sibling_owned.?, .{}) catch {
                gpa.free(sibling_owned.?);
                sibling_owned = null;
                break :blk false;
            };
            break :blk true;
        };

        var prog_buf: [std.fs.max_path_bytes]u8 = undefined;
        const resolved = mei_path.resolve(mei_bin, sibling_owned, sibling_exists, &prog_buf) catch "mei";
        const mei_program = try gpa.dupe(u8, resolved);

        var replay_file: ?std.Io.File = null;
        const replay_done = replay == null;
        if (replay) |path| {
            replay_file = std.Io.Dir.cwd().openFile(io, path, .{}) catch |err| {
                const diag = try std.fmt.allocPrint(gpa,
                    \\```toml
                    \\code = "E_MEI_CONFIG"
                    \\description = "open the Mei replay file: {s}"
                    \\cause = "the replay path must name a readable Mei JSONL file\npath = {s}\nvalue = {s}"
                    \\resolution = "pass the file saved from a live run, for example `calyx --replay run.jsonl`"
                    \\```
                , .{ @errorName(err), path, path });
                defer gpa.free(diag);
                std.debug.print("{s}\n", .{diag});
                gpa.free(session_dir);
                gpa.free(mei_program);
                return error.ReplayOpen;
            };
        }

        const reporting = program_status.Reporting.deciding(environ.get("SPRAWLING_PROGRAM_STATUS"));

        return .{
            .gpa = gpa,
            .io = io,
            .environ = environ,
            .editor = editor_mod.Editor.init(gpa),
            .screen = .{ .gpa = gpa },
            .seen = try Seen.init(gpa, model),
            .bus = .{ .gpa = gpa, .io = io },
            .session_dir = session_dir,
            .replay_path = replay,
            .replay_file = replay_file,
            .replay_done = replay_done,
            .reporting = reporting,
            .mei_program = mei_program,
            .frame_arena = .init(gpa),
        };
    }

    fn deinit(self: *Ui) void {
        if (self.process) |*p| p.deinit();
        if (self.session) |*s| s.deinit();
        for (self.pending.items) |line| self.gpa.free(line);
        self.pending.deinit(self.gpa);
        self.bus.deinit();
        self.seen.deinit();
        self.editor.deinit();
        self.screen.deinit();
        self.frame_arena.deinit();
        self.gpa.free(self.session_dir);
        self.gpa.free(self.mei_program);
        if (self.replay_file) |f| f.close(self.io);
        self.* = undefined;
    }

    fn loop(self: *Ui) u8 {
        if (self.screen.take() == .lost) return self.giveBack(1);

        const gateway = self.environ.get("ANTHROPIC_BASE_URL") orelse "http://127.0.0.1:3425";
        const address = if (std.mem.startsWith(u8, gateway, "http://")) gateway["http://".len..] else gateway;
        {
            const banner = [_]write.Entry{.{ .banner = .{
                .city = self.session_dir,
                .address = address,
                .url = gateway,
            } }};
            self.drawn(self.screen.frameEntries(&banner, null, termio.size()));
        }

        var stdin_buf: [4096]u8 = undefined;
        var stdin_filled: usize = 0;

        while (true) {
            if (self.lost) return self.giveBack(1);
            self.advanceReplay();
            self.drainMei();

            const timeout: i32 = if (self.process != null or !self.replay_done) 50 else -1;
            if (!pollStdin(timeout) and stdin_filled == 0) continue;

            const event = termio.readEvent(&stdin_buf, &stdin_filled) catch {
                self.lost = true;
                self.stop();
                continue;
            } orelse continue;

            if (self.handleEvent(event)) |code| return self.giveBack(code);
        }
    }

    fn handleEvent(self: *Ui, event: termio.Event) ?u8 {
        switch (event) {
            .key => |k| return self.onKey(k),
            .paste => |pasted| {
                self.editor.paste(pasted) catch {};
                self.redraw(&.{});
            },
            .resize => self.redraw(&.{}),
            .lost => {
                self.lost = true;
                self.stop();
            },
        }
        return null;
    }

    fn onKey(self: *Ui, k: termio.Key) ?u8 {
        if (k.modifiers.control) return null;
        if (k.code == .enter and !k.modifiers.shift) {
            self.enter();
            return null;
        }
        if (k.code == .esc) return self.escape();
        if (editing(k)) |edit| {
            self.editor.apply(edit) catch {};
            self.redraw(&.{});
        }
        return null;
    }

    fn escape(self: *Ui) ?u8 {
        if (!self.editor.isEmpty()) {
            self.editor.apply(.clear) catch {};
            self.redraw(&.{});
            return null;
        }
        if (self.process != null) {
            self.stop();
            return null;
        }
        return 0;
    }

    fn enter(self: *Ui) void {
        const typed = self.editor.enter() catch {
            self.redraw(&.{});
            return;
        } orelse {
            self.redraw(&.{});
            return;
        };
        defer self.gpa.free(typed);
        if (std.mem.trim(u8, typed, &std.ascii.whitespace).len == 0) return;

        const at = nowMs(self.io);
        _ = self.frame_arena.reset(.retain_capacity);
        const arena = self.frame_arena.allocator();
        const you = arena.alloc(write.Entry, 1) catch return;
        you[0] = .{ .you = .{ .at = tod(at), .said = arena.dupe(u8, typed) catch typed } };
        self.print(you);

        if (self.process != null) {
            self.printNote("one Mei task is already running; Esc stops it");
            return;
        }
        if (self.replay_path != null) {
            self.printNote("replay is read-only");
            return;
        }
        if (self.command(typed)) return;
        if (self.session == null and !self.createSession()) return;

        for (self.pending.items) |line| self.gpa.free(line);
        self.pending.clearRetainingCapacity();
        self.seen.begin(at);
        self.status(.{ .counted = self.seen.counts() });

        const resume_path = if (self.session) |*s| s.resumePath() else null;
        switch (mei_process.Process.spawn(self.gpa, self.io, &self.bus, self.mei_program, typed, resume_path)) {
            .ok => |proc| self.process = proc,
            .err => |diag| {
                defer self.gpa.free(diag);
                const entries = self.seen.finished(arena, .{ .err = diag }, nowMs(self.io)) catch return;
                self.print(entries);
                self.ended();
            },
        }
    }

    fn command(self: *Ui, typed: []const u8) bool {
        if (std.mem.eql(u8, typed, "/new")) {
            _ = self.createSession();
            return true;
        }
        if (std.mem.eql(u8, typed, "/resume")) {
            self.printNote("usage: /resume <jsonl>");
            return true;
        }
        if (std.mem.startsWith(u8, typed, "/resume ")) {
            const path = std.mem.trim(u8, typed["/resume ".len..], &std.ascii.whitespace);
            if (path.len == 0) self.printNote("usage: /resume <jsonl>") else self.resumeSession(path);
            return true;
        }
        if (std.mem.startsWith(u8, typed, "/")) {
            self.printNote("unknown Mei command; available: /new, /resume <jsonl>");
            return true;
        }
        return false;
    }

    fn createSession(self: *Ui) bool {
        const at = nowMs(self.io);
        switch (mei_session.Session.createIn(self.gpa, self.io, self.session_dir, at)) {
            .ok => |session| {
                if (self.session) |*old| old.deinit();
                const path = session.pathSlice();
                for (self.pending.items) |line| self.gpa.free(line);
                self.pending.clearRetainingCapacity();
                self.session = session;
                self.resetSeen();
                var buf: [512]u8 = undefined;
                const msg = std.fmt.bufPrint(&buf, "new Mei session: {s}", .{path}) catch "new Mei session";
                self.printNote(msg);
                return true;
            },
            .err => |diag| {
                defer self.gpa.free(diag);
                self.printNote(diag);
                return false;
            },
        }
    }

    fn resumeSession(self: *Ui, path_in: []const u8) void {
        const path = if (std.fs.path.isAbsolute(path_in))
            path_in
        else blk: {
            break :blk std.fs.path.join(self.gpa, &.{ self.session_dir, path_in }) catch {
                self.printNote("join resume path");
                return;
            };
        };
        defer if (!std.fs.path.isAbsolute(path_in)) self.gpa.free(path);

        switch (mei_session.Session.import(self.gpa, self.io, path)) {
            .ok => |session| {
                const at = nowMs(self.io);
                for (self.pending.items) |line| self.gpa.free(line);
                self.pending.clearRetainingCapacity();
                self.resetSeen();
                _ = self.frame_arena.reset(.retain_capacity);
                const arena = self.frame_arena.allocator();
                var entries: std.ArrayList(write.Entry) = .empty;
                for (session.historyLines()) |line| {
                    const more = self.seen.line(arena, line, at) catch continue;
                    entries.appendSlice(arena, more) catch {};
                }
                var buf: [512]u8 = undefined;
                const msg = std.fmt.bufPrint(&buf, "resumed Mei session: {s}", .{session.pathSlice()}) catch "resumed Mei session";
                entries.append(arena, .{ .note = arena.dupe(u8, msg) catch msg }) catch {};
                if (self.session) |*old| old.deinit();
                self.session = session;
                self.print(entries.items);
            },
            .err => |diag| {
                defer self.gpa.free(diag);
                self.printNote(diag);
            },
        }
    }

    fn resetSeen(self: *Ui) void {
        const model = self.gpa.dupe(u8, self.seen.model) catch return;
        defer self.gpa.free(model);
        self.seen.deinit();
        self.seen = Seen.init(self.gpa, model) catch return;
    }

    fn drainMei(self: *Ui) void {
        var drained: std.ArrayList(mei_process.Event) = .empty;
        defer drained.deinit(self.gpa);
        self.bus.drainInto(&drained);
        for (drained.items) |ev| {
            switch (ev) {
                .line => |line| self.onLine(line),
                .finished => |fin| self.onFinished(fin),
            }
        }
    }

    fn onLine(self: *Ui, line: []u8) void {
        const is_message = blk: {
            var parsed = std.json.parseFromSlice(std.json.Value, self.gpa, line, .{}) catch break :blk false;
            defer parsed.deinit();
            if (parsed.value != .object) break :blk false;
            const role = parsed.value.object.get("role") orelse break :blk false;
            if (role != .string) break :blk false;
            if (!std.mem.eql(u8, role.string, "user") and !std.mem.eql(u8, role.string, "assistant")) break :blk false;
            if (parsed.value.object.get("content") == null) break :blk false;
            break :blk true;
        };
        if (is_message) {
            self.pending.append(self.gpa, line) catch {
                self.gpa.free(line);
                return;
            };
        } else {
            // line still needed for projection; free after if not pending
        }
        defer if (!is_message) self.gpa.free(line);

        _ = self.frame_arena.reset(.retain_capacity);
        const arena = self.frame_arena.allocator();
        const entries = self.seen.line(arena, line, nowMs(self.io)) catch return;
        self.print(entries);
    }

    fn onFinished(self: *Ui, fin: mei_process.Finished) void {
        if (self.process) |*p| {
            p.finish();
            self.process = null;
        }

        const successful = !self.lost and !fin.cancelled and fin.result == .ok;
        if (successful) {
            if (self.session) |*session| {
                switch (session.append(self.pending.items)) {
                    .ok => {},
                    .err => |diag| {
                        defer self.gpa.free(diag);
                        if (self.session) |*s| {
                            s.deinit();
                            self.session = null;
                        }
                        self.printNote(diag);
                    },
                }
            }
            for (self.pending.items) |line| self.gpa.free(line);
            self.pending.clearRetainingCapacity();
        } else {
            for (self.pending.items) |line| self.gpa.free(line);
            self.pending.clearRetainingCapacity();
        }

        _ = self.frame_arena.reset(.retain_capacity);
        const arena = self.frame_arena.allocator();
        const at = nowMs(self.io);
        const entries = if (fin.cancelled)
            self.seen.cancelled(arena, at) catch return
        else
            self.seen.finished(arena, fin.result, at) catch return;
        // finished/cancelled don't free result — free here
        fin.result.deinit(self.gpa);
        self.print(entries);
        self.ended();
    }

    fn stop(self: *Ui) void {
        if (self.process) |*p| {
            switch (p.requestStop()) {
                .ok => {},
                .err => |e| {
                    defer self.gpa.free(e);
                    self.printNote(e);
                },
            }
        }
    }

    fn ended(self: *Ui) void {
        self.status(.{ .ended = self.seen.rest });
        self.status(.{ .counted = self.seen.counts() });
    }

    fn status(self: *Ui, event: program_status.Event) void {
        if (self.reporting == .off) return;
        const stepped = program_status.step(self.held, event);
        self.held = stepped.held;
        if (stepped.report) |report| {
            const ansi = report.writeAnsi(self.gpa) catch return;
            defer self.gpa.free(ansi);
            self.drawn(self.screen.report(ansi));
        }
    }

    fn advanceReplay(self: *Ui) void {
        var i: usize = 0;
        while (i < replay_batch) : (i += 1) {
            if (self.replay_done or self.process != null or self.lost) return;
            const file = self.replay_file orelse return;
            var buf: [64 * 1024]u8 = undefined;
            var reader = file.readerStreaming(self.io, &buf);
            const line = reader.interface.takeDelimiterExclusive('\n') catch |err| {
                self.replay_done = true;
                if (err != error.EndOfStream) {
                    var msg_buf: [128]u8 = undefined;
                    const msg = std.fmt.bufPrint(&msg_buf, "read Mei replay: {s}", .{@errorName(err)}) catch "read Mei replay";
                    self.printNote(msg);
                }
                return;
            };
            const trimmed = std.mem.trimEnd(u8, line, "\r");
            _ = self.frame_arena.reset(.retain_capacity);
            const arena = self.frame_arena.allocator();
            const entries = self.seen.line(arena, trimmed, nowMs(self.io)) catch continue;
            self.print(entries);
        }
    }

    fn printNote(self: *Ui, said: []const u8) void {
        _ = self.frame_arena.reset(.retain_capacity);
        const arena = self.frame_arena.allocator();
        const entries = noteEntries(arena, said) catch return;
        self.print(entries);
    }

    fn print(self: *Ui, entries: []const write.Entry) void {
        self.redraw(entries);
    }

    fn redraw(self: *Ui, entries: []const write.Entry) void {
        const typed = self.editor.textOwned() catch return;
        defer self.gpa.free(typed);
        var offer_buf: [256]u8 = undefined;
        const offer = self.seen.offer(&offer_buf);

        const working: ?write.Working = if (self.seen.since) |since| .{
            .resident = "Mei",
            .since = tod(since),
        } else null;

        var calling_buf: [1]write.Calling = undefined;
        const calling: []const write.Calling = if (self.seen.calling) blk: {
            calling_buf[0] = .{ .name = "python", .subject = "工具波执行中" };
            break :blk calling_buf[0..1];
        } else &.{};

        const live: write.Live = .{
            .working = working,
            .calling = calling,
            .composer = .{
                .typed = typed,
                .cursor = self.editor.cursorAt(),
                .placeholder = if (self.replay_path != null) "replay; Esc exits" else "to Mei…",
                .room = "Mei",
                .offer = offer,
            },
        };
        self.drawn(self.screen.frameEntries(entries, live, termio.size()));
    }

    fn drawn(self: *Ui, written: screen_mod.Written) void {
        if (written == .lost) self.lost = true;
    }

    fn giveBack(self: *Ui, code: u8) u8 {
        self.stop();
        var spins: usize = 0;
        while (self.process != null and spins < 50) : (spins += 1) {
            self.drainMei();
            if (self.process) |*p| {
                if (p.isFinished()) break;
            } else break;
            self.io.sleep(std.Io.Duration.fromMilliseconds(100), .real) catch {};
        }
        if (self.process) |*p| {
            p.deinit();
            self.process = null;
        }
        self.status(.closed);
        self.drawn(self.screen.giveBack());
        return code;
    }
};

fn editing(key: termio.Key) ?editor_mod.Edit {
    const alt = key.modifiers.alt;
    return switch (key.code) {
        .enter => if (key.modifiers.shift) .newline else null,
        .char => |ch| if (!alt) .{ .insert = ch } else null,
        .backspace => if (alt) .delete_word else .backspace,
        .delete => .delete,
        .left => if (alt) .word_left else .left,
        .right => if (alt) .word_right else .right,
        .home => .home,
        .end => .end,
        .up => .older,
        .down => .newer,
        else => null,
    };
}

fn pollStdin(timeout_ms: i32) bool {
    return termio.waitReadable(timeout_ms);
}

fn nowMs(io: std.Io) u64 {
    const ts = std.Io.Clock.real.now(io);
    const ms = ts.toMilliseconds();
    return if (ms < 0) 0 else @intCast(ms);
}

test "observations update offer without notes" {
    const gpa = std.testing.allocator;
    var seen = try Seen.init(gpa, "deepseek/flash");
    defer seen.deinit();
    var arena_state = std.heap.ArenaAllocator.init(gpa);
    defer arena_state.deinit();
    const arena = arena_state.allocator();

    var offer_buf: [128]u8 = undefined;
    try std.testing.expectEqualStrings("deepseek/flash · unknown", seen.offer(&offer_buf));

    const pulse = try seen.line(arena, "{\"type\":\"mei.obs\",\"kind\":\"pulse\"}", 1);
    try std.testing.expectEqual(@as(usize, 0), pulse.len);

    const usage = try seen.line(arena, "{\"type\":\"mei.obs\",\"kind\":\"usage\",\"usage\":{\"input_tokens\":3}}", 2);
    try std.testing.expectEqual(@as(usize, 0), usage.len);
    try std.testing.expect(std.mem.indexOf(u8, seen.offer(&offer_buf), "input_tokens") != null);
}
