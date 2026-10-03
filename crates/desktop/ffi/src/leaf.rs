// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The leaf's exports as Rust sees them: every function `zig/leaf.zig`
//! exports, declared once.
//!
//! Each returns a step number, which `ended` reads rather than trusts,
//! except the two DPI calls, which return the HRESULT the operating
//! system gave. Every pointer is lent for the call alone: the leaf keeps
//! none of them, frees none of Rust's memory, and writes nothing past
//! the length it was given beside the pointer.

use std::ffi::c_void;

use winsafe::co;

// The declarations themselves assert nothing a caller can rely on; each
// call below them states its own precondition where it is made.
#[expect(
    unsafe_code,
    reason = "the one declaration of the leaf's exports; every call into it is an `unsafe` block of its own"
)]
unsafe extern "C" {
    pub(crate) fn sprawling_desktop_windows(
        into: *mut winsafe::HWND,
        capacity: usize,
        found: *mut usize,
        code: *mut co::ERROR,
    ) -> u32;

    pub(crate) fn sprawling_desktop_capture(
        window: *mut c_void,
        width: i32,
        height: i32,
        into: *mut u8,
        len: usize,
        code: *mut co::ERROR,
    ) -> u32;

    pub(crate) fn sprawling_desktop_clipboard_read(
        into: *mut u16,
        capacity: usize,
        found: *mut usize,
        code: *mut co::ERROR,
    ) -> u32;

    pub(crate) fn sprawling_desktop_clipboard_write(
        text: *const u16,
        len: usize,
        code: *mut co::ERROR,
    ) -> u32;

    pub(crate) fn sprawling_desktop_dpi_declare(awareness: u32) -> co::HRESULT;

    pub(crate) fn sprawling_desktop_dpi_awareness(awareness: *mut u32) -> co::HRESULT;

    pub(crate) fn sprawling_desktop_keep(
        stream: *const usize,
        len: usize,
        into: *mut usize,
        capacity: usize,
        found: *mut usize,
    ) -> u32;

    pub(crate) fn sprawling_desktop_text_copy(
        block: *const u16,
        len: usize,
        into: *mut u16,
        capacity: usize,
        found: *mut usize,
    ) -> u32;

    pub(crate) fn sprawling_desktop_text_fill(
        units: *const u16,
        len: usize,
        block: *mut u16,
        block_len: usize,
    ) -> u32;

    pub(crate) fn sprawling_desktop_bitmap_bytes(width: i32, height: i32, bytes: *mut usize)
    -> u32;

    pub(crate) fn sprawling_desktop_cpu_sets(
        into: *mut u8,
        capacity: usize,
        found: *mut usize,
        code: *mut co::ERROR,
    ) -> u32;

    pub(crate) fn sprawling_desktop_thread_group(
        group: *mut u16,
        mask: *mut u64,
        code: *mut co::ERROR,
    ) -> u32;

    pub(crate) fn sprawling_desktop_full_speed(code: *mut co::ERROR) -> u32;

    pub(crate) fn sprawling_desktop_job_share(
        job: *mut c_void,
        weight: u32,
        memory: usize,
        code: *mut co::ERROR,
    ) -> u32;
}
