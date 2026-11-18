// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// Mei child driver: spawn with stdin prompt, stream stderr lines, stop the
// Unix process group (pgid=0 at spawn). Windows: Job Object with
// KILL_ON_JOB_CLOSE so Esc ends the whole subtree (Console §8-174).

const std = @import("std");
const builtin = @import("builtin");
const windows = std.os.windows;

pub const Event = union(enum) {
    line: []u8,
    finished: Finished,
};

pub const Finished = struct {
    result: Result,
    cancelled: bool,
};

pub const Result = union(enum) {
    ok: []u8,
    err: []u8,

    pub fn deinit(self: Result, gpa: std.mem.Allocator) void {
        switch (self) {
            .ok => |s| gpa.free(s),
            .err => |s| gpa.free(s),
        }
    }
};

pub const Bus = struct {
    gpa: std.mem.Allocator,
    io: std.Io,
    mutex: std.Io.Mutex = .init,
    items: std.ArrayList(Event) = .empty,

    pub fn deinit(self: *Bus) void {
        self.mutex.lockUncancelable(self.io);
        defer self.mutex.unlock(self.io);
        for (self.items.items) |ev| freeEvent(self.gpa, ev);
        self.items.deinit(self.gpa);
    }

    pub fn push(self: *Bus, event: Event) void {
        self.mutex.lockUncancelable(self.io);
        defer self.mutex.unlock(self.io);
        self.items.append(self.gpa, event) catch {
            freeEvent(self.gpa, event);
        };
    }

    pub fn drainInto(self: *Bus, into: *std.ArrayList(Event)) void {
        self.mutex.lockUncancelable(self.io);
        defer self.mutex.unlock(self.io);
        into.appendSlice(self.gpa, self.items.items) catch {
            for (self.items.items) |ev| freeEvent(self.gpa, ev);
            self.items.clearRetainingCapacity();
            return;
        };
        self.items.clearRetainingCapacity();
    }
};

pub fn freeEvent(gpa: std.mem.Allocator, event: Event) void {
    switch (event) {
        .line => |s| gpa.free(s),
        .finished => |f| f.result.deinit(gpa),
    }
}

const Signals = struct {
    cancelled: std.atomic.Value(bool) = .init(false),
    done: std.atomic.Value(bool) = .init(false),
};

pub const Process = struct {
    gpa: std.mem.Allocator,
    io: std.Io,
    bus: *Bus,
    signals: *Signals,
    /// Unix process id (group leader when pgid=0). Windows: process HANDLE.
    id: ?std.process.Child.Id = null,
    /// Windows Job Object; closing it kills the assigned subtree.
    job: if (builtin.os.tag == .windows) ?windows.HANDLE else void =
        if (builtin.os.tag == .windows) null else {},
    driver: ?std.Thread = null,

    pub fn spawn(
        gpa: std.mem.Allocator,
        io: std.Io,
        bus: *Bus,
        program: []const u8,
        prompt: []const u8,
        resume_path: ?[]const u8,
        capture_directory: ?[]const u8,
        environ: ?*const std.process.Environ.Map,
    ) SpawnResult {
        var argv_list: std.ArrayList([]const u8) = .empty;
        defer argv_list.deinit(gpa);
        argv_list.append(gpa, program) catch {
            return .{ .err = configFailure(gpa, "allocate argv", program) };
        };
        if (resume_path) |path| {
            argv_list.append(gpa, "--resume") catch {
                return .{ .err = configFailure(gpa, "allocate argv", program) };
            };
            argv_list.append(gpa, path) catch {
                return .{ .err = configFailure(gpa, "allocate argv", program) };
            };
        }

        var child_environment = if (capture_directory) |directory| blk: {
            const base = environ orelse return .{ .err = configFailure(gpa, "missing base environment", program) };
            break :blk captureEnvironment(gpa, base, directory) catch return .{ .err = configFailure(gpa, "set capture directory", program) };
        } else null;
        defer if (child_environment) |*environment| environment.deinit();

        const child = std.process.spawn(io, .{
            .argv = argv_list.items,
            .environ_map = if (child_environment) |*environment| environment else null,
            .stdin = .pipe,
            .stdout = .pipe,
            .stderr = .pipe,
            .pgid = if (builtin.os.tag == .windows) null else 0,
            .create_no_window = true,
        }) catch |err| {
            return .{ .err = configFailure(gpa, @errorName(err), program) };
        };

        const signals = gpa.create(Signals) catch {
            var c = child;
            c.kill(io);
            return .{ .err = configFailure(gpa, "allocate signals", program) };
        };
        signals.* = .{};

        const prompt_owned = gpa.dupe(u8, prompt) catch {
            var c = child;
            c.kill(io);
            gpa.destroy(signals);
            return .{ .err = configFailure(gpa, "duplicate prompt", program) };
        };

        const ctx = gpa.create(DriverCtx) catch {
            gpa.free(prompt_owned);
            var c = child;
            c.kill(io);
            gpa.destroy(signals);
            return .{ .err = configFailure(gpa, "allocate driver", program) };
        };
        ctx.* = .{
            .gpa = gpa,
            .io = io,
            .bus = bus,
            .child = child,
            .prompt = prompt_owned,
            .signals = signals,
        };

        var process: Process = .{
            .gpa = gpa,
            .io = io,
            .bus = bus,
            .signals = signals,
            .id = child.id,
        };

        if (comptime builtin.os.tag == .windows) {
            const job = WinJob.create(child.id) catch {
                gpa.free(prompt_owned);
                ctx.child.kill(io);
                gpa.destroy(ctx);
                gpa.destroy(signals);
                return .{ .err = configFailure(gpa, "create Mei job object", program) };
            };
            process.job = job;
        }

        process.driver = std.Thread.spawn(.{}, driveThread, .{ctx}) catch {
            gpa.free(prompt_owned);
            process.closeJob();
            ctx.child.kill(io);
            gpa.destroy(ctx);
            gpa.destroy(signals);
            return .{ .err = configFailure(gpa, "start Mei driver", program) };
        };
        return .{ .ok = process };
    }

    pub fn requestStop(self: *Process) StopResult {
        if (self.signals.done.swap(true, .seq_cst)) return .ok;
        self.signals.cancelled.store(true, .seq_cst);
        return self.terminateTree();
    }

    pub fn isFinished(self: *const Process) bool {
        return self.signals.done.load(.seq_cst) and self.driver == null;
    }

    pub fn reap(self: *Process) StopResult {
        const thread = self.driver orelse return .ok;
        self.driver = null;
        thread.join();
        return .ok;
    }

    /// Join the driver and free signals without sending another stop.
    pub fn finish(self: *Process) void {
        _ = self.reap();
        self.closeJob();
        self.gpa.destroy(self.signals);
        self.* = undefined;
    }

    pub fn deinit(self: *Process) void {
        _ = self.requestStop();
        _ = self.reap();
        self.closeJob();
        self.gpa.destroy(self.signals);
        self.* = undefined;
    }

    fn closeJob(self: *Process) void {
        if (comptime builtin.os.tag == .windows) {
            if (self.job) |job| {
                _ = WinJob.CloseHandle(job);
                self.job = null;
            }
        }
    }

    fn terminateTree(self: *Process) StopResult {
        const id = self.id orelse return .ok;
        if (comptime builtin.os.tag == .windows) {
            if (self.job != null) {
                self.closeJob();
                return .ok;
            }
            _ = windows.ntdll.NtTerminateProcess(id, @fromBackingInt(1));
            return .ok;
        }
        std.posix.kill(-id, .KILL) catch |err| switch (err) {
            error.ProcessNotFound => {},
            else => {
                const msg = std.fmt.allocPrint(self.gpa, "terminate Mei process group {d}: {s}", .{ id, @errorName(err) }) catch
                    self.gpa.dupe(u8, "terminate Mei process group") catch @panic("oom");
                return .{ .err = msg };
            },
        };
        return .ok;
    }
};

const WinJob = if (builtin.os.tag == .windows) struct {
    const JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE: u32 = 0x0000_2000;
    const JobObjectExtendedLimitInformation: u32 = 9;

    const JOBOBJECT_BASIC_LIMIT_INFORMATION = extern struct {
        PerProcessUserTimeLimit: i64 = 0,
        PerJobUserTimeLimit: i64 = 0,
        LimitFlags: u32 = 0,
        MinimumWorkingSetSize: usize = 0,
        MaximumWorkingSetSize: usize = 0,
        ActiveProcessLimit: u32 = 0,
        Affinity: usize = 0,
        PriorityClass: u32 = 0,
        SchedulingClass: u32 = 0,
    };

    const IO_COUNTERS = extern struct {
        ReadOperationCount: u64 = 0,
        WriteOperationCount: u64 = 0,
        OtherOperationCount: u64 = 0,
        ReadTransferCount: u64 = 0,
        WriteTransferCount: u64 = 0,
        OtherTransferCount: u64 = 0,
    };

    const JOBOBJECT_EXTENDED_LIMIT_INFORMATION = extern struct {
        BasicLimitInformation: JOBOBJECT_BASIC_LIMIT_INFORMATION = .{},
        IoInfo: IO_COUNTERS = .{},
        ProcessMemoryLimit: usize = 0,
        JobMemoryLimit: usize = 0,
        PeakProcessMemoryUsed: usize = 0,
        PeakJobMemoryUsed: usize = 0,
    };

    extern "kernel32" fn CreateJobObjectW(
        lpJobAttributes: ?*anyopaque,
        lpName: ?[*:0]const u16,
    ) callconv(.winapi) ?windows.HANDLE;

    extern "kernel32" fn SetInformationJobObject(
        hJob: windows.HANDLE,
        JobObjectInformationClass: u32,
        lpJobObjectInformation: *anyopaque,
        cbJobObjectInformationLength: windows.DWORD,
    ) callconv(.winapi) windows.BOOL;

    extern "kernel32" fn AssignProcessToJobObject(
        hJob: windows.HANDLE,
        hProcess: windows.HANDLE,
    ) callconv(.winapi) windows.BOOL;

    extern "kernel32" fn CloseHandle(hObject: windows.HANDLE) callconv(.winapi) windows.BOOL;

    fn create(process: ?windows.HANDLE) !windows.HANDLE {
        const proc = process orelse return error.NoProcess;
        const job = CreateJobObjectW(null, null) orelse return error.CreateJob;
        errdefer _ = CloseHandle(job);

        var info = JOBOBJECT_EXTENDED_LIMIT_INFORMATION{};
        info.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
        if (!SetInformationJobObject(
            job,
            JobObjectExtendedLimitInformation,
            &info,
            @sizeOf(JOBOBJECT_EXTENDED_LIMIT_INFORMATION),
        ).toBool()) return error.SetJobInfo;
        if (!AssignProcessToJobObject(job, proc).toBool()) return error.AssignJob;
        return job;
    }
} else struct {
    fn create(_: anytype) !void {
        return error.Unsupported;
    }
};

pub const SpawnResult = union(enum) {
    ok: Process,
    err: []u8,
};

pub const StopResult = union(enum) {
    ok: void,
    err: []u8,
};

const DriverCtx = struct {
    gpa: std.mem.Allocator,
    io: std.Io,
    bus: *Bus,
    child: std.process.Child,
    prompt: []u8,
    signals: *Signals,
};

fn driveThread(ctx: *DriverCtx) void {
    defer {
        ctx.gpa.free(ctx.prompt);
        ctx.gpa.destroy(ctx);
    }
    const finished = drive(ctx);
    ctx.signals.done.store(true, .seq_cst);
    ctx.bus.push(.{ .finished = finished });
}

fn drive(ctx: *DriverCtx) Finished {
    var child = ctx.child;
    const answer = driveOutput(ctx.gpa, ctx.io, &child, ctx.prompt, ctx.bus) catch |err| {
        if (builtin.os.tag != .windows) {
            if (child.id) |pid| {
                _ = std.posix.kill(-pid, .KILL) catch {};
            }
        } else {
            child.kill(ctx.io);
        }
        const msg = std.fmt.allocPrint(ctx.gpa, "{s}", .{@errorName(err)}) catch
            ctx.gpa.dupe(u8, "Mei I/O failure") catch unreachable;
        return .{
            .result = .{ .err = msg },
            .cancelled = ctx.signals.cancelled.load(.seq_cst),
        };
    };
    return .{
        .result = answer,
        .cancelled = ctx.signals.cancelled.load(.seq_cst),
    };
}

fn driveOutput(
    gpa: std.mem.Allocator,
    io: std.Io,
    child: *std.process.Child,
    prompt: []const u8,
    bus: *Bus,
) !Result {
    if (child.stdin) |stdin| {
        try stdin.writeStreamingAll(io, prompt);
        stdin.close(io);
        child.stdin = null;
    } else return error.OpenStdin;

    const stdout_file = child.stdout orelse return error.OpenStdout;
    const stderr_file = child.stderr orelse return error.OpenStderr;

    const StdoutCtx = struct {
        gpa: std.mem.Allocator,
        io: std.Io,
        file: std.Io.File,
        out: ?[]u8 = null,
        err: ?anyerror = null,
    };
    var stdout_ctx: StdoutCtx = .{ .gpa = gpa, .io = io, .file = stdout_file };
    const stdout_thread = try std.Thread.spawn(.{}, struct {
        fn run(c: *StdoutCtx) void {
            var buf: [4096]u8 = undefined;
            var reader = c.file.readerStreaming(c.io, &buf);
            c.out = reader.interface.allocRemaining(c.gpa, .unlimited) catch |e| {
                c.err = e;
                return;
            };
        }
    }.run, .{&stdout_ctx});
    var joined = false;
    errdefer {
        if (!joined) {
            child.kill(io);
            stdout_thread.join();
            joined = true;
            if (stdout_ctx.out) |out| gpa.free(out);
            stdout_ctx.out = null;
        }
    }

    var err_buf: [4096]u8 = undefined;
    var stderr_reader = stderr_file.readerStreaming(io, &err_buf);
    while (true) {
        const line = stderr_reader.interface.takeDelimiterExclusive('\n') catch |err| switch (err) {
            error.EndOfStream => break,
            error.StreamTooLong => {
                _ = stderr_reader.interface.discardRemaining() catch {};
                break;
            },
            else => return err,
        };
        const owned = try gpa.dupe(u8, std.mem.trimEnd(u8, line, "\r"));
        bus.push(.{ .line = owned });
    }
    stderr_file.close(io);
    child.stderr = null;

    const term = try child.wait(io);
    stdout_thread.join();
    joined = true;
    stdout_file.close(io);
    child.stdout = null;

    if (stdout_ctx.err) |e| return e;
    const answer = stdout_ctx.out orelse try gpa.dupe(u8, "");

    if (term.success()) {
        return .{ .ok = answer };
    }
    gpa.free(answer);
    var msg_buf: [128]u8 = undefined;
    const msg = switch (term) {
        .exited => |code| try std.fmt.bufPrint(&msg_buf, "Mei exited with code {d}", .{code}),
        .signal => |sig| try std.fmt.bufPrint(&msg_buf, "Mei terminated with signal {d}", .{@backingInt(sig)}),
        .stopped => |sig| try std.fmt.bufPrint(&msg_buf, "Mei stopped with signal {d}", .{@backingInt(sig)}),
        .unknown => "Mei terminated unexpectedly",
    };
    return .{ .err = try gpa.dupe(u8, msg) };
}

fn captureEnvironment(gpa: std.mem.Allocator, base: *const std.process.Environ.Map, directory: []const u8) !std.process.Environ.Map {
    var child = try base.clone(gpa);
    errdefer child.deinit();
    try child.put("MEI_CAPTURE_DIR", directory);
    return child;
}

fn configFailure(gpa: std.mem.Allocator, subject: []const u8, program: []const u8) []u8 {
    return std.fmt.allocPrint(gpa,
        \\```toml
        \\code = "E_MEI_CONFIG"
        \\description = "start Mei: {s}"
        \\cause = "MEI_BIN must name an executable Mei binary\npath = {s}\nvalue = {s}"
        \\resolution = "set MEI_BIN to the built binary, for example `MEI_BIN=target/debug/mei`, put `mei` beside `calyx`, or put `mei` on PATH"
        \\```
    , .{ subject, program, program }) catch gpa.dupe(u8, subject) catch @panic("oom");
}

pub fn buildArgv(
    gpa: std.mem.Allocator,
    program: []const u8,
    resume_path: ?[]const u8,
) ![]const []const u8 {
    var list: std.ArrayList([]const u8) = .empty;
    errdefer list.deinit(gpa);
    try list.append(gpa, program);
    if (resume_path) |path| {
        try list.append(gpa, "--resume");
        try list.append(gpa, path);
    }
    return try list.toOwnedSlice(gpa);
}

test "resume path is passed on the argv" {
    const gpa = std.testing.allocator;
    const argv = try buildArgv(gpa, "mei", "/tmp/chosen.jsonl");
    defer gpa.free(argv);
    try std.testing.expectEqual(@as(usize, 3), argv.len);
    try std.testing.expectEqualStrings("mei", argv[0]);
    try std.testing.expectEqualStrings("--resume", argv[1]);
    try std.testing.expectEqualStrings("/tmp/chosen.jsonl", argv[2]);
}

test "capture directory is copied into only the child environment" {
    const gpa = std.testing.allocator;
    var base = std.process.Environ.Map.init(gpa);
    defer base.deinit();
    try base.put("KEEP", "yes");
    var child = try captureEnvironment(gpa, &base, "session.captures/run-1");
    defer child.deinit();
    try std.testing.expectEqualStrings("yes", child.get("KEEP").?);
    try std.testing.expectEqualStrings("session.captures/run-1", child.get("MEI_CAPTURE_DIR").?);
    try std.testing.expect(base.get("MEI_CAPTURE_DIR") == null);
}

test "missing program yields a structured diagnostic" {
    const io = std.testing.io;
    const gpa = std.testing.allocator;
    var bus: Bus = .{ .gpa = gpa, .io = io };
    defer bus.deinit();
    switch (Process.spawn(gpa, io, &bus, "mei-definitely-not-on-path-xyz", "prompt", null, null, null)) {
        .ok => |proc| {
            var p = proc;
            defer p.deinit();
            try std.testing.expect(false);
        },
        .err => |diag| {
            defer gpa.free(diag);
            try std.testing.expect(std.mem.indexOf(u8, diag, "```toml") != null);
            try std.testing.expect(std.mem.indexOf(u8, diag, "E_MEI_CONFIG") != null);
            try std.testing.expect(std.mem.indexOf(u8, diag, "mei-definitely-not-on-path-xyz") != null);
        },
    }
}

test "stopping a process group reaps its driver" {
    if (builtin.os.tag == .windows) return;
    const io = std.testing.io;
    const gpa = std.testing.allocator;
    var bus: Bus = .{ .gpa = gpa, .io = io };
    defer bus.deinit();

    var process = switch (Process.spawn(gpa, io, &bus, "sh", "sleep 30", null, null, null)) {
        .ok => |p| p,
        .err => |e| {
            defer gpa.free(e);
            return error.TestUnexpectedResult;
        },
    };
    defer process.deinit();

    switch (process.requestStop()) {
        .ok => {},
        .err => |e| {
            defer gpa.free(e);
            return error.TestUnexpectedResult;
        },
    }
    _ = process.reap();

    var drained: std.ArrayList(Event) = .empty;
    defer {
        for (drained.items) |ev| freeEvent(gpa, ev);
        drained.deinit(gpa);
    }
    var spins: usize = 0;
    while (spins < 200) : (spins += 1) {
        bus.drainInto(&drained);
        for (drained.items) |ev| {
            if (ev == .finished) {
                try std.testing.expect(ev.finished.cancelled);
                return;
            }
        }
        io.sleep(std.Io.Duration.fromMilliseconds(10), .real) catch {};
    }
    try std.testing.expect(false);
}
