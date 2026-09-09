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

use super::fault;
use super::geometry::Bounds;
use crate::refusal::{Refusal, RefusalCode};

/// Render the window's real content, not the stale cache of it.
const PW_RENDERFULLCONTENT: PRINT_WINDOW_FLAGS = PRINT_WINDOW_FLAGS(0x0000_0002);

/// One window's pixels, in the order `image` reads them.
///
/// # Errors
/// Refuses a window GDI will not draw into a bitmap, a size this machine
/// will not allocate, and a capture that came back entirely black.
pub(crate) fn window(handle: HWND, bounds: Bounds) -> Result<image::RgbaImage, Refusal> {
    let (width, height) = (bounds.width(), bounds.height());
    let (Ok(pixel_width), Ok(pixel_height)) = (i32::try_from(width), i32::try_from(height)) else {
        return Err(too_large(width, height));
    };
    let bgra = Surface::over(handle, pixel_width, pixel_height)?.drawn(handle)?;
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
    /// bottom-up BGRA.
    #[expect(
        unsafe_code,
        reason = "PrintWindow is the one call that asks a single window for its own pixels, which is what makes the unit of capture the unit of permission"
    )]
    fn drawn(&self, handle: HWND) -> Result<Vec<u8>, Refusal> {
        // SAFETY: `self.memory` and `self.bitmap` are the non-null
        // objects this value owns and has not released. Selecting the
        // bitmap into the memory context is what gives `PrintWindow`
        // somewhere to draw; the previous object is the stock bitmap,
        // which `DeleteDC` disposes of with the context.
        let previous = unsafe { SelectObject(self.memory, HGDIOBJ(self.bitmap.0)) };
        // SAFETY: `handle` names the window being captured and
        // `self.memory` is the context the bitmap is now selected into.
        // `PrintWindow` writes only into that bitmap, whose size was
        // fixed from this window's own measurement.
        let drew = unsafe { PrintWindow(handle, self.memory, PW_RENDERFULLCONTENT) }.as_bool();
        let read = drew.then(|| self.bits()).transpose();
        // SAFETY: `previous` is whatever `SelectObject` displaced above,
        // which is a valid object for this context by construction, and
        // `self.memory` is still the live context it came out of.
        unsafe { SelectObject(self.memory, previous) };
        match read? {
            Some(bits) => Ok(bits),
            None => Err(fault::last(
                "ask the window to draw itself",
                "bring the window to the front and try again; a minimised window has nothing to \
                 draw",
            )),
        }
    }

    /// The bitmap's pixels, top row first.
    #[expect(
        unsafe_code,
        reason = "GetDIBits is the only way to read a GDI bitmap back into memory this process owns"
    )]
    fn bits(&self) -> Result<Vec<u8>, Refusal> {
        let mut info = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: u32::try_from(std::mem::size_of::<BITMAPINFOHEADER>()).unwrap_or_default(),
                biWidth: self.width,
                // Negative height is how GDI is told to hand back a
                // top-down image. Without it every capture is upside
                // down, which is the kind of defect that survives review
                // because nothing in the code says which way is up.
                biHeight: self.height.checked_neg().unwrap_or(self.height),
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB.0,
                ..BITMAPINFOHEADER::default()
            },
            ..BITMAPINFO::default()
        };
        let count = usize::try_from(self.width)
            .ok()
            .and_then(|width| width.checked_mul(usize::try_from(self.height).ok()?))
            .and_then(|pixels| pixels.checked_mul(4))
            .ok_or_else(|| fault::last("measure the capture", "ask for a `region` instead"))?;
        let mut bits: Vec<u8> = vec![0; count];
        // SAFETY: `self.memory` and `self.bitmap` are the live objects
        // this value owns. `bits` holds exactly the number of bytes the
        // header above describes — width times height times four, for
        // the 32 bits per pixel it declares — so the buffer Win32 fills
        // is the buffer that exists. `info` is a live local of the type
        // the parameter names.
        let rows = unsafe {
            GetDIBits(
                self.memory,
                self.bitmap,
                0,
                u32::try_from(self.height).unwrap_or_default(),
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
        Ok(bits)
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
        // no copies of its handles. Releasing an invalid handle is what
        // the failure path in `over` leaves behind, and each of these
        // three tolerates one, which is why there is no branch here.
        unsafe {
            let _released = DeleteObject(HGDIOBJ(self.bitmap.0));
            let _released = DeleteDC(self.memory);
            ReleaseDC(None, self.screen);
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

    /// Repeating the failing path does not leak the bounded resource
    /// GDI hands out: `Surface::drop` runs on the error path too. A leak
    /// here would show as a later capture failing on a machine where
    /// nothing changed.
    #[test]
    fn the_failing_path_releases_what_it_took() {
        let bounds = Bounds::from_corners(0, 0, 32, 32).unwrap();
        for _attempt in 0..200 {
            assert!(window(HWND(std::ptr::null_mut()), bounds).is_err());
        }
    }
}
