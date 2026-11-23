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
const menu_mod = @import("menu.zig");
const mei_view = @import("mei_view.zig");

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
    log: std.ArrayList(write.Entry) = .empty,
    log_arena: std.heap.ArenaAllocator,
    cards: std.ArrayList(mei_view.Card) = .empty,
    focus: usize = 0,
    menu: ?menu_mod.Menu = null,
    hits: std.ArrayList(mei_view.Hit) = .empty,
    columns: u16 = 0,
    rows: u16 = 0,
    phase: u2 = 0,
    hinted: bool = false,
    seen: observation.Seen,
    bus: mei_process.Bus,
    process: ?mei_process.Process = null,
    session: ?mei_session.Session = null,
    previous_session: ?[]u8 = null,
    capture_dir: ?[]u8 = null,
    steer_path: ?[]u8 = null,
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
    cadence: present.Cadence = .{},

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
            .log_arena = .init(gpa),
        };
    }

    fn deinit(self: *Ui) void {
        if (self.process) |*process| process.deinit();
        if (self.steer_path) |path| self.gpa.free(path);
        if (self.capture_dir) |path| {
            mei_session.discardCaptureDirectory(self.io, path) catch {};
            self.gpa.free(path);
        }
        if (self.session) |*session| session.deinit();
        if (self.previous_session) |path| self.gpa.free(path);
        for (self.pending.items) |line| self.gpa.free(line);
        self.pending.deinit(self.gpa);
        self.clearMenu();
        self.freeCards();
        self.cards.deinit(self.gpa);
        self.hits.deinit(self.gpa);
        self.log.deinit(self.gpa);
        self.log_arena.deinit();
        self.bus.deinit();
        self.seen.deinit();
        self.editor.deinit();
        self.screen.deinit();
        self.frame_arena.deinit();
        self.gpa.free(self.session_dir);
        self.gpa.free(self.mei_program);
        if (self.replay_file) |file| file.close(self.io);
        self.* = undefined;
    }

    fn loop(self: *Ui) u8 {
        if (self.screen.take() == .lost) return self.giveBack(1);
        self.openFace();
        self.pushBanner();
        _ = self.redraw();
        var stdin_buf: [4096]u8 = undefined;
        var stdin_filled: usize = 0;
        while (true) {
            if (self.lost) return self.giveBack(1);
            self.tick();
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

    fn openFace(self: *Ui) void {
        _ = self;
        termio.enterAlternate() catch {};
        termio.trackPointer() catch {};
        termio.setTitle("mei") catch {};
    }

    fn releaseFace(self: *Ui) void {
        termio.untrackPointer() catch {};
        termio.plainKeyboard() catch {};
        termio.setCursorStyle(.default) catch {};
        termio.writeAll("\x1b[?25h") catch {};
        termio.leaveAlternate() catch {};
        termio.disableRaw() catch {};
        self.screen.cut();
    }

    fn pollTimeout(self: *const Ui) i32 {
        const at = nowNs(self.io);
        if (self.cadence.due(at, false)) return 0;
        if (self.cadence.dirty) return self.cadence.waitMs(at);
        if (self.process != null or !self.replay_done or self.seen.since != null or self.seen.calling) return 50;
        return 120;
    }

    fn tick(self: *Ui) void {
        const size = termio.size();
        if (size.columns != self.columns or size.rows != self.rows) self.touch(true);
        if (self.seen.since != null or self.seen.calling) {
            const phase: u2 = @intCast((self.now() / 160) % 4);
            if (phase != self.phase) {
                self.phase = phase;
                self.touch(true);
            }
        }
    }

    fn handleEvent(self: *Ui, event: termio.Event) ?u8 {
        switch (event) {
            .key => |key| return self.onKey(key),
            .paste => |pasted| {
                self.editor.paste(pasted) catch {};
                self.refilter();
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
        if (self.clicked(key)) return null;
        if (key.modifiers.control) {
            if (key.code == .char and key.code.char == 'o') {
                self.cycleFocused();
                self.touch(true);
            } else if (key.code == .char and key.code.char == 'c' and !self.hinted) {
                self.hinted = true;
                self.printNote("Esc stops the run; /quit leaves");
            }
            return null;
        }
        if (key.modifiers.alt and (key.code == .up or key.code == .down)) {
            self.moveFocus(if (key.code == .up) -1 else 1);
            return null;
        }
        if (self.menu != null and self.menuKey(key)) return if (self.quit) 0 else null;
        if (key.code == .enter and !key.modifiers.shift) {
            self.enter();
            return if (self.quit) 0 else null;
        }
        if (key.code == .esc) return self.escape();
        if (key.code == .tab or key.code == .back_tab) {
            self.refilter();
            self.touch(true);
            return null;
        }
        if (key.code == .up or key.code == .down) {
            const edit: editor_mod.Edit = if (key.code == .up) .older else .newer;
            self.editor.apply(edit) catch {};
            self.refilter();
            self.touch(true);
            return null;
        }
        if (editing(key)) |edit| {
            self.editor.apply(edit) catch {};
            self.refilter();
            self.touch(true);
        }
        return null;
    }

    fn clicked(self: *Ui, key: termio.Key) bool {
        if (key.code != .click) return false;
        const click = key.code.click;
        if (click.release or (click.button & 3) != 0 or click.button >= 64 or click.row == 0) return true;
        const row = click.row - 1;
        for (self.hits.items) |hit| {
            if (row >= hit.row and row < hit.end and hit.card < self.cards.items.len) {
                self.focus = hit.card;
                self.cards.items[hit.card].cycle();
                self.touch(true);
                break;
            }
        }
        return true;
    }

    fn menuKey(self: *Ui, key: termio.Key) bool {
        const menu = &self.menu.?;
        if (key.code == .tab or key.code == .down) {
            menu.next();
        } else if (key.code == .back_tab or key.code == .up) {
            menu.previous();
        } else if (key.code == .esc) {
            self.clearMenu();
        } else if (key.code == .enter) {
            const fit = menu.chosenFit() orelse return false;
            const line = self.gpa.dupe(u8, fit.line) catch return false;
            defer self.gpa.free(line);
            const then = fit.then;
            self.editor.set(line) catch {};
            self.clearMenu();
            if (then == .carry) self.enter();
        } else return false;
        self.touch(true);
        return true;
    }

    fn refilter(self: *Ui) void {
        const typed = self.editor.textOwned() catch return;
        defer self.gpa.free(typed);
        const chosen = if (self.menu) |menu| menu.chosen else 0;
        self.clearMenu();
        if (!mei_view.wantsMenu(typed)) return;
        const fits = mei_view.commands(self.gpa, typed) catch return;
        self.menu = menu_mod.Menu.of(fits);
        if (self.menu) |*menu| {
            if (chosen < menu.fits.len) menu.chosen = chosen;
        }
    }

    fn clearMenu(self: *Ui) void {
        if (self.menu) |*menu| {
            menu_mod.freeVerbs(self.gpa, menu.fits);
            self.menu = null;
        }
    }

    fn escape(self: *Ui) ?u8 {
        switch (escEffect(self.editor.isEmpty(), self.process != null)) {
            .clear_draft => {
                self.editor.apply(.clear) catch {};
                self.touch(true);
            },
            .stop_run => runtime.stop(self),
            .idle => {},
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
        self.clearMenu();
        if (std.mem.trim(u8, typed, &std.ascii.whitespace).len == 0) {
            self.touch(true);
            return;
        }
        const at = self.now();
        switch (session_control.dispatch(self, typed)) {
            .handled => self.showYou(typed, at),
            .quit => self.quit = true,
            .steer => {
                self.showYou(typed, at);
                runtime.steer(self, typed);
            },
            .task => {
                if (self.session == null and !session_control.createSession(self)) return;
                self.showYou(typed, at);
                runtime.submit(self, typed, at);
            },
        }
    }

    fn showYou(self: *Ui, typed: []const u8, at: u64) void {
        self.seen.suppressEcho(typed);
        const arena = self.frame_arena.allocator();
        const you = arena.alloc(write.Entry, 1) catch return;
        you[0] = .{ .you = .{ .at = observation.tod(at), .said = arena.dupe(u8, typed) catch typed } };
        self.print(you);
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
            self.absorb(entries);
            _ = self.frame_arena.reset(.retain_capacity);
        }
        self.cadence.mark();
        self.flushPresent(false);
    }

    pub fn clearConversation(self: *Ui) void {
        self.freeLog();
        self.freeCards();
        self.focus = 0;
        self.hits.clearRetainingCapacity();
        self.pushBanner();
    }

    fn pushBanner(self: *Ui) void {
        const gateway = self.environ.get("ANTHROPIC_BASE_URL") orelse "http://127.0.0.1:3425";
        const address = if (std.mem.startsWith(u8, gateway, "http://"))
            gateway["http://".len..]
        else if (std.mem.startsWith(u8, gateway, "https://"))
            gateway["https://".len..]
        else
            gateway;
        const banner = [_]write.Entry{.{ .banner = .{ .city = self.session_dir, .address = address, .url = gateway } }};
        self.absorb(&banner);
    }

    fn absorb(self: *Ui, entries: []const write.Entry) void {
        const bodies = self.seen.takeBodies() catch return;
        const allocated = bodies.len > 0;
        var next: usize = 0;
        defer {
            while (next < bodies.len) : (next += 1) {
                self.gpa.free(bodies[next].code);
                self.gpa.free(bodies[next].output);
            }
            if (allocated) self.gpa.free(bodies);
        }
        const before = self.cards.items.len;
        for (entries) |entry| {
            const owned = present.cloneEntry(self.log_arena.allocator(), entry) catch return;
            self.log.append(self.gpa, owned) catch return;
            if (owned == .tool and std.mem.eql(u8, owned.tool.name, "python") and next < bodies.len) {
                self.cards.append(self.gpa, .{
                    .code = bodies[next].code,
                    .output = bodies[next].output,
                }) catch return;
                next += 1;
            }
        }
        if (self.cards.items.len > before) self.focus = self.cards.items.len - 1;
    }

    fn freeLog(self: *Ui) void {
        self.log.clearRetainingCapacity();
        _ = self.log_arena.reset(.retain_capacity);
    }

    fn freeCards(self: *Ui) void {
        for (self.cards.items) |card| {
            self.gpa.free(card.code);
            self.gpa.free(card.output);
        }
        self.cards.clearRetainingCapacity();
    }

    fn cycleFocused(self: *Ui) void {
        if (self.cards.items.len == 0) return;
        if (self.focus >= self.cards.items.len) self.focus = self.cards.items.len - 1;
        self.cards.items[self.focus].cycle();
    }

    fn moveFocus(self: *Ui, delta: i2) void {
        if (self.cards.items.len == 0) return;
        if (delta < 0) {
            self.focus = if (self.focus == 0) self.cards.items.len - 1 else self.focus - 1;
        } else self.focus = (self.focus + 1) % self.cards.items.len;
        self.touch(true);
    }

    pub fn touch(self: *Ui, force: bool) void {
        self.cadence.mark();
        self.flushPresent(force);
    }

    pub fn flushPresent(self: *Ui, force: bool) void {
        const at = nowNs(self.io);
        if (!self.cadence.due(at, force)) return;
        if (!self.redraw()) return;
        self.cadence.presented(at);
    }

    fn redraw(self: *Ui) bool {
        const size = termio.size();
        const typed = self.editor.textOwned() catch return false;
        defer self.gpa.free(typed);
        var offer_buf: [512]u8 = undefined;
        const offer = self.seen.offer(&offer_buf);
        const resident: []const u8 = switch (self.phase) {
            0 => "Mei",
            1 => "Mei ·",
            2 => "Mei ··",
            3 => "Mei ···",
        };
        const working: ?write.Working = if (self.seen.since) |since| .{ .resident = resident, .since = observation.tod(since) } else null;
        const running: []const u8 = switch (self.phase) {
            0 => "running",
            1 => "running ·",
            2 => "running ··",
            3 => "running ···",
        };
        var calling_buf: [1]write.Calling = undefined;
        const calling: []const write.Calling = if (self.seen.calling) blk: {
            calling_buf[0] = .{ .name = "python", .subject = running };
            break :blk calling_buf[0..1];
        } else &.{};
        var menu_buf: [5]menu_mod.Choice = undefined;
        var menu_choices: [5]write.Choice = undefined;
        const menu_live: ?write.Menu = if (self.menu) |*menu| blk: {
            const win = menu.window(&menu_buf);
            const shown = win[0];
            var i: usize = 0;
            while (i < shown.len and i < menu_choices.len) : (i += 1) {
                menu_choices[i] = .{
                    .spelling = shown[i].spelling,
                    .takes = shown[i].takes,
                    .summary = shown[i].summary,
                };
            }
            break :blk .{ .shown = menu_choices[0..shown.len], .chosen = win[1], .more = win[2] };
        } else null;
        const placeholder: []const u8 = if (self.replay_path != null)
            "replay; /q exits"
        else if (self.process != null)
            "steer…   esc stops"
        else
            "to Mei…   / commands · ctrl-o tool";
        const live: write.Live = .{
            .working = working,
            .calling = calling,
            .composer = .{
                .typed = typed,
                .cursor = self.editor.cursorAt(),
                .placeholder = placeholder,
                .room = "Mei",
                .offer = offer,
            },
            .menu = menu_live,
        };
        const bytes = mei_view.render(
            self.gpa,
            self.log.items,
            self.cards.items,
            self.focus,
            self.seen.draftText(),
            live,
            size.columns,
            size.rows,
            &self.hits,
        ) catch return false;
        defer self.gpa.free(bytes);
        termio.writeAll(bytes) catch {
            self.lost = true;
            return false;
        };
        self.columns = size.columns;
        self.rows = size.rows;
        return true;
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
        self.releaseFace();
        return code;
    }
};

const EscEffect = enum { clear_draft, stop_run, idle };

fn escEffect(draft_empty: bool, running: bool) EscEffect {
    if (!draft_empty) return .clear_draft;
    if (running) return .stop_run;
    return .idle;
}

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

test "idle Esc neither clears a draft nor stops a run" {
    try std.testing.expectEqual(EscEffect.idle, escEffect(true, false));
    try std.testing.expectEqual(EscEffect.clear_draft, escEffect(false, false));
    try std.testing.expectEqual(EscEffect.stop_run, escEffect(true, true));
    try std.testing.expectEqual(EscEffect.clear_draft, escEffect(false, true));
}
