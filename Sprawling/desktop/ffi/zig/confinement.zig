// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// Platform effects; admission, limits and argv policy remain in Rust.
const std = @import("std");
const w = std.os.windows;
const api = @import("confinement_api.zig");
const cpu = @import("cpu.zig");
const grants = @import("confinement_grants.zig");
const allocator = std.heap.page_allocator;
const Packet = struct { strings: [7][:0]const u16, environment: []const u16 };

fn packet(units: []const u16) ?Packet {
    var remaining = units;
    var strings: [7][:0]const u16 = undefined;
    for (&strings, 0..) |*part, index| {
        const end = std.mem.indexOfScalar(u16, remaining, 0) orelse return null;
        if (end == 0 and index != 6) return null;
        part.* = remaining[0..end :0];
        remaining = remaining[end + 1 ..];
    }
    if (remaining.len < 2 or remaining[remaining.len - 1] != 0 or remaining[remaining.len - 2] != 0) return null;
    const environment = remaining;
    remaining = remaining[0 .. remaining.len - 1];
    if (remaining.len == 1 and remaining[0] == 0) return .{ .strings = strings, .environment = environment };
    while (remaining.len != 0) {
        const end = std.mem.indexOfScalar(u16, remaining, 0) orelse return null;
        if (end == 0) return null;
        const equal = std.mem.indexOfScalar(u16, remaining[0..end], '=') orelse return null;
        if (equal == 0) return null;
        remaining = remaining[end + 1 ..];
    }
    return .{ .strings = strings, .environment = environment };
}

export fn sprawling_native_packet_valid(text: [*]const u16, units: usize) u32 {
    return @intFromBool(packet(text[0..units]) != null);
}
fn remember(record: *api.Record, code: u32) void {
    if (record.cleanup_error == 0) record.cleanup_error = code;
}
fn handle(raw: usize) api.HANDLE {
    return @ptrFromInt(raw);
}
fn lastError() u32 {
    return @backingInt(w.GetLastError());
}
fn close(raw: *usize) u32 {
    if (raw.* == 0) return 0;
    if (api.CloseHandle(handle(raw.*)) == .FALSE) return lastError();
    raw.* = 0;
    return 0;
}
fn phase(record: *api.Record, name: []const u8) void {
    @memset(&record.failure_phase, 0);
    const count = @min(name.len, record.failure_phase.len);
    @memcpy(record.failure_phase[0..count], name[0..count]);
}

fn acl(directory: [*:0]const u16, sid: *anyopaque, record: *api.Record) u32 {
    phase(record, @src().fn_name);
    var old: ?*anyopaque = null;
    var descriptor: ?*anyopaque = null;
    const read = api.GetNamedSecurityInfoW(directory, api.object_file, api.dacl_information | api.label_information, null, null, &old, null, &descriptor);
    if (read != 0) return read;
    record.root_security = @intFromPtr(descriptor orelse return api.invalid_parameter);
    const entry: api.Entry = .{ .trustee = .{ .name = sid } };
    var changed: ?*anyopaque = null;
    const set = api.SetEntriesInAclW(1, &entry, old, &changed);
    if (set != 0) return set;
    defer remember(record, if (api.LocalFree(changed) == null) 0 else lastError());
    return api.SetNamedSecurityInfoW(directory, api.object_file, api.dacl_information, null, null, changed, null);
}
fn integrity(directory: [*:0]const u16, record: *api.Record) u32 {
    phase(record, @src().fn_name);
    const sddl = "S:(ML;OICI;NW;;;LW)";
    const low = comptime block: {
        var units: [sddl.len:0]u16 = undefined;
        _ = std.unicode.utf8ToUtf16Le(&units, sddl) catch @compileError("invalid integrity label");
        units[sddl.len] = 0;
        break :block units;
    };
    var descriptor: ?*anyopaque = null;
    if (api.ConvertStringSecurityDescriptorToSecurityDescriptorW(&low, 1, &descriptor, null) == .FALSE) return lastError();
    defer remember(record, if (api.LocalFree(descriptor) == null) 0 else lastError());
    var present: api.BOOL = .FALSE;
    var defaulted: api.BOOL = .FALSE;
    var label: ?*anyopaque = null;
    if (api.GetSecurityDescriptorSacl(descriptor orelse return api.invalid_parameter, &present, &label, &defaulted) == .FALSE) return lastError();
    if (present == .FALSE or label == null) return api.invalid_parameter;
    return api.SetNamedSecurityInfoW(directory, api.object_file, api.label_information, null, null, null, label);
}

fn limits(record: *api.Record) u32 {
    phase(record, @src().fn_name);
    const job = api.CreateJobObjectW(null, null) orelse return lastError();
    record.job = @intFromPtr(job);
    // The run Job holds the memory ceiling; an equal limit here would take
    // its refusal message away from the run Job's watch (runtime Exec.lean D95).
    var closing: cpu.JOBOBJECT_EXTENDED_LIMIT_INFORMATION = .{};
    closing.BasicLimitInformation.LimitFlags = api.kill_on_close;
    if (cpu.SetInformationJobObject(job, cpu.JOB_OBJECT_EXTENDED_LIMIT_INFORMATION, &closing, @sizeOf(@TypeOf(closing))) == .FALSE) return lastError();
    const rate: cpu.JOBOBJECT_CPU_RATE_CONTROL_INFORMATION = .{
        .ControlFlags = cpu.JOB_OBJECT_CPU_RATE_CONTROL_ENABLE | api.hard_cap,
        .Weight = @intCast(record.cpu),
    };
    if (cpu.SetInformationJobObject(job, cpu.JOB_OBJECT_CPU_RATE_CONTROL_INFORMATION, &rate, @sizeOf(@TypeOf(rate))) == .FALSE) return lastError();
    var actual_closing: cpu.JOBOBJECT_EXTENDED_LIMIT_INFORMATION = .{};
    if (api.QueryInformationJobObject(job, cpu.JOB_OBJECT_EXTENDED_LIMIT_INFORMATION, &actual_closing, @sizeOf(@TypeOf(actual_closing)), null) == .FALSE) return lastError();
    if (actual_closing.BasicLimitInformation.LimitFlags & closing.BasicLimitInformation.LimitFlags != closing.BasicLimitInformation.LimitFlags) return 5;
    var actual_rate = std.mem.zeroes(cpu.JOBOBJECT_CPU_RATE_CONTROL_INFORMATION);
    if (api.QueryInformationJobObject(job, cpu.JOB_OBJECT_CPU_RATE_CONTROL_INFORMATION, &actual_rate, @sizeOf(@TypeOf(actual_rate)), null) == .FALSE) return lastError();
    if (actual_rate.ControlFlags != rate.ControlFlags or actual_rate.Weight != rate.Weight) return 5;
    return 0;
}
fn file(path: [*:0]const u16, access: u32, creation: u32) ?api.HANDLE {
    const security: api.Security = .{};
    const opened = api.CreateFileW(path, access, api.file_share_read | api.file_share_write, &security, creation, api.file_normal, null);
    if (opened == w.INVALID_HANDLE_VALUE) return null;
    return opened;
}
fn identity(process: api.HANDLE, sid: *anyopaque, record: *api.Record) u32 {
    phase(record, @src().fn_name);
    var token: api.HANDLE = undefined;
    if (api.OpenProcessToken(process, api.token_query, &token) == .FALSE) return lastError();
    defer remember(record, if (api.CloseHandle(token) != .FALSE) 0 else lastError());
    var appcontainer: u32 = 0;
    var returned: u32 = 0;
    if (api.GetTokenInformation(token, api.token_is_appcontainer, &appcontainer, @sizeOf(u32), &returned) == .FALSE) return lastError();
    if (appcontainer == 0) return 5;
    var size: u32 = 0;
    if (api.GetTokenInformation(token, api.token_appcontainer_sid, &appcontainer, 0, &size) != .FALSE or lastError() != 122 or size == 0) return api.invalid_parameter;
    const storage = allocator.alignedAlloc(u8, .of(usize), size) catch return 8;
    defer allocator.free(storage);
    if (api.GetTokenInformation(token, api.token_appcontainer_sid, @ptrCast(storage.ptr), size, &returned) == .FALSE) return lastError();
    const principal: *const extern struct { sid: ?*anyopaque } = @ptrCast(storage.ptr);
    const found = principal.sid orelse return 5;
    if (api.EqualSid(found, sid) == .FALSE) return 5;
    return 0;
}
fn membership(process: api.HANDLE, job: api.HANDLE) u32 {
    var member: api.BOOL = .FALSE;
    if (api.IsProcessInJob(process, job, &member) == .FALSE) return lastError();
    if (member == .FALSE) return 5;
    return 0;
}

export fn sprawling_native_may_resume(record: *const api.Record, bytes: usize) u32 {
    if (bytes != @sizeOf(api.Record)) return 0;
    return @intFromBool(record.run_assigned == 1 and record.command_assigned == 1 and record.identity_verified == 1);
}

fn start(parts: Packet, sid: *anyopaque, record: *api.Record) u32 {
    phase(record, @src().fn_name);
    const command = allocator.dupeSentinel(u16, parts.strings[1], 0) catch return 8;
    defer allocator.free(command);
    phase(record, "stdin");
    const input = file(api.nul, api.file_read, api.open_existing) orelse return lastError();
    defer remember(record, if (api.CloseHandle(input) != .FALSE) 0 else lastError());
    phase(record, "stdout");
    const output = file(parts.strings[4].ptr, api.file_write, api.create_always) orelse return lastError();
    defer remember(record, if (api.CloseHandle(output) != .FALSE) 0 else lastError());
    phase(record, "stderr");
    const errors = file(parts.strings[5].ptr, api.file_write, api.create_always) orelse return lastError();
    defer remember(record, if (api.CloseHandle(errors) != .FALSE) 0 else lastError());
    var size: usize = 0;
    phase(record, "measure attributes");
    const measured = api.InitializeProcThreadAttributeList(null, 2, 0, &size);
    if (measured != .FALSE or lastError() != 122 or size == 0) return api.invalid_parameter;
    const storage = allocator.alignedAlloc(u8, .of(usize), size) catch return 8;
    defer allocator.free(storage);
    const attributes: *anyopaque = @ptrCast(storage.ptr);
    phase(record, "initialize attributes");
    if (api.InitializeProcThreadAttributeList(attributes, 2, 0, &size) == .FALSE) return lastError();
    defer api.DeleteProcThreadAttributeList(attributes);
    var capabilities: api.Capabilities = .{ .sid = sid };
    phase(record, "security capabilities");
    if (api.UpdateProcThreadAttribute(attributes, 0, api.security_capabilities, &capabilities, @sizeOf(api.Capabilities), null, null) == .FALSE) return lastError();
    var inherited = [_]api.HANDLE{ input, output, errors };
    phase(record, "inherited handles");
    if (api.UpdateProcThreadAttribute(attributes, 0, api.handle_list, &inherited, @sizeOf(@TypeOf(inherited)), null, null) == .FALSE) return lastError();
    var startup: api.Startup = .{ .stdin = input, .stdout = output, .stderr = errors, .attributes = attributes };
    var process: api.Process = undefined;
    phase(record, "CreateProcessW");
    if (api.CreateProcessW(parts.strings[0].ptr, command.ptr, null, null, api.BOOL.TRUE, api.suspended | api.unicode_environment | api.extended_startup | api.no_window | api.below_normal, @ptrCast(@constCast(parts.environment.ptr)), parts.strings[2].ptr, &startup, &process) == .FALSE) return lastError();
    record.process = @intFromPtr(process.process);
    record.pid = process.pid;
    defer remember(record, if (api.CloseHandle(process.thread) != .FALSE) 0 else lastError());
    phase(record, "assign run Job");
    if (api.AssignProcessToJobObject(handle(record.parent_job), process.process) == .FALSE) return lastError();
    phase(record, "assign command Job");
    if (api.AssignProcessToJobObject(handle(record.job), process.process) == .FALSE) return lastError();
    const token = identity(process.process, sid, record);
    if (token != 0) return token;
    record.identity_verified = 1;
    const parent = membership(process.process, handle(record.parent_job));
    if (parent != 0) return parent;
    record.run_assigned = 1;
    const child = membership(process.process, handle(record.job));
    if (child != 0) return child;
    record.command_assigned = 1;
    if (sprawling_native_may_resume(record, @sizeOf(api.Record)) == 0) return 5;
    phase(record, "ResumeThread");
    if (api.ResumeThread(process.thread) == std.math.maxInt(u32)) return lastError();
    return 0;
}
export fn sprawling_native_launch(text: [*]const u16, units: usize, record: *api.Record, bytes: usize) u32 {
    if (bytes != @sizeOf(api.Record)) return api.invalid_parameter;
    phase(record, @src().fn_name);
    if (record.cpu == 0 or record.cpu > 10_000 or record.parent_job == 0 or record.job != 0 or record.process != 0) return api.invalid_parameter;
    const parts = packet(text[0..units]) orelse return api.invalid_parameter;
    var sid: ?*anyopaque = null;
    const profile = api.CreateAppContainerProfile(parts.strings[3].ptr, parts.strings[3].ptr, parts.strings[3].ptr, null, 0, &sid);
    if (profile < 0) return @bitCast(profile);
    record.profile_created = 1;
    const named = sid orelse {
        record.cleanup_error = sprawling_native_cleanup(record, bytes, parts.strings[2].ptr, parts.strings[2].len + parts.strings[3].len + 2);
        return api.invalid_parameter;
    };
    defer remember(record, if (api.FreeSid(named) == null) 0 else lastError());
    var result = limits(record);
    if (result == 0) result = acl(parts.strings[2].ptr, named, record);
    if (result == 0) result = integrity(parts.strings[2].ptr, record);
    if (result == 0) {
        phase(record, "toolchain grants");
        result = grants.grant(parts.strings[6], named, record);
    }
    if (result == 0) result = start(parts, named, record);
    if (result == 0 and record.cleanup_error != 0) result = @intCast(record.cleanup_error);
    if (result != 0) {
        const failed_phase = record.failure_phase;
        remember(record, sprawling_native_cleanup(record, bytes, parts.strings[2].ptr, parts.strings[2].len + parts.strings[3].len + 2));
        record.failure_phase = failed_phase;
    }
    return result;
}
export fn sprawling_native_poll(record: *api.Record, bytes: usize) u32 {
    if (bytes != @sizeOf(api.Record) or record.process == 0) return api.invalid_parameter;
    const wait = api.WaitForSingleObject(handle(record.process), 0);
    if (wait == api.wait_timeout) return 0;
    if (wait != 0) return lastError();
    var code: u32 = 0;
    if (api.GetExitCodeProcess(handle(record.process), &code) == .FALSE) return lastError();
    record.exit = code;
    record.finished = 1;
    return 0;
}
export fn sprawling_native_terminate(record: *api.Record, bytes: usize) u32 {
    if (bytes != @sizeOf(api.Record)) return api.invalid_parameter;
    if (record.job == 0) return 0;
    if (api.TerminateJobObject(handle(record.job), 1) == .FALSE) return lastError();
    return 0;
}
fn waitTree(record: *api.Record) u32 {
    if (record.job == 0) return 0;
    var capacity: usize = 16;
    for (0..4) |_| {
        const storage = allocator.alignedAlloc(u8, .of(usize), capacity) catch return 8;
        defer allocator.free(storage);
        @memset(storage, 0);
        const count: *const extern struct { assigned: u32, listed: u32 } = @ptrCast(storage.ptr);
        const bytes = std.math.cast(u32, capacity) orelse return api.invalid_parameter;
        if (api.QueryInformationJobObject(handle(record.job), api.process_list, @ptrCast(storage.ptr), bytes, null) == .FALSE) {
            const code = lastError();
            if (code != api.more_data) return code;
            const product = @mulWithOverflow(@as(usize, count.assigned), @sizeOf(usize));
            const size = @addWithOverflow(product[0], @sizeOf(@TypeOf(count.*)));
            if (product[1] != 0 or size[1] != 0 or size[0] <= capacity) return api.invalid_parameter;
            capacity = size[0];
            continue;
        }
        const offset = @sizeOf(@TypeOf(count.*));
        const ids: [*]const usize = @ptrCast(@alignCast(storage.ptr + offset));
        if (count.listed > (capacity - offset) / @sizeOf(usize)) return api.invalid_parameter;
        for (ids[0..count.listed]) |pid| {
            const named = std.math.cast(u32, pid) orelse return api.invalid_parameter;
            const process = api.OpenProcess(api.synchronize, .FALSE, named) orelse {
                const code = lastError();
                if (code == api.invalid_parameter) continue;
                return code;
            };
            defer remember(record, if (api.CloseHandle(process) != .FALSE) 0 else lastError());
            const wait = api.WaitForSingleObject(process, api.cleanup_wait_ms);
            if (wait == api.wait_timeout) return api.wait_timeout;
            if (wait != 0) return lastError();
        }
        return 0;
    }
    return api.more_data;
}

fn restore(directory: [*:0]const u16, record: *api.Record) u32 {
    phase(record, @src().fn_name);
    if (record.root_security == 0) return 0;
    const descriptor: *anyopaque = @ptrFromInt(record.root_security);
    var present: api.BOOL = .FALSE;
    var defaulted: api.BOOL = .FALSE;
    var dacl: ?*anyopaque = null;
    var label: ?*anyopaque = null;
    if (api.GetSecurityDescriptorDacl(descriptor, &present, &dacl, &defaulted) == .FALSE) return lastError();
    if (api.GetSecurityDescriptorSacl(descriptor, &present, &label, &defaulted) == .FALSE) return lastError();
    const restored = api.SetNamedSecurityInfoW(directory, api.object_file, api.dacl_information | api.label_information, null, null, dacl, label);
    if (restored != 0 and restored != 2 and restored != 3) return restored;
    if (api.LocalFree(descriptor) != null) return lastError();
    record.root_security = 0;
    return 0;
}

export fn sprawling_native_cleanup(record: *api.Record, bytes: usize, context: [*]const u16, units: usize) u32 {
    if (bytes != @sizeOf(api.Record) or units == 0 or context[units - 1] != 0) return api.invalid_parameter;
    const end = std.mem.indexOfScalar(u16, context[0..units], 0) orelse return api.invalid_parameter;
    if (end == 0 or end + 1 >= units) return api.invalid_parameter;
    const directory = context[0..end :0];
    const profile = context[end + 1 .. units - 1 :0];
    return cleanup(record, .{ .directory = directory, .profile = profile }, .{
        .revoke = grants.revoke,
        .root = api.TerminateProcess,
        .wait = api.WaitForSingleObject,
        .terminate = sprawling_native_terminate,
        .tree = waitTree,
        .close = close,
        .restore = restore,
        .delete = api.DeleteAppContainerProfile,
    });
}

const CleanupContext = struct { directory: [:0]const u16, profile: [:0]const u16 };

fn cleanup(record: *api.Record, context: CleanupContext, comptime effects: anytype) u32 {
    record.cleanup_error = 0;
    var result: u32 = 0;
    if (record.process != 0) {
        // Assignment failure leaves a suspended root outside the Job.
        if (effects.root(handle(record.process), 1) == .FALSE and effects.wait(handle(record.process), 0) != 0) result = lastError();
    }
    const terminated = effects.terminate(record, @sizeOf(api.Record));
    if (result == 0) result = terminated;
    if (record.process != 0) {
        const wait = effects.wait(handle(record.process), api.cleanup_wait_ms);
        if (result == 0 and wait == api.wait_timeout) result = api.wait_timeout;
        if (result == 0 and wait != 0) result = lastError();
    }
    const tree_waited = effects.tree(record);
    if (result == 0) result = tree_waited;
    if (result != 0) return result;
    const process_closed = effects.close(&record.process);
    if (result == 0) result = process_closed;
    const job_closed = effects.close(&record.job);
    if (result == 0) result = job_closed;
    const revoked = effects.revoke(context.profile.ptr, record);
    if (revoked != 0) return revoked;
    const restored = effects.restore(context.directory.ptr, record);
    if (result == 0) result = restored;
    if (record.profile_created != 0) {
        const deleted = effects.delete(context.profile.ptr);
        if (deleted >= 0) record.profile_created = 0;
        if (result == 0 and deleted < 0) result = @bitCast(deleted);
    }
    if (result == 0 and record.cleanup_error != 0) result = @intCast(record.cleanup_error);
    if (result == 0) record.cleanup = 1;
    return result;
}
test "native packet validation rejects missing strings and environment terminators" {
    try std.testing.expectEqual(@as(u32, 0), sprawling_native_packet_valid(&.{}, 0));
    const valid = [_]u16{ 'p', 0, 'c', 0, 'd', 0, 'n', 0, 'o', 0, 'e', 0, 0, 0, 0 };
    try std.testing.expectEqual(@as(u32, 1), sprawling_native_packet_valid(&valid, valid.len));
    for (0..valid.len) |len| try std.testing.expectEqual(@as(u32, 0), sprawling_native_packet_valid(&valid, len));
}
test "native packet parser fuzzes both rejection and bounded spans" {
    var rng = std.Random.DefaultPrng.init(0x44ee7a98);
    var units: [256]u16 = undefined;
    for (0..20_000) |_| {
        const len = rng.random().uintLessThan(usize, units.len);
        for (units[0..len]) |*unit| unit.* = rng.random().int(u16);
        if (packet(units[0..len])) |found| {
            for (found.strings) |part| try std.testing.expect(part.len <= len);
            try std.testing.expect(found.environment.len <= len);
        }
    }
}

test "resume readiness fuzz matches the Rust reference contract" {
    var rng = std.Random.DefaultPrng.init(0xbaca4821);
    var record: api.Record = std.mem.zeroes(api.Record);
    for (0..20_000) |_| {
        record.run_assigned = @intFromBool(rng.random().boolean());
        record.command_assigned = @intFromBool(rng.random().boolean());
        record.identity_verified = @intFromBool(rng.random().boolean());
        const expected = record.run_assigned == 1 and record.command_assigned == 1 and record.identity_verified == 1;
        try std.testing.expectEqual(expected, sprawling_native_may_resume(&record, @sizeOf(api.Record)) != 0);
    }
}

test "short native launch record is rejected before any write" {
    var record = std.mem.zeroes(api.Record);
    @memset(&record.failure_phase, 0x7f);
    const before = record.failure_phase;
    try std.testing.expectEqual(api.invalid_parameter, sprawling_native_launch(&.{}, 0, &record, 0));
    try std.testing.expectEqualSlices(u8, &before, &record.failure_phase);
}

test "native cleanup retains every owner through repeated timeout or refusal" {
    const Refusal = enum { root_wait, tree_wait, termination, none };
    const Double = struct {
        fn replies(comptime refusal: Refusal) type {
            return struct {
                fn root(_: api.HANDLE, _: u32) api.BOOL {
                    return .TRUE;
                }
                fn wait(_: api.HANDLE, _: u32) u32 {
                    return if (refusal == .root_wait) api.wait_timeout else 0;
                }
                fn terminate(_: *api.Record, _: usize) u32 {
                    return if (refusal == .termination) 5 else 0;
                }
                fn tree(_: *api.Record) u32 {
                    return if (refusal == .tree_wait) api.wait_timeout else 0;
                }
                fn close(raw: *usize) u32 {
                    raw.* = 0;
                    return 0;
                }
                fn revoke(_: [*:0]const u16, _: *api.Record) u32 {
                    return 0;
                }
                fn restore(_: [*:0]const u16, record: *api.Record) u32 {
                    record.root_security = 0;
                    return 0;
                }
                fn delete(_: [*:0]const u16) i32 {
                    return 0;
                }
            };
        }
    };
    const context: CleanupContext = .{ .directory = &.{'d'}, .profile = &.{'p'} };
    inline for (.{ Refusal.root_wait, Refusal.tree_wait, Refusal.termination }) |refusal| {
        var record = std.mem.zeroes(api.Record);
        record.process = 1;
        record.job = 2;
        record.root_security = 3;
        record.profile_created = 1;
        const expected: u32 = if (refusal == .termination) 5 else api.wait_timeout;
        for (0..2) |_| {
            try std.testing.expectEqual(expected, cleanup(&record, context, Double.replies(refusal)));
            try std.testing.expectEqual(@as(usize, 1), record.process);
            try std.testing.expectEqual(@as(usize, 2), record.job);
            try std.testing.expectEqual(@as(usize, 3), record.root_security);
            try std.testing.expectEqual(@as(usize, 1), record.profile_created);
            try std.testing.expectEqual(@as(usize, 0), record.cleanup);
        }
        try std.testing.expectEqual(@as(u32, 0), cleanup(&record, context, Double.replies(.none)));
        try std.testing.expectEqual(@as(usize, 1), record.cleanup);
        try std.testing.expectEqual(@as(usize, 0), record.process);
        try std.testing.expectEqual(@as(usize, 0), record.job);
        try std.testing.expectEqual(@as(usize, 0), record.root_security);
        try std.testing.expectEqual(@as(usize, 0), record.profile_created);
    }
}
