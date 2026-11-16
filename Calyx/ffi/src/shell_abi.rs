// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! C ABI into the Zig city shell (`Calyx/shell/host.zig`). Safe Rust hosts
//! implement [`CityHost`]; this module owns the only `unsafe` trampolines
//! (`Calyx/Spec.lean` D7).

use std::ffi::c_void;

#[repr(C)]
pub struct CityConfig {
    pub url_ptr: *const u8,
    pub url_len: usize,
    pub city_ptr: *const u8,
    pub city_len: usize,
    pub token_ptr: *const u8,
    pub token_len: usize,
    pub surface: u32,
}

#[repr(C)]
struct BufIo {
    in_ptr: *const u8,
    in_len: usize,
    out_ptr: *mut u8,
    out_cap: usize,
    out_written: *mut usize,
}

#[repr(C)]
struct HostAbi {
    ctx: *mut c_void,
    poll: unsafe extern "C" fn(*mut c_void, *mut u8, usize, *mut usize) -> u32,
    tell: unsafe extern "C" fn(*mut c_void, u32, u32),
    carry_line: unsafe extern "C" fn(*mut c_void, *const u8, usize),
    open_paired: unsafe extern "C" fn(*mut c_void, *const u8, usize) -> i32,
    carry_remote: unsafe extern "C" fn(*mut c_void, *mut BufIo) -> i32,
    clock_ms: unsafe extern "C" fn(*mut c_void, *mut u64) -> i32,
    post_json: unsafe extern "C" fn(*mut c_void, *const u8, usize) -> i32,
    answer_json: unsafe extern "C" fn(*mut c_void, *mut BufIo) -> i32,
}

/// Host callbacks the Zig city face invokes on the UI thread.
pub trait CityHost {
    fn poll(&self, into: &mut [u8]) -> (u32, usize);
    fn tell(&self, event: u32, arg: u32);
    fn carry_line(&self, line: &str);
    fn open_paired(&self, url: &str) -> i32;
    fn carry_remote(&self, tail: &str, into: &mut [u8]) -> Result<usize, i32>;
    fn clock_ms(&self) -> Result<u64, i32>;
    fn post_json(&self, _json: &str) -> i32 {
        0
    }
    fn answer_json(&self, _json: &str, _into: &mut [u8]) -> Result<usize, i32> {
        Ok(0)
    }
}

struct Erased {
    host: *const c_void,
    poll: fn(*const c_void, &mut [u8]) -> (u32, usize),
    tell: fn(*const c_void, u32, u32),
    carry_line: fn(*const c_void, &str),
    open_paired: fn(*const c_void, &str) -> i32,
    carry_remote: fn(*const c_void, &str, &mut [u8]) -> Result<usize, i32>,
    clock_ms: fn(*const c_void) -> Result<u64, i32>,
    post_json: fn(*const c_void, &str) -> i32,
    answer_json: fn(*const c_void, &str, &mut [u8]) -> Result<usize, i32>,
}

#[expect(
    unsafe_code,
    reason = "Calyx Spec D7: declare Zig city UI export; linked by sprawling-calyx"
)]
unsafe extern "C" {
    fn calyx_city_ui_run(config: *const CityConfig, host: *const HostAbi) -> i32;
}

/// Runs the Zig city UI until it returns. `config` string pointers must stay
/// live for the call; `host` is only borrowed for the call.
pub fn run_city_ui<H: CityHost>(config: &CityConfig, host: &H) -> i32 {
    let erased = Erased {
        host: std::ptr::from_ref(host).cast(),
        poll: shim_poll::<H>,
        tell: shim_tell::<H>,
        carry_line: shim_carry_line::<H>,
        open_paired: shim_open_paired::<H>,
        carry_remote: shim_carry_remote::<H>,
        clock_ms: shim_clock_ms::<H>,
        post_json: shim_post_json::<H>,
        answer_json: shim_answer_json::<H>,
    };
    let abi = HostAbi {
        ctx: std::ptr::from_ref(&erased).cast::<c_void>().cast_mut(),
        poll: trampoline_poll,
        tell: trampoline_tell,
        carry_line: trampoline_carry_line,
        open_paired: trampoline_open_paired,
        carry_remote: trampoline_carry_remote,
        clock_ms: trampoline_clock_ms,
        post_json: trampoline_post_json,
        answer_json: trampoline_answer_json,
    };
    #[expect(
        unsafe_code,
        reason = "Calyx Spec D7: one call into Zig city UI; pointers live for the call"
    )]
    // SAFETY: config and abi are live for this call; Zig keeps none past return.
    // erased.host borrows `host` for the same duration.
    unsafe { calyx_city_ui_run(config, &abi) }
}

fn shim_poll<H: CityHost>(host: *const c_void, into: &mut [u8]) -> (u32, usize) {
    #[expect(unsafe_code, reason = "Calyx Spec D7: recover typed host")]
    // SAFETY: host is &H from run_city_ui for the UI call.
    unsafe { (*host.cast::<H>()).poll(into) }
}

fn shim_tell<H: CityHost>(host: *const c_void, event: u32, arg: u32) {
    #[expect(unsafe_code, reason = "Calyx Spec D7: recover typed host")]
    // SAFETY: host is &H from run_city_ui for the UI call.
    unsafe {
        (*host.cast::<H>()).tell(event, arg);
    }
}

fn shim_carry_line<H: CityHost>(host: *const c_void, line: &str) {
    #[expect(unsafe_code, reason = "Calyx Spec D7: recover typed host")]
    // SAFETY: host is &H from run_city_ui for the UI call.
    unsafe {
        (*host.cast::<H>()).carry_line(line);
    }
}

fn shim_open_paired<H: CityHost>(host: *const c_void, url: &str) -> i32 {
    #[expect(unsafe_code, reason = "Calyx Spec D7: recover typed host")]
    // SAFETY: host is &H from run_city_ui for the UI call.
    unsafe { (*host.cast::<H>()).open_paired(url) }
}

fn shim_carry_remote<H: CityHost>(
    host: *const c_void,
    tail: &str,
    into: &mut [u8],
) -> Result<usize, i32> {
    #[expect(unsafe_code, reason = "Calyx Spec D7: recover typed host")]
    // SAFETY: host is &H from run_city_ui for the UI call.
    unsafe { (*host.cast::<H>()).carry_remote(tail, into) }
}

fn shim_clock_ms<H: CityHost>(host: *const c_void) -> Result<u64, i32> {
    #[expect(unsafe_code, reason = "Calyx Spec D7: recover typed host")]
    // SAFETY: host is &H from run_city_ui for the UI call.
    unsafe { (*host.cast::<H>()).clock_ms() }
}

fn shim_post_json<H: CityHost>(host: *const c_void, json: &str) -> i32 {
    #[expect(unsafe_code, reason = "Calyx Spec D7: recover typed host")]
    // SAFETY: host is &H from run_city_ui for the UI call.
    unsafe { (*host.cast::<H>()).post_json(json) }
}

fn shim_answer_json<H: CityHost>(
    host: *const c_void,
    json: &str,
    into: &mut [u8],
) -> Result<usize, i32> {
    #[expect(unsafe_code, reason = "Calyx Spec D7: recover typed host")]
    // SAFETY: host is &H from run_city_ui for the UI call.
    unsafe { (*host.cast::<H>()).answer_json(json, into) }
}

fn erased<'a>(ctx: *mut c_void) -> &'a Erased {
    #[expect(unsafe_code, reason = "Calyx Spec D7: recover Erased from trampoline ctx")]
    // SAFETY: ctx is &Erased formed in run_city_ui for this call.
    unsafe { &*ctx.cast::<Erased>() }
}

#[expect(
    unsafe_code,
    reason = "Calyx Spec D7: C trampoline into CityHost::poll"
)]
unsafe extern "C" fn trampoline_poll(
    ctx: *mut c_void,
    into: *mut u8,
    capacity: usize,
    written: *mut usize,
) -> u32 {
    let erased = erased(ctx);
    #[expect(
        unsafe_code,
        reason = "Calyx Spec D7: slice from Zig-lent poll buffer"
    )]
    // SAFETY: Zig lends `into`/`written` for this call only.
    let (into, written) = unsafe {
        (
            std::slice::from_raw_parts_mut(into, capacity),
            &mut *written,
        )
    };
    let (kind, n) = (erased.poll)(erased.host, into);
    *written = n;
    kind
}

#[expect(
    unsafe_code,
    reason = "Calyx Spec D7: C trampoline into CityHost::tell"
)]
unsafe extern "C" fn trampoline_tell(ctx: *mut c_void, event: u32, arg: u32) {
    let erased = erased(ctx);
    (erased.tell)(erased.host, event, arg);
}

#[expect(
    unsafe_code,
    reason = "Calyx Spec D7: C trampoline into CityHost::carry_line"
)]
unsafe extern "C" fn trampoline_carry_line(
    ctx: *mut c_void,
    line_ptr: *const u8,
    line_len: usize,
) {
    let erased = erased(ctx);
    #[expect(unsafe_code, reason = "Calyx Spec D7: read Zig-lent line bytes")]
    // SAFETY: Zig lends the line for this call.
    let line = unsafe { std::slice::from_raw_parts(line_ptr, line_len) };
    let Ok(text) = std::str::from_utf8(line) else {
        return;
    };
    (erased.carry_line)(erased.host, text);
}

#[expect(
    unsafe_code,
    reason = "Calyx Spec D7: C trampoline into CityHost::open_paired"
)]
unsafe extern "C" fn trampoline_open_paired(
    ctx: *mut c_void,
    url_ptr: *const u8,
    url_len: usize,
) -> i32 {
    let erased = erased(ctx);
    #[expect(unsafe_code, reason = "Calyx Spec D7: read Zig-lent URL bytes")]
    // SAFETY: Zig lends the URL for this call.
    let url = unsafe { std::slice::from_raw_parts(url_ptr, url_len) };
    let Ok(text) = std::str::from_utf8(url) else {
        return 1;
    };
    (erased.open_paired)(erased.host, text)
}

#[expect(
    unsafe_code,
    reason = "Calyx Spec D7: C trampoline into CityHost::carry_remote"
)]
unsafe extern "C" fn trampoline_carry_remote(ctx: *mut c_void, io: *mut BufIo) -> i32 {
    buf_io_callback(ctx, io, |erased, text, into| {
        (erased.carry_remote)(erased.host, text, into)
    })
}

#[expect(
    unsafe_code,
    reason = "Calyx Spec D7: C trampoline into CityHost::clock_ms"
)]
unsafe extern "C" fn trampoline_clock_ms(ctx: *mut c_void, out: *mut u64) -> i32 {
    let erased = erased(ctx);
    #[expect(unsafe_code, reason = "Calyx Spec D7: write clock into Zig out pointer")]
    // SAFETY: out is live for this call.
    match (erased.clock_ms)(erased.host) {
        Ok(ms) => unsafe {
            *out = ms;
            0
        },
        Err(code) => code,
    }
}

#[expect(
    unsafe_code,
    reason = "Calyx Spec D7: C trampoline into CityHost::post_json"
)]
unsafe extern "C" fn trampoline_post_json(
    ctx: *mut c_void,
    json_ptr: *const u8,
    json_len: usize,
) -> i32 {
    let erased = erased(ctx);
    #[expect(unsafe_code, reason = "Calyx Spec D7: read Zig-lent JSON bytes")]
    // SAFETY: Zig lends JSON for this call.
    let json = unsafe { std::slice::from_raw_parts(json_ptr, json_len) };
    let Ok(text) = std::str::from_utf8(json) else {
        return 1;
    };
    (erased.post_json)(erased.host, text)
}

#[expect(
    unsafe_code,
    reason = "Calyx Spec D7: C trampoline into CityHost::answer_json"
)]
unsafe extern "C" fn trampoline_answer_json(ctx: *mut c_void, io: *mut BufIo) -> i32 {
    buf_io_callback(ctx, io, |erased, text, into| {
        (erased.answer_json)(erased.host, text, into)
    })
}

fn buf_io_callback(
    ctx: *mut c_void,
    io: *mut BufIo,
    call: fn(&Erased, &str, &mut [u8]) -> Result<usize, i32>,
) -> i32 {
    let erased = erased(ctx);
    #[expect(unsafe_code, reason = "Calyx Spec D7: Zig-lent BufIo buffers")]
    // SAFETY: Zig lends `io` and its slices for this call.
    let (text, into, written) = unsafe {
        let io = &*io;
        (
            std::slice::from_raw_parts(io.in_ptr, io.in_len),
            std::slice::from_raw_parts_mut(io.out_ptr, io.out_cap),
            &mut *io.out_written,
        )
    };
    let Ok(text) = std::str::from_utf8(text) else {
        return 1;
    };
    match call(erased, text, into) {
        Ok(n) => {
            *written = n;
            0
        }
        Err(code) => {
            *written = 0;
            code
        }
    }
}
