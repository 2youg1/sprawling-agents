// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

#![expect(
    clippy::unwrap_used,
    reason = "test scaffolding: a window that cannot be opened or closed fails the test that asked for it"
)]

use std::sync::mpsc;
use std::thread::JoinHandle;

use windows::Win32::Foundation::{HWND, LPARAM, WPARAM};
use windows::Win32::System::Threading::GetCurrentThreadId;
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DestroyWindow, DispatchMessageW, GetMessageW, MSG, PostThreadMessageW,
    SW_SHOWNOACTIVATE, ShowWindow, TranslateMessage, WINDOW_EX_STYLE, WINDOW_STYLE, WM_QUIT,
    WS_CHILD, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_POPUP, WS_VISIBLE,
};
use windows::core::{HSTRING, w};

pub const WIDTH: i32 = 160;

pub const HEIGHT: i32 = 90;

const ANSWERS_HIT_TESTS: WINDOW_STYLE = WINDOW_STYLE(0x0100);

pub struct Opened {
    address: usize,
    thread: u32,
    pumping: Option<JoinHandle<()>>,
}

impl Opened {
    pub fn at(title: &str, left: i32, top: i32, button: Option<&str>) -> Opened {
        let (sent, opened) = mpsc::channel();
        let title = title.to_owned();
        let button = button.map(str::to_owned);
        let pumping = std::thread::spawn(move || {
            let handle = window(&title, left, top, button.as_deref());
            sent.send((handle.0.expose_provenance(), this_thread()))
                .unwrap();
            pump();
            close(handle);
        });
        let (address, thread) = opened.recv().unwrap();
        Opened {
            address,
            thread,
            pumping: Some(pumping),
        }
    }

    pub fn address(&self) -> usize {
        self.address
    }

    pub fn handle(&self) -> winsafe::HWND {
        made_up(self.address)
    }
}

impl Drop for Opened {
    fn drop(&mut self) {
        quit(self.thread);
        if let Some(pumping) = self.pumping.take() {
            pumping.join().unwrap();
        }
    }
}

#[expect(
    unsafe_code,
    reason = "a handle is minted from an address only through the binding's unsafe constructor"
)]
pub fn made_up(address: usize) -> winsafe::HWND {
    unsafe { winsafe::HWND::from_ptr(std::ptr::without_provenance_mut(address)) }
}

#[expect(
    unsafe_code,
    reason = "a test opens its own window through the FFI entry point"
)]
fn window(title: &str, left: i32, top: i32, button: Option<&str>) -> HWND {
    let title = HSTRING::from(title);

    let handle = unsafe {
        CreateWindowExW(
            WS_EX_NOACTIVATE | WS_EX_TOOLWINDOW | WS_EX_TOPMOST,
            w!("STATIC"),
            &title,
            WS_POPUP | ANSWERS_HIT_TESTS,
            left,
            top,
            WIDTH,
            HEIGHT,
            None,
            None,
            None,
            None,
        )
    }
    .unwrap();
    if let Some(caption) = button {
        let caption = HSTRING::from(caption);

        unsafe {
            CreateWindowExW(
                WINDOW_EX_STYLE(0),
                w!("BUTTON"),
                &caption,
                WS_CHILD | WS_VISIBLE,
                8,
                8,
                96,
                32,
                Some(handle),
                None,
                None,
                None,
            )
        }
        .unwrap();
    }

    let _was_visible = unsafe { ShowWindow(handle, SW_SHOWNOACTIVATE) };
    handle
}

#[expect(
    unsafe_code,
    reason = "a message loop is the FFI entry points GetMessage and DispatchMessage"
)]
fn pump() {
    let mut message = MSG::default();

    while unsafe { GetMessageW(&raw mut message, None, 0, 0) }.0 > 0 {
        unsafe {
            let _translated = TranslateMessage(&raw const message);
            DispatchMessageW(&raw const message);
        }
    }
}

#[expect(unsafe_code, reason = "destroying a window is an FFI entry point")]
fn close(handle: HWND) {
    unsafe { DestroyWindow(handle) }.unwrap();
}

#[expect(
    unsafe_code,
    reason = "a thread's id is read through an FFI entry point"
)]
fn this_thread() -> u32 {
    unsafe { GetCurrentThreadId() }
}

#[expect(
    unsafe_code,
    reason = "posting a message to a thread is an FFI entry point"
)]
fn quit(thread: u32) {
    unsafe { PostThreadMessageW(thread, WM_QUIT, WPARAM(0), LPARAM(0)) }.unwrap();
}
