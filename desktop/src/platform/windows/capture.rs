// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! `PrintWindow`: one window as pixels, and the all-black answer that is
//! a failure rather than an image.
//!
//! **The unit of capture is the same as the unit of permission.**
//! `PrintWindow` asks one window for its own pixels, so a scope file
//! that lists windows is enforced by the mechanism rather than beside
//! it. Desktop Duplication would copy an entire output, and building a
//! per-window permission on top of a whole-screen mechanism is how the
//! door §8.5 of the SPEC closed gets opened from behind
//! (desktop-SPEC.md §8.6, first pair).
//!
//! `PW_RENDERFULLCONTENT` is what makes a hardware-composited window —
//! most browsers, most editors — render into the bitmap instead of
//! arriving blank. It does not always work, and when it does not the
//! result is a rectangle of pure black. That is checked for and refused:
//! a model shown a black picture of a window will describe the black.

use windows::Win32::Foundation::HWND;
use windows::Win32::Graphics::Gdi::{
    BI_RGB, BITMAPINFO, BITMAPINFOHEADER, CreateCompatibleBitmap, CreateCompatibleDC,
    DIB_RGB_COLORS, DeleteDC, DeleteObject, GetDC, GetDIBits, HGDIOBJ, ReleaseDC, SelectObject,
};
use windows::Win32::Storage::Xps::{PRINT_WINDOW_FLAGS, PrintWindow};
use windows::Win32::UI::WindowsAndMessaging::PW_RENDERFULLCONTENT;

use super::fault;
use super::geometry::Bounds;
use crate::refusal::{Refusal, RefusalCode};

/// One window's pixels, in the order `image` reads them.
///
/// # Errors
/// Refuses a window GDI will not draw into a bitmap, a size this machine
/// will not allocate, and a capture that came back entirely black.
pub(crate) fn window(handle: HWND, bounds: Bounds) -> Result<image::RgbaImage, Refusal> {
    // `GetDC` of no window is the context of the whole screen, which is
    // exactly what this module exists not to read.
    if handle.is_invalid() {
        return Err(Refusal::new(
            RefusalCode::ToolUnavailable,
            "capture a window",
            "no window was named".to_owned(),
            "call `desktop.windows` again and name a window from it",
        ));
    }
    let (width, height) = (bounds.width(), bounds.height());
    let (Ok(pixel_width), Ok(pixel_height)) = (i32::try_from(width), i32::try_from(height)) else {
        return Err(too_large(width, height));
    };
    let bgra = Surface::over(handle, pixel_width, pixel_height)?.drawn()?;
    let mut pixels = image::RgbaImage::new(width, height);
    let mut lit = false;
    for (at, pixel) in pixels.pixels_mut().enumerate() {
        let start = at.saturating_mul(4);
        let Some([blue, green, red, _alpha]) = bgra.get(start..start.saturating_add(4)) else {
            return Err(too_large(width, height));
        };
        // GDI gives BGRA with an alpha channel that means nothing here:
        // `PrintWindow` leaves it zero on most windows, and honouring it
        // would make every capture fully transparent.
        lit = lit || *blue != 0 || *green != 0 || *red != 0;
        *pixel = image::Rgba([*red, *green, *blue, u8::MAX]);
    }
    if !lit {
        return Err(Refusal::new(
            RefusalCode::ToolUnavailable,
            "capture a window",
            "the window drew nothing: every pixel came back black".to_owned(),
            "this window composes itself in a way GDI cannot read — bring it to the front and \
             try again, or read it with `desktop.snapshot`, which does not go through pixels",
        ));
    }
    Ok(pixels)
}

/// A window too big to measure in the units GDI counts in.
fn too_large(width: u32, height: u32) -> Refusal {
    Refusal::new(
        RefusalCode::InvalidArgs,
        "capture a window",
        format!("a {width}x{height} window is more than this machine will draw at once"),
        "ask for a `region` of the window rather than the whole of it",
    )
}

/// The three GDI objects a capture needs, released together.
///
/// They exist as one value because they are only ever created together
/// and only ever destroyed together, and because `Drop` is the only
/// thing that releases them on the error paths between the two — a
/// capture that refuses halfway through must not leave a device context
/// behind, since GDI hands out a bounded number of them per process.
struct Surface {
    /// The window `screen` was taken for, which is the window it is
    /// given back with.
    window: HWND,
    screen: windows::Win32::Graphics::Gdi::HDC,
    memory: windows::Win32::Graphics::Gdi::HDC,
    bitmap: windows::Win32::Graphics::Gdi::HBITMAP,
    width: i32,
    height: i32,
}

impl Surface {
    /// # Errors
    /// Refuses when GDI will not give this process another device
    /// context or another bitmap of this size.
    #[expect(
        unsafe_code,
        reason = "GDI hands out drawing contexts and bitmaps only through the FFI entry points; each precondition is stated at its block"
    )]
    fn over(handle: HWND, width: i32, height: i32) -> Result<Surface, Refusal> {
        // SAFETY: `handle` is an HWND this connection resolved in this
        // same call. `GetDC` answers with a null context for a window
        // that has gone away rather than misbehaving, and that null is
        // what the check below reads.
        let screen = unsafe { GetDC(Some(handle)) };
        if screen.is_invalid() {
            return Err(fault::last(
                "borrow the window's drawing context",
                "call `desktop.windows` again; the window may have closed since it was named",
            ));
        }
        // SAFETY: `screen` is the non-null context obtained immediately
        // above and is still owned here; both calls only read its
        // format to make something compatible with it.
        let (memory, bitmap) = unsafe {
            (
                CreateCompatibleDC(Some(screen)),
                CreateCompatibleBitmap(screen, width, height),
            )
        };
        let surface = Surface {
            window: handle,
            screen,
            memory,
            bitmap,
            width,
            height,
        };
        if surface.memory.is_invalid() || surface.bitmap.is_invalid() {
            // `surface` is returned by value into the error path's drop,
            // so whichever of the three did succeed is still released.
            return Err(fault::last(
                "make a bitmap the size of this window",
                "ask for a `region` of the window, or close something: this machine is out of \
                 drawing resources",
            ));
        }
        Ok(surface)
    }

    /// Asks the window to draw itself, and reads the result back as
    /// top-down BGRA once the bitmap is no longer selected.
    #[expect(
        unsafe_code,
        reason = "PrintWindow is the one call that asks a single window for its own pixels, which is what makes the unit of capture the unit of permission"
    )]
    fn drawn(&self) -> Result<Vec<u8>, Refusal> {
        let drew = {
            let _selected = Selected::into_memory(self)?;
            // SAFETY: `self.window` names the window being captured, and
            // `self.memory` is the live context the bitmap is selected
            // into while `_selected` lives. `PrintWindow` writes only
            // into that bitmap, whose size was fixed from this window's
            // own measurement.
            let drew = unsafe {
                PrintWindow(
                    self.window,
                    self.memory,
                    PRINT_WINDOW_FLAGS(PW_RENDERFULLCONTENT),
                )
            }
            .as_bool();
            // The reason is read before the guard selects the old object
            // back, which would overwrite it.
            drew.then_some(()).ok_or_else(|| {
                fault::last(
                    "ask the window to draw itself",
                    "bring the window to the front and try again; a minimised window has \
                     nothing to draw",
                )
            })
        };
        drew?;
        self.bits()
    }

    /// The bitmap's pixels, top row first.
    #[expect(
        unsafe_code,
        reason = "GetDIBits is the only way to read a GDI bitmap back into memory this process owns"
    )]
    fn bits(&self) -> Result<Vec<u8>, Refusal> {
        let unmeasurable = || too_large(self.width.unsigned_abs(), self.height.unsigned_abs());
        let header_size =
            u32::try_from(std::mem::size_of::<BITMAPINFOHEADER>()).map_err(|_| unmeasurable())?;
        let mut info = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: header_size,
                biWidth: self.width,
                // Negative height is how GDI is told to hand back a
                // top-down image. Without it every capture is upside
                // down, which is the kind of defect that survives review
                // because nothing in the code says which way is up.
                biHeight: self.height.checked_neg().ok_or_else(unmeasurable)?,
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB.0,
                ..BITMAPINFOHEADER::default()
            },
            ..BITMAPINFO::default()
        };
        let lines = u32::try_from(self.height).map_err(|_| unmeasurable())?;
        let count = usize::try_from(self.width)
            .ok()
            .and_then(|width| width.checked_mul(usize::try_from(self.height).ok()?))
            .and_then(|pixels| pixels.checked_mul(4))
            .ok_or_else(unmeasurable)?;
        let mut bits: Vec<u8> = vec![0; count];
        // SAFETY: `self.memory` and `self.bitmap` are the live objects
        // this value owns, and the bitmap is selected into no context at
        // this moment: `Selected` put the old object back when its block
        // in `drawn` ended. `bits` holds exactly the number of bytes the
        // header above describes — width times height times four, for
        // the 32 bits per pixel it declares — so the buffer Win32 fills
        // is the buffer that exists. `info` is a live local of the type
        // the parameter names.
        let rows = unsafe {
            GetDIBits(
                self.memory,
                self.bitmap,
                0,
                lines,
                Some(bits.as_mut_ptr().cast::<std::ffi::c_void>()),
                std::ptr::from_mut(&mut info),
                DIB_RGB_COLORS,
            )
        };
        if rows == 0 {
            return Err(fault::last(
                "read the captured pixels back",
                "try again; if it persists, ask for a smaller `region`",
            ));
        }
        if u32::try_from(rows).ok() != Some(lines) {
            return Err(Refusal::new(
                RefusalCode::ToolUnavailable,
                "capture a window",
                format!("GDI read back {rows} of {lines} rows"),
                "try again; a picture with rows missing is not handed over",
            ));
        }
        Ok(bits)
    }
}

/// The surface's bitmap, selected into its memory context for as long
/// as this lives.
///
/// `PrintWindow` draws into whatever bitmap the context holds, and
/// `GetDIBits` must not read a bitmap that is selected into any context,
/// so the selection is a scope: it ends before the read begins.
struct Selected<'a> {
    surface: &'a Surface,
    previous: HGDIOBJ,
}

impl<'a> Selected<'a> {
    /// # Errors
    /// Refuses when GDI will not select the bitmap.
    #[expect(
        unsafe_code,
        reason = "a bitmap is selected into a context only through the FFI entry point"
    )]
    fn into_memory(surface: &'a Surface) -> Result<Selected<'a>, Refusal> {
        // SAFETY: `surface.memory` and `surface.bitmap` are the live
        // objects that surface owns, and the bitmap is selected into no
        // other context, since only this guard ever selects it.
        let previous = unsafe { SelectObject(surface.memory, HGDIOBJ(surface.bitmap.0)) };
        if previous.is_invalid() {
            return Err(fault::last(
                "select a bitmap to draw the window into",
                "try again; if it persists, ask for a smaller `region`",
            ));
        }
        Ok(Selected { surface, previous })
    }
}

impl Drop for Selected<'_> {
    #[expect(
        unsafe_code,
        reason = "putting the old object back is the FFI call this guard exists to make"
    )]
    fn drop(&mut self) {
        // SAFETY: `previous` is the object `SelectObject` displaced from
        // this same context when the guard was made, and the context is
        // still alive, because the guard borrows the surface that owns it.
        let _bitmap = unsafe { SelectObject(self.surface.memory, self.previous) };
    }
}

impl Drop for Surface {
    #[expect(
        unsafe_code,
        reason = "GDI objects are released only through the FFI entry points, and releasing them is the whole purpose of this Drop"
    )]
    fn drop(&mut self) {
        // SAFETY: each of the three is released exactly once, here, and
        // nothing borrows any of them afterwards — `Surface` hands out
        // no copies of its handles. `self.screen` is the context `GetDC`
        // gave for `self.window`, so it goes back with that same window.
        // Releasing an invalid handle is what the failure path in `over`
        // leaves behind, and each of these three tolerates one, which is
        // why there is no branch here.
        unsafe {
            let _released = DeleteObject(HGDIOBJ(self.bitmap.0));
            let _released = DeleteDC(self.memory);
            ReleaseDC(Some(self.window), self.screen);
        }
    }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "test code"
)]
mod tests {
    use super::*;

    /// A handle that names no window is a refusal with a next step, not
    /// a crash and not a black picture. Whether a real window captures
    /// correctly is an operator's check on a real desktop
    /// (desktop-SPEC.md §16.2).
    #[test]
    fn a_handle_that_names_no_window_is_refused_rather_than_captured() {
        let bounds = Bounds::from_corners(0, 0, 64, 64).unwrap();
        let refusal = window(HWND(std::ptr::null_mut()), bounds)
            .expect_err("no window, no pixels, and no pretending otherwise");
        assert_eq!(refusal.as_error()["data"]["code"], "E_TOOL_UNAVAILABLE");
    }

    /// Capturing a real window over and over leaves this process holding
    /// the GDI objects it held before: every context and bitmap a capture
    /// takes goes back the way it came. A leak here would show as a later
    /// capture failing on a machine where nothing changed.
    #[expect(unsafe_code, reason = "test code creates the window it captures")]
    #[test]
    fn capturing_a_window_gives_back_every_gdi_object_it_took() {
        use windows::Win32::System::Threading::{
            GR_GDIOBJECTS, GetCurrentProcess, GetGuiResources,
        };
        use windows::Win32::UI::WindowsAndMessaging::{
            CreateWindowExW, DestroyWindow, WINDOW_EX_STYLE, WS_POPUP,
        };
        use windows::core::{PCWSTR, w};

        // SAFETY: `STATIC` is a class every process has registered, the
        // name is a null pointer the call accepts for "no title", and no
        // parent, menu, instance or creation data is lent to it.
        let handle = unsafe {
            CreateWindowExW(
                WINDOW_EX_STYLE(0),
                w!("STATIC"),
                PCWSTR::null(),
                WS_POPUP,
                0,
                0,
                64,
                64,
                None,
                None,
                None,
                None,
            )
        }
        .unwrap();
        let bounds = Bounds::from_corners(0, 0, 64, 64).unwrap();
        // SAFETY: the pseudo-handle of this process is always valid for
        // the process that asks, and the call only reads a counter.
        let held = || unsafe { GetGuiResources(GetCurrentProcess(), GR_GDIOBJECTS) };
        // One capture first, so what GDI allocates once per process is
        // not counted as a leak.
        let _first = window(handle, bounds);
        let before = held();
        for _attempt in 0..200 {
            let _captured = window(handle, bounds);
        }
        let after = held();
        // SAFETY: `handle` is the window this test created above on this
        // thread and has not destroyed.
        unsafe { DestroyWindow(handle) }.unwrap();
        assert_eq!(after, before);
    }
}
