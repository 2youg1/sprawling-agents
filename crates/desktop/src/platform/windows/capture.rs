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
//! (desktop-SPEC.md §8.6, first pair). A recording takes every frame
//! through [`window`] as well, so what holds for a screenshot holds for
//! each frame of a recording (desktop-SPEC.md section 12.7).
//!
//! `PW_RENDERFULLCONTENT` is what makes a hardware-composited window —
//! most browsers, most editors — render into the bitmap instead of
//! arriving blank. It does not always work, and when it does not the
//! result is a rectangle of pure black. That is checked for and refused:
//! a model shown a black picture of a window will describe the black.

use desktop_ffi::ended::Failure;
use desktop_ffi::step::Step;

use super::fault;
use super::geometry::Bounds;
use crate::refusal::{Refusal, RefusalCode};

/// What every refusal of a capture names as the action that failed.
const DOING: &str = "capture a window";

/// One window's pixels, in the order `image` reads them.
///
/// The GDI half — a context for the window, a bitmap of its size, the
/// drawing and the read back — is the Zig leaf's, which releases every
/// object it took on every path (desktop-SPEC.md section 8-12). What
/// the pixels mean is decided here.
///
/// # Errors
/// Refuses a window GDI will not draw into a bitmap, a size this machine
/// will not allocate, and a capture that came back entirely black.
pub(crate) fn window(handle: &winsafe::HWND, bounds: Bounds) -> Result<image::RgbaImage, Refusal> {
    let (width, height) = (bounds.width(), bounds.height());
    let (Ok(pixel_width), Ok(pixel_height)) = (i32::try_from(width), i32::try_from(height)) else {
        return Err(too_large(width, height));
    };
    let bgra = desktop_ffi::capture::pixels(handle, pixel_width, pixel_height)
        .map_err(|failure| refused(failure, width, height))?;
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
        return Err(drew_nothing());
    }
    Ok(pixels)
}

/// A capture that came back entirely black, which is refused rather
/// than shown: a model shown a black picture of a window describes the
/// black.
fn drew_nothing() -> Refusal {
    Refusal::new(
        RefusalCode::ToolUnavailable,
        DOING,
        "the window drew nothing: every pixel came back black".to_owned(),
        "this window composes itself in a way GDI cannot read — bring it to the front and try \
         again, or read it with `desktop.snapshot`, which does not go through pixels",
    )
}

/// A window too big to measure in the units GDI counts in.
fn too_large(width: u32, height: u32) -> Refusal {
    Refusal::new(
        RefusalCode::InvalidArgs,
        DOING,
        format!("a {width}x{height} window is more than this machine will draw at once"),
        "ask for a `region` of the window rather than the whole of it",
    )
}

/// The step the leaf stopped at, as the sentence a caller can act on.
fn refused(failure: Failure, width: u32, height: u32) -> Refusal {
    let Failure::At { step, code } = failure else {
        return fault::leaf(DOING, "try again", failure);
    };
    let machine = |doing: &str, recovery: &str| fault::system(doing, recovery, code);
    match step {
        // `GetDC` of no window is the context of the whole screen, which
        // is exactly what this module exists not to read.
        Step::NoWindow => Refusal::new(
            RefusalCode::ToolUnavailable,
            DOING,
            "no window was named".to_owned(),
            "call `desktop.windows` again and name a window from it",
        ),
        Step::Measuring => too_large(width, height),
        Step::Context => machine(
            "borrow the window's drawing context",
            "call `desktop.windows` again; the window may have closed since it was named",
        ),
        Step::Bitmap => machine(
            "make a bitmap the size of this window",
            "ask for a `region` of the window, or close something: this machine is out of \
             drawing resources",
        ),
        Step::Selecting => machine(
            "select a bitmap to draw the window into",
            "try again; if it persists, ask for a smaller `region`",
        ),
        Step::Drawing => machine(
            "ask the window to draw itself",
            "bring the window to the front and try again; a minimised window has nothing to draw",
        ),
        Step::Reading => machine(
            "read the captured pixels back",
            "try again; if it persists, ask for a smaller `region`",
        ),
        Step::ShortRows => Refusal::new(
            RefusalCode::ToolUnavailable,
            DOING,
            "GDI read back fewer rows than the window has".to_owned(),
            "try again; a picture with rows missing is not handed over",
        ),
        Step::Finished
        | Step::Absent
        | Step::NoRoom
        | Step::Listing
        | Step::Owner
        | Step::Opening
        | Step::Fetching
        | Step::Locking
        | Step::EmptyBlock
        | Step::Allocating
        | Step::Emptying
        | Step::Handing => fault::stray(DOING, step),
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
        use winsafe::prelude::Handle;

        let bounds = Bounds::from_corners(0, 0, 64, 64).unwrap();
        let refusal = window(&winsafe::HWND::NULL, bounds)
            .expect_err("no window, no pixels, and no pretending otherwise");
        assert_eq!(refusal.as_error()["data"]["code"], "E_TOOL_UNAVAILABLE");
    }

    /// The contract desktop-SPEC.md section 8-11 holds the capture row
    /// to, whichever interface answers it: every GDI step runs to the
    /// end for a window this process opens — a context, a bitmap of its
    /// size, the drawing and every row read back — so what comes back is
    /// its pixels at its own size, or the one refusal that is judged on
    /// pixels already read, the all-black one. The window sits off every
    /// monitor, so the person at this desktop does not see it, and there
    /// a composed window may draw black.
    #[test]
    fn a_window_this_process_opens_is_read_back_whole() {
        let title = format!("sprawling contract capture {}", std::process::id());
        let opened = super::super::fixture::Opened::at(&title, -20_000, -20_000, None);
        let named = super::super::enumerate::desktop()
            .unwrap()
            .into_iter()
            .find(|listed| listed.named.title == title)
            .unwrap();
        match window(&named.handle, named.bounds) {
            Ok(pixels) => assert_eq!(
                (pixels.width(), pixels.height()),
                (opened.bounds().width(), opened.bounds().height())
            ),
            Err(refusal) => assert_eq!(refusal, drew_nothing()),
        }
    }

    /// Capturing a real window over and over leaves this process holding
    /// the GDI objects it held before: every context and bitmap a capture
    /// takes goes back the way it came. A leak here would show as a later
    /// capture failing on a machine where nothing changed.
    #[test]
    fn capturing_a_window_gives_back_every_gdi_object_it_took() {
        let title = format!("sprawling contract gdi {}", std::process::id());
        let opened = super::super::fixture::Opened::at(&title, -20_000, -20_000, None);
        let captured = opened.handle();
        let held = || {
            winsafe::HPROCESS::GetCurrentProcess()
                .GetGuiResources(winsafe::co::GR::GDIOBJECTS)
                .unwrap()
        };
        // One capture first, so what GDI allocates once per process is
        // not counted as a leak.
        let _first = window(&captured, opened.bounds());
        let before = held();
        for _attempt in 0..200 {
            let _captured = window(&captured, opened.bounds());
        }
        assert_eq!(held(), before);
    }
}
