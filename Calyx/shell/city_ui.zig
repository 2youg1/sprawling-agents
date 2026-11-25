// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

const std = @import("std");
const leaf = @import("leaf");
const write = leaf.write;
const host_mod = @import("host.zig");
const editor_mod = @import("editor.zig");
const menu_mod = @import("menu.zig");
const screen_mod = @import("screen.zig");
const termio = @import("termio.zig");
const lifecycle = @import("lifecycle.zig");

const held_back_cap: usize = 512;
const face_look_ms: i32 = 100;

pub const RunMode = enum { tty, dry };

pub fn run(
    gpa: std.mem.Allocator,
    config: *const host_mod.CityConfig,
    host: *const host_mod.Host,
    mode: RunMode,
) i32 {
    const ui = gpa.create(Ui) catch return 1;
    ui.init(gpa, config, host) catch {
        gpa.destroy(ui);
        return 1;
    };
    defer {
        ui.deinit();
        gpa.destroy(ui);
    }
    return ui.loop(mode);
}

export fn calyx_city_ui_run(config: *const host_mod.CityConfig, host: *const host_mod.Host) i32 {
    return run(std.heap.c_allocator, config, host, .tty);
}

const LiveMeta = struct {
    room: []u8 = "",
    offer: []u8 = "",
    waiting: []u8 = "",
    waiting_more: usize = 0,
    working_since: ?u32 = null,
    calling_name: []u8 = "",
    calling_subject: []u8 = "",
};

const Ui = struct {
    gpa: std.mem.Allocator,
    host: *const host_mod.Host,
    config: *const host_mod.CityConfig,
    editor: editor_mod.Editor,
    screen: screen_mod.Screen,
    shown: lifecycle.Face = .opening,
    menu: ?menu_mod.Menu = null,
    transient: ?[]u8 = null,
    pairing: []u8 = "",
    live: LiveMeta = .{},
    held_back: std.ArrayList(write.Entry) = .empty,
    held_arena: std.heap.ArenaAllocator,
    frame_arena: std.heap.ArenaAllocator,
    asking: ?u64 = null,
    hinted: bool = false,
    lost: bool = false,
    dry: bool = false,
    poll_buf: []u8 = &.{},
    stdin_scratch: [4096]u8 = undefined,

    fn init(self: *Ui, gpa: std.mem.Allocator, config: *const host_mod.CityConfig, host: *const host_mod.Host) !void {
        self.gpa = gpa;
        self.host = host;
        self.config = config;
        self.editor = .{ .allocator = gpa };
        self.screen = .{ .gpa = gpa };
        self.shown = .opening;
        self.menu = null;
        self.transient = null;
        self.pairing = "";
        self.live = .{};
        self.held_back = .empty;
        self.held_arena = .init(gpa);
        self.frame_arena = .init(gpa);
        self.asking = null;
        self.hinted = false;
        self.lost = false;
        self.dry = false;
        self.poll_buf = &.{};
        errdefer {
            self.held_arena.deinit();
            self.frame_arena.deinit();
        }
        self.pairing = try gpa.dupe(u8, "");
        errdefer gpa.free(self.pairing);
        self.poll_buf = try gpa.alloc(u8, 64 * 1024);
    }

    fn deinit(self: *Ui) void {
        self.editor.deinit();
        self.screen.deinit();
        self.clearMenu();
        if (self.transient) |t| self.gpa.free(t);
        self.gpa.free(self.pairing);
        self.freeLive();
        self.held_back.deinit(self.gpa);
        self.held_arena.deinit();
        self.frame_arena.deinit();
        if (self.poll_buf.len > 0) self.gpa.free(self.poll_buf);
        self.* = undefined;
    }

    fn freeLive(self: *Ui) void {
        if (self.live.room.len > 0) self.gpa.free(self.live.room);
        if (self.live.offer.len > 0) self.gpa.free(self.live.offer);
        if (self.live.waiting.len > 0) self.gpa.free(self.live.waiting);
        if (self.live.calling_name.len > 0) self.gpa.free(self.live.calling_name);
        if (self.live.calling_subject.len > 0) self.gpa.free(self.live.calling_subject);
        self.live = .{};
    }

    fn loop(self: *Ui, mode: RunMode) i32 {
        self.dry = mode == .dry;
        if (!self.dry) {
            if (self.screen.take("sprawling · closing this window stops it", .steady_bar) == .lost) {
                self.host.tell(self.host.ctx, @backingInt(host_mod.LifecycleEvent.terminal_lost), 0);
                return 1;
            }
        }

        const url = host_mod.urlOf(self.config);
        const city = host_mod.cityOf(self.config);
        const address = stripUrl(url);
        {
            const banner = [_]write.Entry{.{ .banner = .{
                .city = city,
                .address = address,
                .url = url,
            } }};
            self.print(&banner);
        }

        const surface: lifecycle.Face = if (self.config.surface == 1) .quiet_host else .cli;
        self.turnTo(surface);

        var stdin_filled: usize = 0;

        while (true) {
            if (self.lost) return self.giveBack(1);
            if (self.shown == .gone) return self.giveBack(0);

            self.drainHost();

            if (self.dry) return self.giveBack(0);

            const timeout: i32 = face_look_ms;
            if (!pollStdin(timeout) and stdin_filled == 0) continue;

            const event = termio.readEvent(&self.stdin_scratch, &stdin_filled) catch {
                self.lost = true;
                self.host.tell(self.host.ctx, @backingInt(host_mod.LifecycleEvent.terminal_lost), 0);
                continue;
            } orelse continue;

            if (self.handleEvent(event)) |_| return self.giveBack(0);
        }
    }

    fn drainHost(self: *Ui) void {
        var guard: usize = 0;
        while (guard < 64) : (guard += 1) {
            var written: usize = 0;
            const kind_u = self.host.poll(self.host.ctx, self.poll_buf.ptr, self.poll_buf.len, &written);
            const kind: host_mod.HostEventKind = std.enums.fromInt(host_mod.HostEventKind, kind_u) orelse .empty;
            if (kind == .empty) return;
            const payload = self.poll_buf[0..written];
            switch (kind) {
                .empty => return,
                .face => self.onFace(payload),
                .notice => self.onNotice(payload),
                .pairing => self.onPairing(payload),
                .entries => self.onEntries(payload),
                .live_meta => self.onLiveMeta(payload),
                .asking => self.onAsking(payload),
                .lost => {
                    self.screen.cut();
                    self.host.tell(self.host.ctx, @backingInt(host_mod.LifecycleEvent.terminal_lost), 0);
                    self.lost = true;
                    return;
                },
            }
        }
    }

    fn onFace(self: *Ui, payload: []const u8) void {
        if (payload.len < @sizeOf(host_mod.FaceBytes)) return;
        var face: host_mod.FaceBytes = undefined;
        @memcpy(std.mem.asBytes(&face), payload[0..@sizeOf(host_mod.FaceBytes)]);
        const next: lifecycle.Face = switch (@as(host_mod.FaceTag, @fromBackingInt(@intCast(face.tag)))) {
            .opening => .opening,
            .cli => .cli,
            .quiet_host => .quiet_host,
            .headless => .headless,
            .stopping => .{ .stopping = .{
                .mode = if (face.mode == 1) .interrupt else .drain,
                .sinks = if (face.sinks == 1) .cut else .held,
            } },
            .gone => .{ .gone = if (face.handoff == 1) .skipped else .written },
        };
        if (!facesEqual(next, self.shown)) self.turnTo(next);
    }

    fn onNotice(self: *Ui, payload: []const u8) void {
        if (self.screen.isQuiet()) {
            if (self.transient) |t| self.gpa.free(t);
            self.transient = self.gpa.dupe(u8, payload) catch null;
            self.quiet();
        } else {
            self.note(payload);
        }
    }

    fn onAsking(self: *Ui, payload: []const u8) void {
        if (payload.len < 8) return;
        var raw: [8]u8 = undefined;
        @memcpy(&raw, payload[0..8]);
        self.asking = std.mem.readInt(u64, &raw, .little);
        self.redraw(&.{});
    }

    fn onPairing(self: *Ui, payload: []const u8) void {
        self.gpa.free(self.pairing);
        self.pairing = self.gpa.dupe(u8, payload) catch "";
        if (self.shown == .quiet_host) self.quiet();
    }

    fn onEntries(self: *Ui, payload: []const u8) void {
        _ = self.frame_arena.reset(.retain_capacity);
        const arena = self.frame_arena.allocator();
        const entries = host_mod.readEntries(arena, payload) catch {
            self.note("  the console dropped a malformed host frame");
            return;
        };
        self.print(entries);
    }

    fn onLiveMeta(self: *Ui, payload: []const u8) void {
        if (payload.len < @sizeOf(host_mod.LiveMetaHeader)) return;
        var header: host_mod.LiveMetaHeader = undefined;
        @memcpy(std.mem.asBytes(&header), payload[0..@sizeOf(host_mod.LiveMetaHeader)]);
        var at: usize = @sizeOf(host_mod.LiveMetaHeader);
        const room = takeStr(payload, &at, header.room_len) orelse return;
        const offer = takeStr(payload, &at, header.offer_len) orelse return;
        const waiting = takeStr(payload, &at, header.waiting_len) orelse return;
        var calling_name: []const u8 = "";
        var calling_subject: []const u8 = "";
        if (header.calling_count > 0) {
            if (at + 4 > payload.len) return;
            const nlen = std.mem.readInt(u32, payload[at..][0..4], .little);
            at += 4;
            calling_name = takeStr(payload, &at, nlen) orelse return;
            if (at + 4 > payload.len) return;
            const slen = std.mem.readInt(u32, payload[at..][0..4], .little);
            at += 4;
            calling_subject = takeStr(payload, &at, slen) orelse return;
        }
        self.freeLive();
        self.live = .{
            .room = self.gpa.dupe(u8, room) catch "",
            .offer = self.gpa.dupe(u8, offer) catch "",
            .waiting = self.gpa.dupe(u8, waiting) catch "",
            .waiting_more = header.waiting_more,
            .working_since = if (header.working_since == std.math.maxInt(u64)) null else @intCast(header.working_since),
            .calling_name = self.gpa.dupe(u8, calling_name) catch "",
            .calling_subject = self.gpa.dupe(u8, calling_subject) catch "",
        };
        if (self.shown == .cli or self.shown == .stopping) self.redraw(&.{});
    }

    fn turnTo(self: *Ui, next: lifecycle.Face) void {
        self.shown = next;
        switch (next) {
            .quiet_host => {
                self.quiet();
            },
            .cli => {
                if (self.transient) |t| {
                    self.gpa.free(t);
                    self.transient = null;
                }
                if (!self.dry) _ = self.screen.leaveQuiet();
                const held = self.held_back.items;
                self.redraw(held);
                self.held_back.clearRetainingCapacity();
                _ = self.held_arena.reset(.retain_capacity);
            },
            .stopping => |s| {
                if (s.sinks == .cut) {
                    self.screen.cut();
                } else {
                    const msg: []const u8 = switch (s.mode) {
                        .drain => "  sprawling is closing; the runs under way finish first (/quit again stops them now)",
                        .interrupt => "  sprawling is closing; the runs under way stop at their next safe point",
                    };
                    self.onNotice(msg);
                }
            },
            .gone => {
                if (!self.dry) _ = self.screen.giveBack();
            },
            .opening, .headless => {},
        }
    }

    fn handleEvent(self: *Ui, event: termio.Event) ?u8 {
        switch (event) {
            .key => |k| {
                self.onKey(k);
            },
            .paste => |pasted| {
                if (self.shown == .cli or self.shown == .stopping) {
                    self.editor.paste(pasted) catch {};
                    self.refilter();
                    self.redraw(&.{});
                }
            },
            .resize => {
                if (self.shown == .quiet_host) self.quiet() else self.redraw(&.{});
            },
            .lost => {
                self.screen.cut();
                self.host.tell(self.host.ctx, @backingInt(host_mod.LifecycleEvent.terminal_lost), 0);
                self.lost = true;
            },
        }
        return null;
    }

    fn onKey(self: *Ui, k: termio.Key) void {
        if (k.modifiers.control) {
            if (k.code == .char and k.code.char == 'c' and !self.hinted) {
                self.hinted = true;
                self.onNotice("  复制请先选中文字；关闭城市用 /quit");
            }
            return;
        }
        switch (self.shown) {
            .quiet_host => {
                if (k.code == .esc) {
                    self.host.tell(self.host.ctx, @backingInt(host_mod.LifecycleEvent.back), 0);
                } else if (k.code == .enter) {
                    const url = host_mod.urlOf(self.config);
                    _ = self.host.open_paired(self.host.ctx, url.ptr, url.len);
                }
            },
            .stopping => |s| {
                if (self.screen.isQuiet()) return;
                if (s.sinks == .cut) return;
                self.typing(k);
            },
            .cli => self.typing(k),
            .opening, .headless, .gone => {},
        }
    }

    fn typing(self: *Ui, k: termio.Key) void {
        if (self.asking) |going| {
            self.answerQuit(k, going);
            return;
        }
        if (self.menu != null and self.inMenu(k)) return;
        if (k.code == .enter and !k.modifiers.shift) {
            self.enter();
            return;
        }
        if (k.code == .esc) {
            self.escape();
            return;
        }
        if (k.code == .tab) {
            self.complete();
            return;
        }
        if (editing(k)) |edit| {
            self.editor.apply(edit) catch {};
            self.refilter();
            self.redraw(&.{});
        }
    }

    fn answerQuit(self: *Ui, k: termio.Key, going: u64) void {
        const event: ?host_mod.LifecycleEvent = if (k.code == .enter)
            .quit_drain
        else if (k.code == .char and k.code.char == 'n')
            .quit_interrupt
        else if (k.code == .esc)
            null
        else
            return;
        self.asking = null;
        if (event) |ev| {
            self.redraw(&.{});
            self.host.tell(self.host.ctx, @backingInt(ev), 0);
        } else {
            var buf: [96]u8 = undefined;
            const msg = std.fmt.bufPrint(&buf, "  the city keeps serving; {d} runs go on", .{going}) catch return;
            self.note(msg);
        }
    }

    fn inMenu(self: *Ui, k: termio.Key) bool {
        const open = self.menu orelse return false;
        _ = open;
        const menu = &self.menu.?;
        if (k.code == .tab or k.code == .down) {
            menu.next();
        } else if (k.code == .back_tab or k.code == .up) {
            menu.previous();
        } else if (k.code == .esc) {
            self.clearMenu();
        } else if (k.code == .enter) {
            const fit = menu.chosenFit() orelse return false;
            self.editor.set(fit.line) catch {};
            const then = fit.then;
            self.clearMenu();
            if (then == .carry) self.enter();
        } else return false;
        self.redraw(&.{});
        return true;
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
        if (std.mem.trim(u8, typed, &std.ascii.whitespace).len == 0) {
            self.redraw(&.{});
            return;
        }
        self.clearMenu();
        self.host.carry_line(self.host.ctx, typed.ptr, typed.len);
        self.redraw(&.{});
    }

    fn escape(self: *Ui) void {
        if (!self.editor.isEmpty()) {
            self.editor.apply(.clear) catch {};
            self.clearMenu();
            self.redraw(&.{});
            return;
        }
        self.host.tell(self.host.ctx, @backingInt(host_mod.LifecycleEvent.web), 0);
    }

    fn complete(self: *Ui) void {
        const typed = self.editor.textOwned() catch return;
        defer self.gpa.free(typed);
        self.clearMenu();
        if (!std.mem.startsWith(u8, typed, "/")) {
            self.redraw(&.{});
            return;
        }
        const fits = menu_mod.verbs(self.gpa, typed) catch {
            self.redraw(&.{});
            return;
        };
        self.menu = menu_mod.Menu.of(fits);
        self.redraw(&.{});
    }

    fn refilter(self: *Ui) void {
        if (self.menu == null) return;
        const typed = self.editor.textOwned() catch return;
        defer self.gpa.free(typed);
        self.clearMenu();
        if (!std.mem.startsWith(u8, typed, "/")) return;
        const fits = menu_mod.verbs(self.gpa, typed) catch return;
        self.menu = menu_mod.Menu.of(fits);
    }

    fn clearMenu(self: *Ui) void {
        if (self.menu) |*m| {
            menu_mod.freeVerbs(self.gpa, m.fits);
            self.menu = null;
        }
    }

    fn quiet(self: *Ui) void {
        if (self.dry) return;
        const written = self.screen.quiet(.{
            .url = host_mod.urlOf(self.config),
            .code = self.pairing,
            .key = host_mod.tokenOf(self.config),
            .transient = if (self.transient) |t| std.mem.trim(u8, t, &std.ascii.whitespace) else null,
        });
        self.drawn(written);
    }

    fn note(self: *Ui, said: []const u8) void {
        _ = self.frame_arena.reset(.retain_capacity);
        const arena = self.frame_arena.allocator();
        const entries = arena.alloc(write.Entry, 1) catch return;
        entries[0] = .{ .note = arena.dupe(u8, said) catch return };
        self.print(entries);
    }

    fn print(self: *Ui, entries: []const write.Entry) void {
        if (self.screen.isQuiet()) {
            for (entries) |entry| {
                const owned = dupEntry(self.held_arena.allocator(), entry) catch continue;
                self.held_back.append(self.gpa, owned) catch continue;
            }
            const over = self.held_back.items.len -| held_back_cap;
            var i: usize = 0;
            while (i < over and self.held_back.items.len > 0) : (i += 1) {
                _ = self.held_back.orderedRemove(0);
            }
            return;
        }
        self.redraw(entries);
    }

    fn redraw(self: *Ui, entries: []const write.Entry) void {
        if (self.dry) return;
        const typed = self.editor.textOwned() catch return;
        defer self.gpa.free(typed);
        const placeholder: []const u8 = "to the city…";
        var calling_buf: [1]write.Calling = undefined;
        const calling: []const write.Calling = if (self.live.calling_name.len > 0) blk: {
            calling_buf[0] = .{ .name = self.live.calling_name, .subject = self.live.calling_subject };
            break :blk calling_buf[0..1];
        } else &.{};

        var menu_buf: [5]menu_mod.Choice = undefined;
        var menu_choices: [5]write.Choice = undefined;
        const menu_live: ?write.Menu = if (self.menu) |*m| blk: {
            const win = m.window(&menu_buf);
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

        const live: ?write.Live = switch (self.shown) {
            .cli, .stopping => .{
                .waiting = if (self.live.waiting.len > 0) .{
                    .what = self.live.waiting,
                    .more = self.live.waiting_more,
                } else null,
                .working = if (self.live.working_since) |since| .{
                    .resident = if (self.live.room.len > 0) self.live.room else "city",
                    .since = since,
                } else null,
                .calling = calling,
                .asking = self.asking,
                .composer = .{
                    .typed = typed,
                    .cursor = self.editor.cursorAt(),
                    .placeholder = placeholder,
                    .room = if (self.live.room.len > 0) self.live.room else "city",
                    .offer = self.live.offer,
                },
                .menu = menu_live,
            },
            else => null,
        };
        self.drawn(self.screen.frameEntries(entries, live));
    }

    fn drawn(self: *Ui, written: screen_mod.Written) void {
        if (written == .lost) {
            self.lost = true;
            self.host.tell(self.host.ctx, @backingInt(host_mod.LifecycleEvent.terminal_lost), 0);
        }
    }

    fn giveBack(self: *Ui, code: i32) i32 {
        if (!self.dry and !self.lost) _ = self.screen.giveBack();
        return code;
    }
};

fn facesEqual(a: lifecycle.Face, b: lifecycle.Face) bool {
    return switch (a) {
        .opening => b == .opening,
        .cli => b == .cli,
        .quiet_host => b == .quiet_host,
        .headless => b == .headless,
        .stopping => |as| switch (b) {
            .stopping => |bs| as.mode == bs.mode and as.sinks == bs.sinks,
            else => false,
        },
        .gone => |ah| switch (b) {
            .gone => |bh| ah == bh,
            else => false,
        },
    };
}

fn takeStr(payload: []const u8, at: *usize, len: u32) ?[]const u8 {
    const n: usize = len;
    if (at.* + n > payload.len) return null;
    const slice = payload[at.* .. at.* + n];
    at.* += n;
    return slice;
}

fn stripUrl(url: []const u8) []const u8 {
    var s = url;
    if (std.mem.startsWith(u8, s, "http://")) s = s["http://".len..];
    if (std.mem.startsWith(u8, s, "https://")) s = s["https://".len..];
    return std.mem.trimEnd(u8, s, "/");
}

fn dupEntry(arena: std.mem.Allocator, entry: write.Entry) !write.Entry {
    return switch (entry) {
        .note => |s| .{ .note = try arena.dupe(u8, s) },
        .reply => |s| .{ .reply = try arena.dupe(u8, s) },
        .banner => |b| .{ .banner = .{
            .city = try arena.dupe(u8, b.city),
            .address = try arena.dupe(u8, b.address),
            .url = try arena.dupe(u8, b.url),
        } },
        .you => |y| .{ .you = .{ .at = y.at, .said = try arena.dupe(u8, y.said) } },
        .head => |h| blk: {
            const facts = try arena.alloc([]const u8, h.facts.len);
            for (h.facts, 0..) |f, i| facts[i] = try arena.dupe(u8, f);
            break :blk .{ .head = .{ .at = h.at, .resident = try arena.dupe(u8, h.resident), .facts = facts } };
        },
        .reasoning => |n| .{ .reasoning = n },
        .tool => |t| .{ .tool = .{
            .at = t.at,
            .name = try arena.dupe(u8, t.name),
            .subject = try arena.dupe(u8, t.subject),
            .took_us = t.took_us,
            .outcome = t.outcome,
        } },
        .ended => |e| .{ .ended = e },
        .resolved => |r| .{ .resolved = .{ .at = r.at, .verdict = r.verdict, .what = try arena.dupe(u8, r.what) } },
    };
}

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

const StubHost = struct {
    gpa: std.mem.Allocator,
    events: std.ArrayList(Queued) = .empty,
    tells: std.ArrayList(u32) = .empty,
    lines: std.ArrayList([]u8) = .empty,
    opened: usize = 0,

    const Queued = struct { kind: host_mod.HostEventKind, bytes: []u8 };

    fn deinit(self: *StubHost) void {
        for (self.events.items) |e| self.gpa.free(e.bytes);
        self.events.deinit(self.gpa);
        self.tells.deinit(self.gpa);
        for (self.lines.items) |l| self.gpa.free(l);
        self.lines.deinit(self.gpa);
    }

    fn pushFace(self: *StubHost, tag: host_mod.FaceTag) !void {
        var face = host_mod.FaceBytes{
            .tag = @backingInt(tag),
            .mode = 0,
            .sinks = 0,
            .handoff = 0,
        };
        const bytes = try self.gpa.dupe(u8, std.mem.asBytes(&face));
        try self.events.append(self.gpa, .{ .kind = .face, .bytes = bytes });
    }

    fn pushNotice(self: *StubHost, text: []const u8) !void {
        try self.events.append(self.gpa, .{ .kind = .notice, .bytes = try self.gpa.dupe(u8, text) });
    }

    fn pushAsking(self: *StubHost, going: u64) !void {
        var raw: [8]u8 = undefined;
        std.mem.writeInt(u64, &raw, going, .little);
        try self.events.append(self.gpa, .{ .kind = .asking, .bytes = try self.gpa.dupe(u8, &raw) });
    }

    fn pushEntries(self: *StubHost, notes: []const []const u8) !void {
        const bytes = try host_mod.encodeNotes(self.gpa, notes);
        try self.events.append(self.gpa, .{ .kind = .entries, .bytes = bytes });
    }

    fn asHost(self: *StubHost) host_mod.Host {
        return .{
            .ctx = self,
            .poll = poll,
            .tell = tell,
            .carry_line = carryLine,
            .open_paired = openPaired,
            .carry_remote = carryRemote,
            .clock_ms = clockMs,
            .post_json = postJson,
            .answer_json = answerJson,
        };
    }

    fn poll(ctx: *anyopaque, into: [*]u8, capacity: usize, written: *usize) callconv(.c) u32 {
        const self: *StubHost = @ptrCast(@alignCast(ctx));
        if (self.events.items.len == 0) {
            written.* = 0;
            return @backingInt(host_mod.HostEventKind.empty);
        }
        const ev = self.events.orderedRemove(0);
        defer self.gpa.free(ev.bytes);
        if (ev.bytes.len > capacity) {
            written.* = 0;
            return @backingInt(host_mod.HostEventKind.empty);
        }
        @memcpy(into[0..ev.bytes.len], ev.bytes);
        written.* = ev.bytes.len;
        return @backingInt(ev.kind);
    }

    fn tell(ctx: *anyopaque, event: u32, arg: u32) callconv(.c) void {
        _ = arg;
        const self: *StubHost = @ptrCast(@alignCast(ctx));
        self.tells.append(self.gpa, event) catch {};
    }

    fn carryLine(ctx: *anyopaque, line_ptr: [*]const u8, line_len: usize) callconv(.c) void {
        const self: *StubHost = @ptrCast(@alignCast(ctx));
        const owned = self.gpa.dupe(u8, line_ptr[0..line_len]) catch return;
        self.lines.append(self.gpa, owned) catch self.gpa.free(owned);
    }

    fn openPaired(ctx: *anyopaque, url_ptr: [*]const u8, url_len: usize) callconv(.c) i32 {
        _ = url_ptr;
        _ = url_len;
        const self: *StubHost = @ptrCast(@alignCast(ctx));
        self.opened += 1;
        return 0;
    }

    fn carryRemote(ctx: *anyopaque, io: *host_mod.BufIo) callconv(.c) i32 {
        _ = ctx;
        io.out_written.* = 0;
        return 0;
    }

    fn clockMs(ctx: *anyopaque, out: *u64) callconv(.c) i32 {
        _ = ctx;
        out.* = 0;
        return 0;
    }

    fn postJson(ctx: *anyopaque, json_ptr: [*]const u8, json_len: usize) callconv(.c) i32 {
        _ = ctx;
        _ = json_ptr;
        _ = json_len;
        return 0;
    }

    fn answerJson(ctx: *anyopaque, io: *host_mod.BufIo) callconv(.c) i32 {
        _ = ctx;
        io.out_written.* = 0;
        return 0;
    }
};

test "dry city ui drains host face entries and handles quiet keys via host" {
    const gpa = std.testing.allocator;
    var stub: StubHost = .{ .gpa = gpa };
    defer stub.deinit();
    try stub.pushFace(.quiet_host);
    try stub.pushEntries(&.{"projected note"});
    try stub.pushNotice("pairing soon");

    const url = "http://127.0.0.1:3425/";
    const city = "demo";
    const config = host_mod.CityConfig{
        .url_ptr = url.ptr,
        .url_len = url.len,
        .city_ptr = city.ptr,
        .city_len = city.len,
        .token_ptr = null,
        .token_len = 0,
        .surface = 1,
    };
    var host = stub.asHost();
    const code = run(gpa, &config, &host, .dry);
    try std.testing.expectEqual(@as(i32, 0), code);

    var ui: Ui = undefined;
    try ui.init(gpa, &config, &host);
    defer ui.deinit();
    ui.dry = true;
    ui.shown = .quiet_host;
    ui.onKey(.{ .code = .esc });
    ui.onKey(.{ .code = .enter });
    try std.testing.expect(stub.tells.items.len >= 1);
    try std.testing.expectEqual(@backingInt(host_mod.LifecycleEvent.back), stub.tells.items[0]);
    try std.testing.expectEqual(@as(usize, 1), stub.opened);
}

test "cli enter forwards the line to the host" {
    const gpa = std.testing.allocator;
    var stub: StubHost = .{ .gpa = gpa };
    defer stub.deinit();
    const url = "http://127.0.0.1:1/";
    const city = "c";
    const config = host_mod.CityConfig{
        .url_ptr = url.ptr,
        .url_len = url.len,
        .city_ptr = city.ptr,
        .city_len = city.len,
        .token_ptr = null,
        .token_len = 0,
        .surface = 0,
    };
    var host = stub.asHost();
    var ui: Ui = undefined;
    try ui.init(gpa, &config, &host);
    defer ui.deinit();
    ui.dry = true;
    ui.shown = .cli;
    try ui.editor.set("/serving");
    ui.enter();
    try std.testing.expectEqual(@as(usize, 1), stub.lines.items.len);
    try std.testing.expectEqualStrings("/serving", stub.lines.items[0]);
}

test "host asking event arms quit confirm and enter drains" {
    const gpa = std.testing.allocator;
    var stub: StubHost = .{ .gpa = gpa };
    defer stub.deinit();
    try stub.pushAsking(3);
    const url = "http://127.0.0.1:1/";
    const city = "c";
    const config = host_mod.CityConfig{
        .url_ptr = url.ptr,
        .url_len = url.len,
        .city_ptr = city.ptr,
        .city_len = city.len,
        .token_ptr = null,
        .token_len = 0,
        .surface = 0,
    };
    var host = stub.asHost();
    var ui: Ui = undefined;
    try ui.init(gpa, &config, &host);
    defer ui.deinit();
    ui.dry = true;
    ui.shown = .cli;
    ui.drainHost();
    try std.testing.expectEqual(@as(?u64, 3), ui.asking);
    ui.onKey(.{ .code = .enter });
    try std.testing.expect(ui.asking == null);
    try std.testing.expectEqual(@as(usize, 1), stub.tells.items.len);
    try std.testing.expectEqual(@backingInt(host_mod.LifecycleEvent.quit_drain), stub.tells.items[0]);
}
