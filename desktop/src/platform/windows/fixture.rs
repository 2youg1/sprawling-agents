// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! A window a contract test opens in its own process, and closes again.
//!
//! The contracts of desktop-SPEC.md section 8-11 are read back from a
//! real window, and the only window a test may read is one it made: a
//! test that listed, hit-tested or walked the person's own windows would
//! depend on what happens to be open, and could change it. This window
//! takes no focus and shows no taskbar button.
//!
//! It lives on a thread of its own that answers its messages. UI
//! Automation reads a control by sending messages to the thread that
//! owns it, and a test thread that owned the window would be waiting on
//! itself.

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

use super::geometry::Bounds;

/// How wide the window is, in pixels.
const WIDTH: i32 = 160;

/// How tall the window is, in pixels.
const HEIGHT: i32 = 90;

/// `SS_NOTIFY`, which the binding does not name: a static window with
/// it answers a hit test as its own client area rather than as
/// transparent, so a point on it is a point on it.
const ANSWERS_HIT_TESTS: WINDOW_STYLE = WINDOW_STYLE(0x0100);

/// One open window, closed when this is dropped.
pub(crate) struct Opened {
    address: usize,
    bounds: Bounds,
    thread: u32,
    pumping: Option<JoinHandle<()>>,
}

impl Opened {
    /// Opens a window titled `title` whose top-left corner is at
    /// `(left, top)` on the screen, holding one button captioned
    /// `button` when one is given.
    pub(crate) fn at(title: &str, left: i32, top: i32, button: Option<&str>) -> Opened {
        let bounds = Bounds::from_corners(
            left,
            top,
            left.checked_add(WIDTH).unwrap(),
            top.checked_add(HEIGHT).unwrap(),
        )
        .unwrap();
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
            bounds,
            thread,
            pumping: Some(pumping),
        }
    }

    /// The window's handle, as the address it is.
    pub(crate) fn address(&self) -> usize {
        self.address
    }

    /// Where the window is on the screen.
    pub(crate) fn bounds(&self) -> Bounds {
        self.bounds
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

/// The window, and the button inside it.
#[expect(
    unsafe_code,
    reason = "a test opens its own window through the FFI entry point"
)]
fn window(title: &str, left: i32, top: i32, button: Option<&str>) -> HWND {
    let title = HSTRING::from(title);
    // SAFETY: `STATIC` is a class every process has registered, `title`
    // is a terminated string that lives across the call, and no parent,
    // menu, instance or creation data is lent to it.
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
        // SAFETY: `BUTTON` is a class every process has registered,
        // `caption` lives across the call, and the parent is the window
        // created above on this thread, which lives until `close`.
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
    // SAFETY: `handle` is the window created above on this thread. The
    // answer is whether it was visible before, which is not a failure.
    let _was_visible = unsafe { ShowWindow(handle, SW_SHOWNOACTIVATE) };
    handle
}

/// Answers the window's messages until `quit` asks this thread to stop.
#[expect(
    unsafe_code,
    reason = "a message loop is the FFI entry points GetMessage and DispatchMessage"
)]
fn pump() {
    let mut message = MSG::default();
    // SAFETY: `message` is a live local that `GetMessageW` writes one
    // message into; it answers zero for `WM_QUIT` and minus one for a
    // failure, and both end the loop.
    while unsafe { GetMessageW(&raw mut message, None, 0, 0) }.0 > 0 {
        // SAFETY: `message` is the message `GetMessageW` just wrote, and
        // both calls only read it.
        unsafe {
            let _translated = TranslateMessage(&raw const message);
            DispatchMessageW(&raw const message);
        }
    }
}

/// Closes the window on the thread that opened it.
#[expect(unsafe_code, reason = "destroying a window is an FFI entry point")]
fn close(handle: HWND) {
    // SAFETY: `handle` is the window this thread created and has not
    // destroyed.
    unsafe { DestroyWindow(handle) }.unwrap();
}

/// The thread this is called on.
#[expect(
    unsafe_code,
    reason = "a thread's id is read through an FFI entry point"
)]
fn this_thread() -> u32 {
    // SAFETY: the call takes nothing and reads the calling thread's id.
    unsafe { GetCurrentThreadId() }
}

/// Asks the window's thread to stop answering.
#[expect(
    unsafe_code,
    reason = "posting a message to a thread is an FFI entry point"
)]
fn quit(thread: u32) {
    // SAFETY: `thread` is the id of the thread that opened the window,
    // which made a message queue when it did, and the message carries
    // no pointer.
    unsafe { PostThreadMessageW(thread, WM_QUIT, WPARAM(0), LPARAM(0)) }.unwrap();
}
