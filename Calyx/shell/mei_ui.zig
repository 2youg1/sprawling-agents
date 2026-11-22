// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

const std = @import("std");
const leaf = @import("leaf");
const write = leaf.write;
const argv = @import("argv.zig");
const mei_path = @import("mei_path.zig");
const editor_mod = @import("editor.zig");
const program_status = @import("program_status.zig");
const screen_mod = @import("screen.zig");
const termio = @import("termio.zig");
const mei_process = @import("mei_process.zig");
const mei_line = @import("mei_line.zig");
const mei_session = @import("mei_session.zig");
const observation = @import("mei_observation.zig");
const session_control = @import("mei_session_control.zig");
const runtime = @import("mei_runtime.zig");
const present = @import("mei_present.zig");

const replay_batch: usize = 64;

pub fn run(init: std.process.Init) u8 {
    const gpa = init.gpa;
    const io = init.io;
    var arg_list: std.ArrayList([]const u8) = .empty;
    defer arg_list.deinit(gpa);
    var args_it = std.process.Args.Iterator.initAllocator(init.minimal.args, gpa) catch return 1;
    defer args_it.deinit();
    while (args_it.next()) |arg| arg_list.append(gpa, arg) catch return 1;
    const replay = switch (argv.replayFlag(arg_list.items)) {
        .none => null,
        .missing_value => {
            std.debug.print("missing value for --replay\n", .{});
            return 1;
        },
        .path => |path| path,
    };
    var ui = Ui.init(gpa, io, init.environ_map, replay) catch |err| {
        std.debug.print("Calyx initialization failed: {s}\n", .{@errorName(err)});
        return 1;
    };
    defer ui.deinit();
    return ui.loop();
}

pub const Ui = struct {
    gpa: std.mem.Allocator,
    io: std.Io,
    environ: *std.process.Environ.Map,
    editor: editor_mod.Editor,
    screen: screen_mod.Screen,
    seen: observation.Seen,
    bus: mei_process.Bus,
    process: ?mei_process.Process = null,
    session: ?mei_session.Session = null,
    previous_session: ?[]u8 = null,
    capture_dir: ?[]u8 = null,
    session_dir: []u8,
    pending: std.ArrayList([]u8) = .empty,
    replay_path: ?[]const u8,
    replay_file: ?std.Io.File = null,
    replay_buf: [8192]u8 = undefined,
    replay_reader: ?std.Io.File.Reader = null,
    replay_done: bool,
    reporting: program_status.Reporting,
    held: program_status.Held = .opening,
    lost: bool = false,
    quit: bool = false,
    mei_program: []u8,
    frame_arena: std.heap.ArenaAllocator,
    queue_arena: std.heap.ArenaAllocator,
    cadence: present.Cadence = .{},
    queued: std.ArrayList(write.Entry) = .empty,

    fn init(gpa: std.mem.Allocator, io: std.Io, environ: *std.process.Environ.Map, replay: ?[]const u8) !Ui {
        const model = environ.get("HARNESS_MODEL") orelse "auto";
        var cwd_buf: [std.fs.max_path_bytes]u8 = undefined;
        const session_cwd: []const u8 = blk: {
            const cwd_len = std.process.currentPath(io, &cwd_buf) catch break :blk ".";
            if (cwd_len == 0) break :blk ".";
            break :blk cwd_buf[0..cwd_len];
        };
        const session_dir = try gpa.dupe(u8, session_cwd);
        const mei_bin = environ.get("MEI_BIN");
        var sibling_owned: ?[]u8 = null;
        defer if (sibling_owned) |path| gpa.free(path);
        const sibling_exists = blk: {
            var exe_buf: [std.fs.max_path_bytes]u8 = undefined;
            const n = std.process.executablePath(io, &exe_buf) catch break :blk false;
            const dir = std.fs.path.dirname(exe_buf[0..n]) orelse break :blk false;
            var sib_buf: [std.fs.max_path_bytes]u8 = undefined;
            const sibling = std.fmt.bufPrint(&sib_buf, "{s}{c}{s}", .{ dir, std.fs.path.sep, mei_path.siblingName() }) catch break :blk false;
            sibling_owned = try gpa.dupe(u8, sibling);
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
        return .{
            .gpa = gpa,
            .io = io,
            .environ = environ,
            .editor = editor_mod.Editor.init(gpa),
            .screen = .{ .gpa = gpa },
            .seen = try observation.Seen.init(gpa, model),
            .bus = .{ .gpa = gpa, .io = io },
            .session_dir = session_dir,
            .replay_path = replay,
            .replay_file = replay_file,
            .replay_done = replay_done,
            .reporting = program_status.Reporting.deciding(environ.get("SPRAWLING_PROGRAM_STATUS")),
            .mei_program = mei_program,
            .frame_arena = .init(gpa),
            .queue_arena = .init(gpa),
        };
    }

    fn deinit(self: *Ui) void {
        if (self.process) |*process| process.deinit();
        if (self.capture_dir) |path| {
            mei_session.discardCaptureDirectory(self.io, path) catch {};
            self.gpa.free(path);
        }
        if (self.session) |*session| session.deinit();
        if (self.previous_session) |path| self.gpa.free(path);
        for (self.pending.items) |line| self.gpa.free(line);
        self.pending.deinit(self.gpa);
        self.queued.deinit(self.gpa);
        self.bus.deinit();
        self.seen.deinit();
        self.editor.deinit();
        self.screen.deinit();
        self.frame_arena.deinit();
        self.queue_arena.deinit();
        self.gpa.free(self.session_dir);
        self.gpa.free(self.mei_program);
        if (self.replay_file) |file| file.close(self.io);
        self.* = undefined;
    }

    fn loop(self: *Ui) u8 {
        if (self.screen.take() == .lost) return self.giveBack(1);
        const gateway = self.environ.get("ANTHROPIC_BASE_URL") orelse "http://127.0.0.1:3425";
        const address = if (std.mem.startsWith(u8, gateway, "http://")) gateway["http://".len..] else gateway;
        const banner = [_]write.Entry{.{ .banner = .{ .city = self.session_dir, .address = address, .url = gateway } }};
        self.drawn(self.screen.frameEntries(&banner, null, termio.size()));
        var stdin_buf: [4096]u8 = undefined;
        var stdin_filled: usize = 0;
        while (true) {
            if (self.lost) return self.giveBack(1);
            self.advanceReplay();
            runtime.drain(self);
            self.flushPresent(false);
            const timeout = self.pollTimeout();
            if (stdin_filled == 0 and !pollStdin(timeout)) continue;
            const event = termio.readEvent(&stdin_buf, &stdin_filled) catch {
                self.lost = true;
                runtime.stop(self);
                continue;
            } orelse continue;
            if (self.handleEvent(event)) |code| return self.giveBack(code);
        }
    }

    fn pollTimeout(self: *const Ui) i32 {
        const at = nowNs(self.io);
        if (self.cadence.due(at, false)) return 0;
        if (self.cadence.dirty) return self.cadence.waitMs(at);
        if (self.process != null or !self.replay_done) return 50;
        return -1;
    }

    fn handleEvent(self: *Ui, event: termio.Event) ?u8 {
        switch (event) {
            .key => |key| return self.onKey(key),
            .paste => |pasted| {
                self.editor.paste(pasted) catch {};
                self.touch(true);
            },
            .resize => self.touch(true),
            .lost => {
                self.lost = true;
                runtime.stop(self);
            },
        }
        return null;
    }

    fn onKey(self: *Ui, key: termio.Key) ?u8 {
        if (key.modifiers.control) return null;
        if (key.code == .enter and !key.modifiers.shift) {
            self.enter();
            return if (self.quit) 0 else null;
        }
        if (key.code == .esc) return self.escape();
        if (editing(key)) |edit| {
            self.editor.apply(edit) catch {};
            self.touch(true);
        }
        return null;
    }

    fn escape(self: *Ui) ?u8 {
        if (!self.editor.isEmpty()) {
            self.editor.apply(.clear) catch {};
            self.touch(true);
            return null;
        }
        if (self.process != null) {
            runtime.stop(self);
        }
        return null;
    }

    fn enter(self: *Ui) void {
        const typed = self.editor.enter() catch {
            self.touch(true);
            return;
        } orelse {
            self.touch(true);
            return;
        };
        defer self.gpa.free(typed);
        if (std.mem.trim(u8, typed, &std.ascii.whitespace).len == 0) return;
        const at = self.now();
        const arena = self.frame_arena.allocator();
        const you = arena.alloc(write.Entry, 1) catch return;
        you[0] = .{ .you = .{ .at = observation.tod(at), .said = arena.dupe(u8, typed) catch typed } };
        self.print(you);
        switch (session_control.dispatch(self, typed)) {
            .handled => return,
            .quit => {
                self.quit = true;
                return;
            },
            .task => {},
        }
        if (self.session == null and !session_control.createSession(self)) return;
        runtime.submit(self, typed, at);
    }

    pub fn now(self: *Ui) u64 {
        return nowMs(self.io);
    }

    fn replayReader(self: *Ui) ?*std.Io.Reader {
        if (self.replay_reader == null) {
            const file = self.replay_file orelse return null;
            self.replay_reader = std.Io.File.Reader.initStreaming(file, self.io, &self.replay_buf);
        }
        return &self.replay_reader.?.interface;
    }

    fn advanceReplay(self: *Ui) void {
        var index: usize = 0;
        while (index < replay_batch) : (index += 1) {
            if (self.replay_done or self.process != null or self.lost) return;
            const reader = self.replayReader() orelse return;
            const line = (mei_line.read(self.gpa, reader, mei_line.MAX_BYTES) catch |err| {
                self.replay_done = true;
                const said = mei_line.failureText(self.gpa, err) catch {
                    self.printNote("read Mei replay: the line outgrew its ceiling or the file failed");
                    return;
                };
                defer self.gpa.free(said);
                self.printNote(said);
                return;
            }) orelse {
                self.replay_done = true;
                return;
            };
            defer self.gpa.free(line);
            const arena = self.frame_arena.allocator();
            const entries = self.seen.line(arena, line, self.now()) catch continue;
            self.print(entries);
        }
    }

    pub fn printNote(self: *Ui, said: []const u8) void {
        const arena = self.frame_arena.allocator();
        const entries = observation.noteEntries(arena, said) catch return;
        self.print(entries);
    }

    pub fn print(self: *Ui, entries: []const write.Entry) void {
        if (entries.len > 0) {
            if (!self.enqueueOwned(entries)) {
                self.cadence.mark();
                self.flushPresent(true);
                if (!self.enqueueOwned(entries)) {
                    self.redraw(entries);
                    if (self.queued.items.len == 0) {
                        _ = self.queue_arena.reset(.retain_capacity);
                    }
                    _ = self.frame_arena.reset(.retain_capacity);
                    self.cadence.mark();
                    self.cadence.presented(nowNs(self.io));
                    return;
                }
            }
            _ = self.frame_arena.reset(.retain_capacity);
        }
        self.cadence.mark();
        self.flushPresent(false);
    }

    fn enqueueOwned(self: *Ui, entries: []const write.Entry) bool {
        const start = self.queued.items.len;
        const q = self.queue_arena.allocator();
        for (entries) |entry| {
            const owned = present.cloneEntry(q, entry) catch {
                self.queued.shrinkRetainingCapacity(start);
                return false;
            };
            self.queued.append(self.gpa, owned) catch {
                self.queued.shrinkRetainingCapacity(start);
                return false;
            };
        }
        return true;
    }

    pub fn touch(self: *Ui, force: bool) void {
        self.cadence.mark();
        self.flushPresent(force);
    }

    pub fn flushPresent(self: *Ui, force: bool) void {
        const at = nowNs(self.io);
        if (!self.cadence.due(at, force)) return;
        self.redraw(self.queued.items);
        self.queued.clearRetainingCapacity();
        _ = self.queue_arena.reset(.retain_capacity);
        self.cadence.presented(at);
    }

    fn redraw(self: *Ui, entries: []const write.Entry) void {
        const typed = self.editor.textOwned() catch return;
        defer self.gpa.free(typed);
        var offer_buf: [512]u8 = undefined;
        const offer = self.seen.offer(&offer_buf);
        const working: ?write.Working = if (self.seen.since) |since| .{ .resident = "Mei", .since = observation.tod(since) } else null;
        var calling_buf: [1]write.Calling = undefined;
        const calling: []const write.Calling = if (self.seen.calling) blk: {
            calling_buf[0] = .{ .name = "python", .subject = "工具波执行中" };
            break :blk calling_buf[0..1];
        } else &.{};
        const draft = self.seen.draftText();
        const live: write.Live = .{
            .working = working,
            .calling = calling,
            .replying = if (draft.len > 0) draft else null,
            .composer = .{
                .typed = typed,
                .cursor = self.editor.cursorAt(),
                .placeholder = if (self.replay_path != null) "replay; /q exits" else "to Mei…",
                .room = "Mei",
                .offer = offer,
            },
        };
        self.drawn(self.screen.frameEntries(entries, live, termio.size()));
    }

    fn drawn(self: *Ui, written: screen_mod.Written) void {
        if (written == .lost) self.lost = true;
    }

    pub fn status(self: *Ui, event: program_status.Event) void {
        if (self.reporting == .off) return;
        const stepped = program_status.step(self.held, event);
        self.held = stepped.held;
        if (stepped.report) |report| {
            const ansi = report.writeAnsi(self.gpa) catch return;
            defer self.gpa.free(ansi);
            self.drawn(self.screen.report(ansi));
        }
    }

    pub fn ended(self: *Ui) void {
        self.status(.{ .ended = self.seen.rest });
        self.status(.{ .counted = self.seen.counts() });
    }

    fn giveBack(self: *Ui, code: u8) u8 {
        runtime.stop(self);
        var spins: usize = 0;
        while (self.process != null and spins < 50) : (spins += 1) {
            runtime.drain(self);
            self.flushPresent(true);
            if (self.process) |*process| {
                if (process.isFinished()) break;
            } else break;
            self.io.sleep(std.Io.Duration.fromMilliseconds(100), .real) catch {};
        }
        if (self.process) |*process| {
            process.deinit();
            self.process = null;
        }
        self.flushPresent(true);
        self.status(.closed);
        self.drawn(self.screen.giveBack());
        return code;
    }
};

fn editing(key: termio.Key) ?editor_mod.Edit {
    const alt = key.modifiers.alt;
    return switch (key.code) {
        .enter => if (key.modifiers.shift) .newline else null,
        .char => |char| if (!alt) .{ .insert = char } else null,
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
    const timestamp = std.Io.Clock.real.now(io);
    const milliseconds = timestamp.toMilliseconds();
    return if (milliseconds < 0) 0 else @intCast(milliseconds);
}

fn nowNs(io: std.Io) u64 {
    const timestamp = std.Io.Clock.real.now(io);
    const nanoseconds = timestamp.toNanoseconds();
    return if (nanoseconds < 0) 0 else @intCast(nanoseconds);
}
