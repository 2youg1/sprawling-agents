// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The leaf's one export as Rust sees it, and the safe function around
//! it (`crates/read_ffi/Spec.lean` §1).
//!
//! The leaf opens the file, asks the handle where it really lives, how
//! big it is and when it last changed, and reads the window the caller
//! asked for — all from the one handle, so there is no moment between a
//! judgement and an open in which a link could be swapped. The leaf
//! decides no policy: whether the final path lies inside the city, and
//! what address it names there, stays in `runtime::tools::chosen_path`.

use std::fmt;
use std::path::{Path, PathBuf};
use std::time::SystemTime;
#[cfg(windows)]
use std::time::{Duration, UNIX_EPOCH};

/// What the leaf answers beside the bytes: the file's own facts and
/// where the final path was written.
#[cfg(windows)]
#[repr(C)]
#[derive(Debug, Default, Clone, Copy)]
struct Drawn {
    written: usize,
    size: u64,
    modified_nanos: u64,
    final_len: usize,
}

#[cfg(windows)]
#[expect(
    unsafe_code,
    reason = "the one declaration of the leaf's export; the call into it is an `unsafe` block of its own"
)]
unsafe extern "C" {
    fn sprawling_read_window(
        name: *const u16,
        start: u64,
        want: u32,
        direct: u32,
        into: *mut u8,
        capacity: usize,
        final_into: *mut u16,
        final_capacity: u32,
        drawn: *mut Drawn,
    ) -> u32;
}

/// Where the bytes came from: the file's real location, its size, and
/// when it last changed.
#[derive(Debug, Clone)]
pub struct Window {
    /// The real location of the handle that was read, in the verbatim
    /// spelling the system stores and `std::fs::canonicalize` answers.
    pub final_path: PathBuf,
    pub size: u64,
    pub modified: SystemTime,
    /// How many bytes of the caller's buffer the leaf filled.
    pub written: usize,
}

/// Why the leaf answered nothing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Refused {
    Missing,
    Directory,
    Unreadable,
    ReadFailed,
    PathTooLong,
    Unsupported,
}

impl fmt::Display for Refused {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let what = match self {
            Refused::Missing => "the file is not there",
            Refused::Directory => "the path is a directory",
            Refused::Unreadable => "the file would not open",
            Refused::ReadFailed => "the open file would not read",
            Refused::PathTooLong => "the file's real path is longer than the leaf's buffer",
            Refused::Unsupported => "this platform has no read leaf",
        };
        write!(f, "{what}")
    }
}

/// One read of one window.
pub struct Ask<'a> {
    /// The path to open, as the judgement spelled it.
    pub path: &'a Path,
    /// The byte the window starts at; the caller aligns it to the sector
    /// when it asks for a direct read.
    pub start: u64,
    /// How many bytes to ask the device for. `into` holds at least this
    /// many, and zero asks for the file's facts alone.
    pub want: u32,
    /// Whether the read bypasses the page cache.
    pub direct: bool,
}

/// The longest real path the leaf is given room for. `MAX_PATH` is 260
/// and a city root is far below it; a longer one is answered as
/// [`Refused::PathTooLong`] rather than truncated.
#[cfg(windows)]
const FINAL_CAPACITY: usize = 1024;

/// Opens `ask.path`, reads its window into `into`, and answers where the
/// opened file really lives.
///
/// `into` holds at least `ask.want` bytes, and both its address and
/// `ask.start` are whole sectors when `ask.direct` holds — which is why
/// the caller hands an aligned buffer.
///
/// # Errors
/// The leaf's own refusals, each naming what the disk did; a platform
/// without the leaf answers [`Refused::Unsupported`], and the caller
/// keeps its own door.
#[cfg(windows)]
pub fn read(ask: &Ask<'_>, into: &mut [u8]) -> Result<Window, Refused> {
    use std::os::windows::ffi::OsStrExt as _;

    let mut wide: Vec<u16> = ask.path.as_os_str().encode_wide().collect();
    wide.push(0);
    let mut final_wide = vec![0u16; FINAL_CAPACITY];
    let mut drawn = Drawn::default();
    let capacity = into.len();
    if capacity < usize::try_from(ask.want).unwrap_or(usize::MAX) {
        return Err(Refused::ReadFailed);
    }
    #[expect(unsafe_code, reason = "the one call into the read window's Zig leaf")]
    // SAFETY: `wide` is live and null-terminated; `into` is live and at
    // least `capacity` bytes long; `final_wide` is live and
    // `FINAL_CAPACITY` long; `drawn` is a live `Drawn` this call owns;
    // the leaf writes no further than the capacities it is given and
    // reads nothing else.
    let status = unsafe {
        sprawling_read_window(
            wide.as_mut_ptr(),
            ask.start,
            ask.want,
            u32::from(ask.direct),
            into.as_mut_ptr(),
            capacity,
            final_wide.as_mut_ptr(),
            u32::try_from(FINAL_CAPACITY).unwrap_or(u32::MAX),
            &raw mut drawn,
        )
    };
    match status {
        0 => {
            let end = drawn.final_len.min(final_wide.len());
            let final_path = PathBuf::from(String::from_utf16_lossy(
                final_wide.get(..end).unwrap_or_default(),
            ));
            Ok(Window {
                final_path,
                size: drawn.size,
                modified: epoch_plus(drawn.modified_nanos),
                written: drawn.written.min(into.len()),
            })
        }
        1 => Err(Refused::Missing),
        2 => Err(Refused::Directory),
        3 => Err(Refused::Unreadable),
        4 => Err(Refused::ReadFailed),
        5 => Err(Refused::PathTooLong),
        _ => Err(Refused::Unreadable),
    }
}

/// The platform without the leaf: the caller's own door answers.
///
/// # Errors
/// Always [`Refused::Unsupported`].
#[cfg(not(windows))]
pub fn read(_ask: &Ask<'_>, _into: &mut [u8]) -> Result<Window, Refused> {
    Err(Refused::Unsupported)
}

/// The stamp the leaf reports, back as a `SystemTime`: it counts
/// nanoseconds since the Unix epoch.
#[cfg(windows)]
fn epoch_plus(nanos: u64) -> SystemTime {
    UNIX_EPOCH
        .checked_add(Duration::from_nanos(nanos))
        .unwrap_or(UNIX_EPOCH)
}
