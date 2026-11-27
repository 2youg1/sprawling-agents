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
const effort = @import("mei_effort.zig");
const workspace = @import("mei_workspace.zig");

const replay_batch: usize = 64;
const application: program_status.Application = .mei;

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
    log: std.ArrayList(mei_view.LogEntry) = .empty,
    log_arena: std.heap.ArenaAllocator,
    cards: std.ArrayList(mei_view.Card) = .empty,
    focus: usize = 0,
    menu: ?menu_mod.Menu = null,
    choosing_effort: bool = false,
    hits: std.ArrayList(mei_view.Hit) = .empty,
    columns: u16 = 0,
    rows: u16 = 0,
    view: mei_view.View = .{},
    tool_arrived_ms: u64 = 0,
    press: ?Press = null,
    hinted: bool = false,
    seen: observation.Seen,
    bus: mei_process.Bus,
    process: ?mei_process.Process = null,
    session: ?mei_session.Session = null,
    previous_session: ?[]u8 = null,
    capture_dir: ?[]u8 = null,
    steer_path: ?[]u8 = null,
    session_dir: []u8,
    git_head: ?[]u8 = null,
    git_probe: ?std.Thread = null,
    git_refresh_pending: bool = false,
    banner_caption: ?[]u8 = null,
    banner_agents: bool = false,
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
            .bus = mei_process.Bus.init(gpa, io),
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
        if (self.git_probe) |thread| thread.join();
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
        self.view.deinit(self.gpa);
        self.log.deinit(self.gpa);
        self.log_arena.deinit();
        self.bus.deinit();
        self.seen.deinit();
        self.editor.deinit();
        self.screen.deinit();
        self.frame_arena.deinit();
        self.gpa.free(self.session_dir);
        if (self.git_head) |head| self.gpa.free(head);
        if (self.banner_caption) |caption| self.gpa.free(caption);
        self.gpa.free(self.mei_program);
        if (self.replay_file) |file| file.close(self.io);
        self.* = undefined;
    }

    fn loop(self: *Ui) u8 {
        if (self.screen.take(@tagName(application), null) == .lost) return self.giveBack(1);
        self.openFace();
        self.refreshGit();
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
            if (stdin_filled == 0 and !self.bus.waitForInput(timeout)) continue;
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
    }

    fn releaseFace(self: *Ui) void {
        termio.untrackPointer() catch {};
        termio.plainKeyboard() catch {};
        termio.writeAll("\x1b[?25h") catch {};
        termio.leaveAlternate() catch {};
        termio.disableRaw() catch {};
        self.screen.cut();
    }

    fn pollTimeout(self: *const Ui) i32 {
        const at = nowNs(self.io);
        if (self.cadence.due(at, false)) return 0;
        if (self.cadence.dirty) return self.cadence.waitMs(at);
        if (self.seen.since != null or self.view.completion != null) return present.MOTION_INTERVAL_MS;
        if (!self.replay_done or (self.bus.notification == null and (self.process != null or self.git_probe != null))) return 50;
        if (!termio.resizeArmed()) return 1000;
        return -1;
    }

    fn tick(self: *Ui) void {
        const motion_ms = nowNs(self.io) / std.time.ns_per_ms;
        if (self.view.completion) |*completion| {
            const tone = present.arrivalTone(motion_ms -| self.tool_arrived_ms);
            if (tone != @as(?u8, completion.tone)) {
                if (tone) |next| completion.tone = next else self.view.completion = null;
                self.cadence.mark();
            }
        }
        if (!termio.resizeArmed()) {
            const size = termio.size();
            if (size.columns != self.columns or size.rows != self.rows) self.noteResize();
        }
        if (self.seen.since != null) self.advanceTide(present.tideTone(motion_ms));
    }

    fn advanceTide(self: *Ui, tone: u8) void {
        if (self.view.working_tone == tone) return;
        self.view.working_tone = tone;
        if (self.cadence.dirty) return;
        const mark = self.view.working_mark orelse return;
        const size = termio.size();
        if (size.columns != self.columns or size.rows != self.rows) return;
        var bytes: [128]u8 = undefined;
        var patch: std.Io.Writer = .fixed(&bytes);
        mei_view.workingPatch(&patch, mark, tone) catch unreachable;
        termio.writeAll(patch.buffered()) catch {
            self.lost = true;
        };
    }

    fn handleEvent(self: *Ui, event: termio.Event) ?u8 {
        switch (event) {
            .key => |key| if (key.phase != .release) return self.onKey(key),
            .paste => |pasted| {
                self.editor.paste(pasted) catch {};
                self.refilter();
                self.touch(true);
            },
            .resize => self.noteResize(),
            .keyboard_flags, .focus_lost => {},
            .lost => {
                self.lost = true;
                runtime.stop(self);
            },
        }
        return null;
    }

    fn onKey(self: *Ui, key: termio.Key) ?u8 {
        if (key.code == .wheel) {
            const wheel = key.code.wheel;
            if (wheel.row != 0 and self.view.contains(wheel.row - 1)) {
                self.view.scroll(wheel.lines);
                self.view.selection = null;
                self.touch(true);
            }
            return null;
        }
        if (self.clicked(key)) return null;
        self.view.selection = null;
        if (key.modifiers.control) {
            if (key.code == .end) {
                self.view.follow();
                self.touch(true);
            } else if (key.code == .char and key.code.char == 'o') {
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
            const text = self.editor.text.items;
            const cursor = self.editor.cursorAt();
            const first = std.mem.indexOfScalar(u21, text[0..@min(cursor, text.len)], '\n') == null;
            const edit = historyMove(cursor, text.len, self.editor.recalled != null, first, key.code == .up);
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
        if ((click.button & 3) != 0 or click.button >= 64 or click.row == 0 or click.column == 0) return true;
        const point: mei_view.Point = .{ .row = click.row - 1, .column = click.column - 1 };
        if ((click.button & 32) != 0) {
            if (self.press) |*press| {
                press.dragged = true;
                if (self.dragRange(self.view.down, point)) |range| {
                    self.view.selection = range;
                    self.touch(true);
                }
            }
            return true;
        }
        if (!click.release) {
            self.press = .{ .point = point };
            self.view.down = if (self.view.contains(point.row)) self.view.location(point) else null;
            if (self.view.selection != null) {
                self.view.selection = null;
                self.touch(true);
            }
            return true;
        }
        const press = self.press orelse return true;
        self.press = null;
        const down = self.view.down;
        self.view.down = null;
        if (press.dragged or !std.meta.eql(press.point, point)) {
            if (self.dragRange(down, point)) |range| self.copySelection(range);
            return true;
        }
        if (down) |body| {
            const released = self.view.location(point) orelse return true;
            if (!std.meta.eql(body, released)) return true;
        }
        if (self.menu) |*menu| {
            var choices: [5]menu_mod.Choice = undefined;
            const shown, const chosen, _ = menu.window(&choices);
            const first_row = self.rows -| shown.len;
            if (click.row >= first_row and click.row < self.rows) {
                menu.chosen = menu.chosen - chosen + click.row - first_row;
                _ = self.menuKey(.{ .code = .enter });
                return true;
            }
        }
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

    const Press = struct {
        point: mei_view.Point,
        dragged: bool = false,
    };

    fn dragRange(self: *const Ui, down: ?mei_view.Location, point: mei_view.Point) ?mei_view.Selection {
        return .{ .start = down orelse return null, .finish = self.view.location(point) orelse return null };
    }

    fn copySelection(self: *Ui, range: mei_view.Selection) void {
        var text: std.Io.Writer.Allocating = .init(self.gpa);
        defer text.deinit();
        self.view.selection = range;
        self.view.copy = &text.writer;
        self.touch(true);
        self.view.copy = null;
        if (text.written().len != 0) termio.copySelection(self.gpa, self.io, self.environ, text.written()) catch {
            self.printNote("could not copy the terminal selection");
        };
    }

    fn menuKey(self: *Ui, key: termio.Key) bool {
        const menu = &self.menu.?;
        if (key.code == .tab or key.code == .down) {
            menu.next();
        } else if (key.code == .back_tab or key.code == .up) {
            menu.previous();
        } else if (key.code == .esc) {
            const return_to_slash = self.choosing_effort;
            self.clearMenu();
            if (return_to_slash) {
                self.editor.set("/effect") catch {};
                self.refilter();
            }
        } else if (key.code == .enter) {
            const fit = menu.chosenFit() orelse return false;
            if (self.choosing_effort) {
                session_control.chooseEffort(self, fit.spelling);
                self.clearMenu();
                self.touch(true);
                return true;
            }
            const line = self.gpa.dupe(u8, fit.line) catch return false;
            defer self.gpa.free(line);
            const then = fit.then;
            self.editor.set(line) catch {};
            self.clearMenu();
            if (then == .carry) self.enter();
        } else return self.choosing_effort;
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
            if (self.choosing_effort) menu_mod.freeArguments(self.gpa, menu.fits) else menu_mod.freeVerbs(self.gpa, menu.fits);
            self.menu = null;
        }
        self.choosing_effort = false;
    }

    fn openEffort(self: *Ui) void {
        if (self.seen.effortLevels().len == 0) self.loadEfforts();
        const levels = self.seen.effortLevels();
        if (levels.len == 0) {
            self.printNote("Magpie reports no reasoning levels for this model");
            return;
        }
        const fits = menu_mod.arguments(self.gpa, "/effect", "", levels) catch return;
        self.menu = menu_mod.Menu.of(fits);
        self.choosing_effort = true;
        self.menu.?.chosen = effort.currentIndex(self.seen.currentEffort(), levels);
        self.touch(true);
    }

    fn loadEfforts(self: *Ui) void {
        const result = std.process.run(self.gpa, self.io, .{
            .argv = &.{ self.mei_program, "--describe" },
            .environ_map = self.environ,
            .stdout_limit = .limited(64 * 1024),
            .stderr_limit = .limited(64 * 1024),
            .timeout = .{ .duration = .{ .raw = std.Io.Duration.fromSeconds(3), .clock = .awake } },
        }) catch return;
        defer self.gpa.free(result.stdout);
        defer self.gpa.free(result.stderr);
        if (!result.term.success()) return;
        const entries = self.seen.line(self.frame_arena.allocator(), result.stdout, self.now()) catch return;
        self.print(entries);
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
            .handled => self.touch(true),
            .effort_menu => self.openEffort(),
            .quit => self.quit = true,
            .steer => {
                self.showYou(typed, at);
                runtime.steer(self, typed);
            },
            .task => {
                if (self.session == null and !session_control.createSession(self)) return;
                self.refreshGit();
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

    pub fn now(self: *const Ui) u64 {
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
        if (self.seen.agents_loaded != self.banner_agents) self.updateBanner();
        if (entries.len > 0) {
            self.absorb(entries);
            _ = self.frame_arena.reset(.retain_capacity);
        }
        self.cadence.mark();
        self.flushPresent(false);
    }

    pub fn clearConversation(self: *Ui) void {
        self.view.follow();
        self.view.trackDraft(null);
        self.view.completion = null;
        self.view.selection = null;
        self.view.down = null;
        self.press = null;
        self.freeLog();
        self.freeCards();
        self.focus = 0;
        self.hits.clearRetainingCapacity();
        self.pushBanner();
    }

    fn pushBanner(self: *Ui) void {
        self.log.append(self.gpa, .{ .item = .{ .banner = .{ .city = self.session_dir, .address = "", .url = "" } } }) catch return;
        self.updateBanner();
    }

    pub fn refreshGit(self: *Ui) void {
        if (self.quit or self.lost) return;
        if (self.git_probe != null) {
            self.git_refresh_pending = true;
            return;
        }
        self.git_probe = std.Thread.spawn(.{}, struct {
            fn run(bus: *mei_process.Bus, directory: []const u8, path: []const u8) void {
                bus.push(.{ .git_head = workspace.gitHead(bus.gpa, bus.io, directory, path) });
            }
        }.run, .{ &self.bus, self.session_dir, self.environ.get("PATH") orelse "" }) catch return;
    }

    pub fn gitHeadArrived(self: *Ui, head: ?[]u8) void {
        if (self.git_probe) |thread| thread.join();
        self.git_probe = null;
        if (self.git_head) |old| self.gpa.free(old);
        self.git_head = head;
        self.updateBanner();
        self.touch(false);
        if (self.git_refresh_pending) {
            self.git_refresh_pending = false;
            self.refreshGit();
        }
    }

    fn updateBanner(self: *Ui) void {
        if (self.log.items.len == 0 or self.log.items[0].item != .banner) return;
        const head = self.git_head orelse "";
        const caption = std.fmt.allocPrint(self.gpa, "{s}{s}{s}", .{
            head,
            if (head.len != 0 and self.seen.agents_loaded) " · " else "",
            if (self.seen.agents_loaded) "AGENTS.md" else "",
        }) catch return;
        if (self.banner_caption) |old| self.gpa.free(old);
        self.banner_caption = caption;
        self.banner_agents = self.seen.agents_loaded;
        self.log.items[0].item.banner.address = caption;
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
        var first_result: ?u16 = null;
        for (entries) |entry| {
            const paired = entry == .tool and std.mem.eql(u8, entry.tool.name, "python") and next < bodies.len;
            const thought = entry == .reasoning and entry.reasoning.body.len != 0;
            if (paired or thought) self.cards.ensureUnusedCapacity(self.gpa, 1) catch return;
            const owned = present.cloneEntry(self.log_arena.allocator(), entry) catch return;
            self.log.append(self.gpa, .{ .item = owned, .card = if (paired or thought) self.cards.items.len else null }) catch return;
            if (entry == .reply and self.seen.draftText().len == 0) self.view.commitDraft(self.log.items.len - 1);
            if (paired) {
                self.cards.appendAssumeCapacity(.{ .python = .{
                    .code = bodies[next].code,
                    .output = bodies[next].output,
                } });
                const card = self.cards.items.len - 1;
                if (self.process != null and card < std.math.maxInt(u16)) {
                    const last: u16 = @intCast(card);
                    first_result = first_result orelse last;
                    self.tool_arrived_ms = nowNs(self.io) / std.time.ns_per_ms;
                    self.view.completion = .{ .first = first_result.?, .last = last, .tone = present.arrivalTone(0).? };
                }
                next += 1;
            } else if (thought) {
                self.cards.appendAssumeCapacity(.{ .reasoning = false });
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
            if (card == .python) {
                self.gpa.free(card.python.code);
                self.gpa.free(card.python.output);
            }
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

    fn noteResize(self: *Ui) void {
        self.touch(true);
    }

    fn redraw(self: *Ui) bool {
        const size = termio.size();
        const typed = self.editor.textOwned() catch return false;
        defer self.gpa.free(typed);
        var offer_buf: [512]u8 = undefined;
        const offer = self.seen.offer(&offer_buf);
        const working: ?write.Working = if (self.seen.since) |since| .{ .resident = "Mei", .since = observation.tod(since) } else null;
        var calling_buf: [1]write.Calling = undefined;
        const calling: []const write.Calling = if (self.seen.calling) blk: {
            calling_buf[0] = .{ .name = "python", .subject = "" };
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
                    .summary = if (self.choosing_effort and std.mem.eql(u8, shown[i].spelling, self.seen.currentEffort())) "current" else shown[i].summary,
                };
            }
            break :blk .{ .shown = menu_choices[0..shown.len], .chosen = win[1], .more = win[2] };
        } else null;
        const placeholder: []const u8 = if (self.replay_path != null)
            "replay; /q exits"
        else if (self.process != null)
            "steer…"
        else
            "to Mei…   / commands";
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
            &self.view,
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
            const ansi = report.writeAnsi(self.gpa, application) catch return;
            defer self.gpa.free(ansi);
            self.drawn(self.screen.report(ansi));
        }
    }

    pub fn ended(self: *Ui) void {
        self.status(.{ .ended = self.seen.rest });
        self.status(.{ .counted = self.seen.counts() });
    }

    fn giveBack(self: *Ui, code: u8) u8 {
        self.quit = true;
        runtime.stop(self);
        const deadline: std.Io.Clock.Timestamp = .fromNow(self.io, .{ .raw = .fromMilliseconds(5000), .clock = .awake });
        while (self.process != null) {
            runtime.drain(self);
            self.flushPresent(true);
            if (self.process == null) break;
            const remaining = deadline.durationFromNow(self.io).raw.toMilliseconds();
            if (remaining <= 0) break;
            self.bus.waitForEvents(@intCast(remaining));
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

fn historyMove(cursor: usize, len: usize, recalled: bool, first: bool, up: bool) editor_mod.Edit {
    if (up) {
        if (recalled and first) return .older;
        return if (cursor == 0) .older else .line_up;
    }
    return if (recalled or cursor == len) .newer else .line_down;
}

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

fn nowMs(io: std.Io) u64 {
    const timestamp = std.Io.Clock.real.now(io);
    const milliseconds = timestamp.toMilliseconds();
    return if (milliseconds < 0) 0 else @intCast(milliseconds);
}

fn nowNs(io: std.Io) u64 {
    const timestamp = std.Io.Clock.awake.now(io);
    const nanoseconds = timestamp.toNanoseconds();
    return if (nanoseconds < 0) 0 else @intCast(nanoseconds);
}

test "mei moves within a line until the cursor is at its end" {
    try std.testing.expectEqual(editor_mod.Edit.line_up, historyMove(3, 8, false, true, true));
    try std.testing.expectEqual(editor_mod.Edit.older, historyMove(0, 8, false, true, true));
    try std.testing.expectEqual(editor_mod.Edit.line_down, historyMove(3, 8, false, false, false));
    try std.testing.expectEqual(editor_mod.Edit.newer, historyMove(8, 8, false, false, false));
    try std.testing.expectEqual(editor_mod.Edit.newer, historyMove(2, 8, true, false, false));
    try std.testing.expectEqual(editor_mod.Edit.older, historyMove(4, 4, true, true, true));
    try std.testing.expectEqual(editor_mod.Edit.line_up, historyMove(6, 10, true, false, true));
}

test "idle Esc neither clears a draft nor stops a run" {
    try std.testing.expectEqual(EscEffect.idle, escEffect(true, false));
    try std.testing.expectEqual(EscEffect.clear_draft, escEffect(false, false));
    try std.testing.expectEqual(EscEffect.stop_run, escEffect(true, true));
    try std.testing.expectEqual(EscEffect.clear_draft, escEffect(false, true));
}
