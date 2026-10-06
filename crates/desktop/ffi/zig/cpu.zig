// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// The processor and job calls of the leaf (`crates/desktop/ffi/Spec.lean`
// D4): each export one whole call, answering a `Step` the way the
// exports of `leaf.zig` do. Nothing here reads a record: the CPU set
// records are copied out unread and walked in Rust.

const std = @import("std");
const windows = std.os.windows;
const Ended = @import("leaf.zig").Ended;
const Step = @import("step.zig").Step;

const BOOL = windows.BOOL;
const HANDLE = windows.HANDLE;

extern "kernel32" fn GetSystemCpuSetInformation(info: ?*anyopaque, len: u32, returned: *u32, process: ?HANDLE, flags: u32) callconv(.winapi) BOOL;
extern "kernel32" fn GetThreadGroupAffinity(thread: HANDLE, affinity: *GROUP_AFFINITY) callconv(.winapi) BOOL;
extern "kernel32" fn SetProcessInformation(process: HANDLE, class: u32, info: *const anyopaque, size: u32) callconv(.winapi) BOOL;
extern "kernel32" fn QueryInformationJobObject(job: HANDLE, class: u32, info: *anyopaque, size: u32, returned: ?*u32) callconv(.winapi) BOOL;
pub extern "kernel32" fn SetInformationJobObject(job: HANDLE, class: u32, info: *const anyopaque, size: u32) callconv(.winapi) BOOL;
extern "kernel32" fn CreateIoCompletionPort(file: HANDLE, existing: ?HANDLE, key: usize, threads: u32) callconv(.winapi) ?HANDLE;
extern "kernel32" fn GetQueuedCompletionStatus(port: HANDLE, bytes: *u32, key: *usize, overlapped: *?*anyopaque, wait_ms: u32) callconv(.winapi) BOOL;
extern "kernel32" fn CloseHandle(handle: HANDLE) callconv(.winapi) BOOL;

// The SDK's values for the processor and job calls, each written once,
// here (`crates/desktop/ffi/Spec.lean` D4).
const ERROR_INSUFFICIENT_BUFFER: u32 = 122;
const PROCESS_POWER_THROTTLING: u32 = 4;
const PROCESS_POWER_THROTTLING_CURRENT_VERSION: u32 = 1;
const PROCESS_POWER_THROTTLING_EXECUTION_SPEED: u32 = 0x1;
pub const JOB_OBJECT_EXTENDED_LIMIT_INFORMATION: u32 = 9;
pub const JOB_OBJECT_CPU_RATE_CONTROL_INFORMATION: u32 = 15;
pub const JOB_OBJECT_LIMIT_JOB_MEMORY: u32 = 0x200;
pub const JOB_OBJECT_CPU_RATE_CONTROL_ENABLE: u32 = 0x1;
const JOB_OBJECT_CPU_RATE_CONTROL_WEIGHT_BASED: u32 = 0x2;
const JOB_OBJECT_ASSOCIATE_COMPLETION_PORT_INFORMATION: u32 = 7;
const JOB_OBJECT_MSG_JOB_MEMORY_LIMIT: u32 = 10;
const WAIT_TIMEOUT: u32 = 258;
const INVALID_HANDLE_VALUE = windows.INVALID_HANDLE_VALUE;
/// The most messages one read takes off a port; the rest wait for the
/// next read, so one read is bounded however busy the job was.
const DRAIN_LIMIT: u32 = 65_536;

const JOBOBJECT_ASSOCIATE_COMPLETION_PORT = extern struct {
    CompletionKey: ?*anyopaque,
    CompletionPort: HANDLE,
};

const GROUP_AFFINITY = extern struct {
    Mask: usize = 0,
    Group: u16 = 0,
    Reserved: [3]u16 = .{ 0, 0, 0 },
};

const PROCESS_POWER_THROTTLING_STATE = extern struct {
    Version: u32,
    ControlMask: u32,
    StateMask: u32,
};

pub const JOBOBJECT_CPU_RATE_CONTROL_INFORMATION = extern struct {
    ControlFlags: u32,
    Weight: u32,
};

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

pub const JOBOBJECT_EXTENDED_LIMIT_INFORMATION = extern struct {
    BasicLimitInformation: JOBOBJECT_BASIC_LIMIT_INFORMATION = .{},
    IoInfo: IO_COUNTERS = .{},
    ProcessMemoryLimit: usize = 0,
    JobMemoryLimit: usize = 0,
    PeakProcessMemoryUsed: usize = 0,
    PeakJobMemoryUsed: usize = 0,
};

/// Every CPU set record of this machine, copied unread into `into` up to
/// `capacity` bytes; `found` is the length the whole answer needs.
/// `NoRoom` asks for a longer buffer; the records are parsed in Rust.
export fn sprawling_desktop_cpu_sets(into: [*]u8, capacity: usize, found: *usize, code: *u32) u32 {
    const len = std.math.cast(u32, capacity) orelse return (Ended{ .step = .Measuring }).answer(code);
    var returned: u32 = 0;
    const buffer: ?*anyopaque = if (len == 0) null else @ptrCast(into);
    const answered = GetSystemCpuSetInformation(buffer, len, &returned, windows.GetCurrentProcess(), 0);
    found.* = returned;
    if (answered != .FALSE) return Ended.finished.answer(code);
    const ended = Ended.failed(.CpuSets);
    if (ended.code == ERROR_INSUFFICIENT_BUFFER) return (Ended{ .step = .NoRoom, .code = ended.code }).answer(code);
    return ended.answer(code);
}

/// The calling thread's processor group and the processors of that group
/// it may run on.
export fn sprawling_desktop_thread_group(group: *u16, mask: *u64, code: *u32) u32 {
    var affinity: GROUP_AFFINITY = .{};
    if (GetThreadGroupAffinity(windows.GetCurrentThread(), &affinity) == .FALSE) return Ended.failed(.Affinity).answer(code);
    group.* = affinity.Group;
    mask.* = affinity.Mask;
    return Ended.finished.answer(code);
}

/// This process's execution speed is never throttled: the control bit set
/// and the state bit clear (`crates/sprawling/spec/Serving/Standing.lean` D40).
export fn sprawling_desktop_full_speed(code: *u32) u32 {
    const state: PROCESS_POWER_THROTTLING_STATE = .{
        .Version = PROCESS_POWER_THROTTLING_CURRENT_VERSION,
        .ControlMask = PROCESS_POWER_THROTTLING_EXECUTION_SPEED,
        .StateMask = 0,
    };
    if (SetProcessInformation(windows.GetCurrentProcess(), PROCESS_POWER_THROTTLING, &state, @sizeOf(PROCESS_POWER_THROTTLING_STATE)) == .FALSE)
        return Ended.failed(.Throttling).answer(code);
    return Ended.finished.answer(code);
}

/// A weighted CPU share for `job` (1 to 9; 5 is an even share) and, when
/// `memory` is not zero, a limit on the memory all its processes commit
/// together. The job's other limits are read first and written back as
/// they were.
export fn sprawling_desktop_job_share(job: ?HANDLE, weight: u32, memory: usize, code: *u32) u32 {
    const named = job orelse return (Ended{ .step = .JobShare }).answer(code);
    const rate: JOBOBJECT_CPU_RATE_CONTROL_INFORMATION = .{
        .ControlFlags = JOB_OBJECT_CPU_RATE_CONTROL_ENABLE | JOB_OBJECT_CPU_RATE_CONTROL_WEIGHT_BASED,
        .Weight = weight,
    };
    if (SetInformationJobObject(named, JOB_OBJECT_CPU_RATE_CONTROL_INFORMATION, &rate, @sizeOf(JOBOBJECT_CPU_RATE_CONTROL_INFORMATION)) == .FALSE)
        return Ended.failed(.JobShare).answer(code);
    if (memory == 0) return Ended.finished.answer(code);
    var limits: JOBOBJECT_EXTENDED_LIMIT_INFORMATION = .{};
    if (QueryInformationJobObject(named, JOB_OBJECT_EXTENDED_LIMIT_INFORMATION, &limits, @sizeOf(JOBOBJECT_EXTENDED_LIMIT_INFORMATION), null) == .FALSE)
        return Ended.failed(.JobShare).answer(code);
    limits.BasicLimitInformation.LimitFlags |= JOB_OBJECT_LIMIT_JOB_MEMORY;
    limits.JobMemoryLimit = memory;
    if (SetInformationJobObject(named, JOB_OBJECT_EXTENDED_LIMIT_INFORMATION, &limits, @sizeOf(JOBOBJECT_EXTENDED_LIMIT_INFORMATION)) == .FALSE)
        return Ended.failed(.JobShare).answer(code);
    return Ended.finished.answer(code);
}

/// A completion port attached to `job`, so the job's messages queue on it;
/// `port` receives the port's handle. A port that cannot be attached is
/// closed again, and nothing is left behind.
export fn sprawling_desktop_job_watch(job: ?HANDLE, port: *usize, code: *u32) u32 {
    const named = job orelse return (Ended{ .step = .JobWatch }).answer(code);
    const made = CreateIoCompletionPort(INVALID_HANDLE_VALUE, null, 0, 1) orelse return Ended.failed(.JobWatch).answer(code);
    const association: JOBOBJECT_ASSOCIATE_COMPLETION_PORT = .{ .CompletionKey = named, .CompletionPort = made };
    if (SetInformationJobObject(named, JOB_OBJECT_ASSOCIATE_COMPLETION_PORT_INFORMATION, &association, @sizeOf(JOBOBJECT_ASSOCIATE_COMPLETION_PORT)) == .FALSE) {
        const ended = Ended.failed(.JobWatch);
        _ = CloseHandle(made);
        return ended.answer(code);
    }
    port.* = @intFromPtr(made);
    return Ended.finished.answer(code);
}

/// Takes the messages queued on `port` without waiting, at most
/// `DRAIN_LIMIT` of them, and counts in `hits` those saying a process
/// of the job was refused at the job's memory limit.
export fn sprawling_desktop_job_memory_hits(port: usize, hits: *u32, code: *u32) u32 {
    if (port == 0) return (Ended{ .step = .JobWatch }).answer(code);
    const named: HANDLE = @ptrFromInt(port);
    var counted: u32 = 0;
    var taken: u32 = 0;
    while (taken < DRAIN_LIMIT) : (taken += 1) {
        var message: u32 = 0;
        var key: usize = 0;
        var overlapped: ?*anyopaque = null;
        if (GetQueuedCompletionStatus(named, &message, &key, &overlapped, 0) == .FALSE) {
            const ended = Ended.failed(.JobWatch);
            if (overlapped == null and ended.code == WAIT_TIMEOUT) break;
            hits.* = counted;
            return ended.answer(code);
        }
        if (message == JOB_OBJECT_MSG_JOB_MEMORY_LIMIT) counted += 1;
    }
    hits.* = counted;
    return Ended.finished.answer(code);
}

/// Closes a port `sprawling_desktop_job_watch` made.
export fn sprawling_desktop_job_unwatch(port: usize, code: *u32) u32 {
    if (port == 0) return (Ended{ .step = .JobWatch }).answer(code);
    if (CloseHandle(@ptrFromInt(port)) == .FALSE) return Ended.failed(.JobWatch).answer(code);
    return Ended.finished.answer(code);
}

test "the layouts the processor and job calls lend match the SDK's sizes" {
    try std.testing.expectEqual(@as(usize, 16), @sizeOf(GROUP_AFFINITY));
    try std.testing.expectEqual(@as(usize, 12), @sizeOf(PROCESS_POWER_THROTTLING_STATE));
    try std.testing.expectEqual(@as(usize, 144), @sizeOf(JOBOBJECT_EXTENDED_LIMIT_INFORMATION));
    try std.testing.expectEqual(@as(usize, 16), @sizeOf(JOBOBJECT_ASSOCIATE_COMPLETION_PORT));
}

test "a job share without a job is refused before any call" {
    var code: u32 = 0;
    try std.testing.expectEqual(@intFromEnum(Step.JobShare), sprawling_desktop_job_share(null, 5, 0, &code));
}

test "a watch without a job or a port is refused before any call" {
    var code: u32 = 0;
    var port: usize = 0;
    var hits: u32 = 0;
    try std.testing.expectEqual(@intFromEnum(Step.JobWatch), sprawling_desktop_job_watch(null, &port, &code));
    try std.testing.expectEqual(@intFromEnum(Step.JobWatch), sprawling_desktop_job_memory_hits(0, &hits, &code));
    try std.testing.expectEqual(@intFromEnum(Step.JobWatch), sprawling_desktop_job_unwatch(0, &code));
}
