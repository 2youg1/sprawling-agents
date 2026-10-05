// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// Win32 declarations used only by the native confinement leaf.
const w = @import("std").os.windows;
pub const HANDLE = w.HANDLE;
pub const BOOL = w.BOOL;
pub const Record = extern struct {
    memory: usize,
    cpu: usize,
    job: usize,
    process: usize,
    pid: usize,
    finished: usize,
    exit: usize,
    cleanup: usize,
    cleanup_error: usize,
    parent_job: usize,
    run_assigned: usize,
    command_assigned: usize,
    identity_verified: usize,
};
pub const Security = extern struct { length: u32 = @sizeOf(Security), descriptor: ?*anyopaque = null, inherit: BOOL = BOOL.TRUE };
pub const Capabilities = extern struct { sid: *anyopaque, capabilities: ?*anyopaque = null, count: u32 = 0, reserved: u32 = 0 };
pub const Startup = extern struct {
    size: u32 = @sizeOf(Startup),
    reserved: ?[*:0]u16 = null,
    desktop: ?[*:0]u16 = null,
    title: ?[*:0]u16 = null,
    x: u32 = 0,
    y: u32 = 0,
    width: u32 = 0,
    height: u32 = 0,
    chars_x: u32 = 0,
    chars_y: u32 = 0,
    fill: u32 = 0,
    flags: u32 = 0x100,
    show: u16 = 0,
    reserved_size: u16 = 0,
    reserved_bytes: ?[*]u8 = null,
    stdin: ?HANDLE = null,
    stdout: ?HANDLE = null,
    stderr: ?HANDLE = null,
    attributes: ?*anyopaque = null,
};
pub const Process = extern struct { process: HANDLE, thread: HANDLE, pid: u32, tid: u32 };
pub const Trustee = extern struct { multiple: ?*anyopaque = null, operation: u32 = 0, form: u32 = 0, kind: u32 = 0, name: *anyopaque };
pub const Entry = extern struct { permissions: u32 = 0x001301bf, mode: u32 = 1, inheritance: u32 = 3, trustee: Trustee };

pub extern "userenv" fn CreateAppContainerProfile(name: [*:0]const u16, display: [*:0]const u16, description: [*:0]const u16, capabilities: ?*anyopaque, count: u32, sid: *?*anyopaque) callconv(.winapi) i32;
pub extern "userenv" fn DeleteAppContainerProfile(name: [*:0]const u16) callconv(.winapi) i32;
pub extern "advapi32" fn OpenProcessToken(process: HANDLE, access: u32, token: *HANDLE) callconv(.winapi) BOOL;
pub extern "advapi32" fn GetTokenInformation(token: HANDLE, class: u32, information: *anyopaque, size: u32, returned: *u32) callconv(.winapi) BOOL;
pub extern "advapi32" fn EqualSid(left: *anyopaque, right: *anyopaque) callconv(.winapi) BOOL;
pub extern "kernel32" fn OpenProcess(access: u32, inherit: BOOL, pid: u32) callconv(.winapi) ?HANDLE;
pub extern "kernel32" fn QueryInformationJobObject(job: HANDLE, class: u32, information: *anyopaque, size: u32, returned: ?*u32) callconv(.winapi) BOOL;
pub extern "kernel32" fn IsProcessInJob(process: HANDLE, job: HANDLE, member: *BOOL) callconv(.winapi) BOOL;
pub extern "advapi32" fn FreeSid(sid: *anyopaque) callconv(.winapi) ?*anyopaque;
pub extern "advapi32" fn GetNamedSecurityInfoW(name: [*:0]const u16, kind: u32, info: u32, owner: ?*?*anyopaque, group: ?*?*anyopaque, dacl: *?*anyopaque, sacl: ?*?*anyopaque, descriptor: *?*anyopaque) callconv(.winapi) u32;
pub extern "advapi32" fn ConvertStringSecurityDescriptorToSecurityDescriptorW(text: [*:0]const u16, revision: u32, descriptor: *?*anyopaque, size: ?*u32) callconv(.winapi) BOOL;
pub extern "advapi32" fn GetSecurityDescriptorSacl(descriptor: *anyopaque, present: *BOOL, acl: *?*anyopaque, defaulted: *BOOL) callconv(.winapi) BOOL;
pub extern "advapi32" fn SetEntriesInAclW(count: u32, entries: *const Entry, old: ?*anyopaque, result: *?*anyopaque) callconv(.winapi) u32;
pub extern "advapi32" fn SetNamedSecurityInfoW(name: [*:0]const u16, kind: u32, info: u32, owner: ?*anyopaque, group: ?*anyopaque, dacl: ?*anyopaque, sacl: ?*anyopaque) callconv(.winapi) u32;
pub extern "kernel32" fn CloseHandle(handle: HANDLE) callconv(.winapi) BOOL;
pub extern "kernel32" fn LocalFree(memory: ?*anyopaque) callconv(.winapi) ?*anyopaque;
pub extern "kernel32" fn CreateJobObjectW(security: ?*const Security, name: ?[*:0]const u16) callconv(.winapi) ?HANDLE;
pub extern "kernel32" fn AssignProcessToJobObject(job: HANDLE, process: HANDLE) callconv(.winapi) BOOL;
pub extern "kernel32" fn TerminateJobObject(job: HANDLE, code: u32) callconv(.winapi) BOOL;
pub extern "kernel32" fn TerminateProcess(process: HANDLE, code: u32) callconv(.winapi) BOOL;
pub extern "kernel32" fn WaitForSingleObject(handle: HANDLE, milliseconds: u32) callconv(.winapi) u32;
pub extern "kernel32" fn GetExitCodeProcess(process: HANDLE, code: *u32) callconv(.winapi) BOOL;
pub extern "kernel32" fn InitializeProcThreadAttributeList(list: ?*anyopaque, count: u32, flags: u32, size: *usize) callconv(.winapi) BOOL;
pub extern "kernel32" fn UpdateProcThreadAttribute(list: *anyopaque, flags: u32, attribute: usize, value: *anyopaque, size: usize, previous: ?*anyopaque, returned: ?*usize) callconv(.winapi) BOOL;
pub extern "kernel32" fn DeleteProcThreadAttributeList(list: *anyopaque) callconv(.winapi) void;
pub extern "kernel32" fn CreateProcessW(program: [*:0]const u16, command: [*:0]u16, process_security: ?*anyopaque, thread_security: ?*anyopaque, inherit: BOOL, flags: u32, environment: *anyopaque, directory: [*:0]const u16, startup: *Startup, result: *Process) callconv(.winapi) BOOL;
pub extern "kernel32" fn ResumeThread(thread: HANDLE) callconv(.winapi) u32;
pub extern "kernel32" fn CreateFileW(name: [*:0]const u16, access: u32, share: u32, security: ?*const Security, creation: u32, flags: u32, template: ?HANDLE) callconv(.winapi) HANDLE;

pub const security_capabilities: usize = 0x00020009;
pub const handle_list: usize = 0x00020002;
pub const suspended: u32 = 0x4;
pub const unicode_environment: u32 = 0x400;
pub const extended_startup: u32 = 0x80000;
pub const no_window: u32 = 0x08000000;
pub const below_normal: u32 = 0x4000;
pub const invalid_parameter: u32 = 87;
pub const invalid_handle: u32 = 6;
pub const file_write: u32 = 0x40000000;
pub const file_read: u32 = 0x80000000;
pub const file_share_read: u32 = 1;
pub const file_share_write: u32 = 2;
pub const create_always: u32 = 2;
pub const open_existing: u32 = 3;
pub const file_normal: u32 = 0x80;
pub const object_file: u32 = 1;
pub const dacl_information: u32 = 4;
pub const kill_on_close: u32 = 0x2000;
pub const hard_cap: u32 = 4;
pub const still_active: u32 = 259;
pub const wait_timeout: u32 = 258;
pub const nul: [*:0]const u16 = &[_:0]u16{ 'N', 'U', 'L' };

pub const token_query: u32 = 8;
pub const token_is_appcontainer: u32 = 29;
pub const token_appcontainer_sid: u32 = 31;

pub const label_information: u32 = 0x10;

pub const process_list: u32 = 3;
pub const synchronize: u32 = 0x100000;
pub const more_data: u32 = 234;
pub const cleanup_wait_ms: u32 = 5_000;
