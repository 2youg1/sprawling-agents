// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// Per-profile grants; policy and declared roots come from the Rust packet.
const std = @import("std");
const api = @import("confinement_api.zig");
const allocator = std.heap.page_allocator;
extern "kernel32" fn CreateMutexExW(security: *const api.Security, name: [*:0]const u16, flags: u32, access: u32) callconv(.winapi) ?api.HANDLE;
extern "kernel32" fn ReleaseMutex(mutex: api.HANDLE) callconv(.winapi) api.BOOL;
extern "userenv" fn DeriveAppContainerSidFromAppContainerName(name: [*:0]const u16, sid: *?*anyopaque) callconv(.winapi) i32;
extern "advapi32" fn GetSecurityDescriptorControl(descriptor: *anyopaque, control: *u16, revision: *u32) callconv(.winapi) api.BOOL;
const mutex_access: u32 = api.synchronize | 1;
const mutex_name = std.unicode.utf8ToUtf16LeStringLiteral("Global\\sprawling.native.acl");

fn lastError() u32 {
    return @intFromEnum(std.os.windows.GetLastError());
}

fn update(path: [*:0]const u16, sid: *anyopaque, mode: u32, declared: bool) u32 {
    const sddl = std.unicode.utf8ToUtf16LeStringLiteral(std.fmt.comptimePrint("D:(A;;0x{x};;;AU)", .{mutex_access}));
    var descriptor: ?*anyopaque = null;
    if (api.ConvertStringSecurityDescriptorToSecurityDescriptorW(sddl, 1, &descriptor, null) == .FALSE) return lastError();
    const security: api.Security = .{ .descriptor = descriptor, .inherit = .FALSE };
    const opened = CreateMutexExW(&security, mutex_name, 0, mutex_access);
    var result: u32 = if (opened == null) lastError() else 0;
    if (api.LocalFree(descriptor) != null and result == 0) result = lastError();
    const mutex = opened orelse return result;
    if (result != 0) {
        if (api.CloseHandle(mutex) == .FALSE) return lastError();
        return result;
    }
    const waited = api.WaitForSingleObject(mutex, api.cleanup_wait_ms);
    if (waited == 0 or waited == 0x80) {
        result = change(path, sid, mode, declared);
        if (ReleaseMutex(mutex) == .FALSE and result == 0) result = lastError();
    } else result = if (waited == api.wait_timeout) waited else lastError();
    if (api.CloseHandle(mutex) == .FALSE and result == 0) result = lastError();
    return result;
}

fn change(path: [*:0]const u16, sid: *anyopaque, mode: u32, declared: bool) u32 {
    var old: ?*anyopaque = null;
    var descriptor: ?*anyopaque = null;
    var result = api.GetNamedSecurityInfoW(path, api.object_file, api.dacl_information, null, null, &old, null, &descriptor);
    if (result != 0) return if (mode == 4 and (result == 2 or result == 3)) 0 else result;
    var control: u16 = 0;
    var revision: u32 = 0;
    if (GetSecurityDescriptorControl(descriptor orelse return api.invalid_parameter, &control, &revision) == .FALSE) result = lastError();
    if (result != 0 or (!declared and control & 0x1000 == 0)) {
        if (api.LocalFree(descriptor) != null and result == 0) result = lastError();
        return result;
    }
    const entry: api.Entry = .{ .permissions = 0x001200a9, .mode = mode, .trustee = .{ .name = sid } };
    var changed: ?*anyopaque = null;
    result = api.SetEntriesInAclW(1, &entry, old, &changed);
    if (result == 0) result = api.SetNamedSecurityInfoW(path, api.object_file, api.dacl_information, null, null, changed, null);
    if (api.LocalFree(changed) != null and result == 0) result = lastError();
    if (api.LocalFree(descriptor) != null and result == 0) result = lastError();
    return result;
}

pub fn grant(roots: []const u16, sid: *anyopaque, record: *api.Record) u32 {
    if (roots.len == 0) return 0;
    const owned = allocator.dupeZ(u16, roots) catch return 8;
    record.grant_paths = @intFromPtr(owned.ptr);
    record.grant_units = owned.len;
    return each(owned, sid, 1, record.grant_roots);
}

fn each(roots: []const u16, sid: *anyopaque, mode: u32, declared: usize) u32 {
    var index: usize = 0;
    var paths = std.mem.splitScalar(u16, roots, 10);
    while (paths.next()) |path| {
        if (path.len == 0) return api.invalid_parameter;
        const terminated = allocator.dupeZ(u16, path) catch return 8;
        defer allocator.free(terminated);
        const result = update(terminated.ptr, sid, mode, index < declared);
        index = std.math.add(usize, index, 1) catch return api.invalid_parameter;
        if (result != 0) return result;
    }
    return 0;
}

pub fn revoke(profile: [*:0]const u16, record: *api.Record) u32 {
    if (record.grant_paths == 0) return 0;
    var sid: ?*anyopaque = null;
    const derived = DeriveAppContainerSidFromAppContainerName(profile, &sid);
    if (derived < 0) return @bitCast(derived);
    const named = sid orelse return api.invalid_parameter;
    const roots: [*:0]u16 = @ptrFromInt(record.grant_paths);
    var result = each(roots[0..record.grant_units], named, 4, record.grant_roots);
    if (api.FreeSid(named) != null and result == 0) result = lastError();
    if (result != 0) return result;
    allocator.free(roots[0..record.grant_units :0]);
    record.grant_paths = 0;
    record.grant_units = 0;
    return 0;
}
