// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The window one answer needs, read from the handle the judgement
//! verified, and nothing more (`crates/runtime/spec/Tools/Read.lean`
//! §8-29-1, D10, D11).
//!
//! One call used to read the whole file, hash it whole, and split every
//! line of it before cutting 512. This module reads in 64 KiB chunks,
//! counts newlines as they pass, keeps only the bytes from the wanted
//! line onward, and stops the moment the cut holds. A ranged read whose
//! file the index knows lands on the answer's byte directly (§8-29-7):
//! one seek, one chunk, no scan. The cut accounting is one rule,
//! [`Cutter`], shared with the whole in-memory texts of the catalog.

use std::cell::{OnceCell, RefCell};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::SystemTime;

use kernel::{Address, AxCode, AxError};
use same_file::Handle;

use super::hot::{Hot, Stamp};
use super::index::{Entry, Hit, Index};
use super::miss::{self, Floor};
use super::{Interval, Taken};
use crate::tools::chosen_path::{CityRoot, Located, still_judged};

/// One chunk of the stream, and the most one direct read asks for. An
/// answer may carry twice this: the budget (`INTERVAL_CAP_BYTES`) is the
/// window-policy ceiling, while the chunk is the device-read quantum —
/// 512 ordinary lines fit in one, and a wider answer continues through
/// the next chunk (or the door) instead of paying for the whole budget
/// up front.
const CHUNK: usize = kernel::consts_policy::INTERVAL_CAP_BYTES / 2;

/// The sector a direct read aligns its buffer, its offset and its count
/// to (D11).
const SECTOR: u64 = 4096;
const SECTOR_USIZE: usize = 4096;

/// Whether this platform has a std spelling of unbuffered reads (D11).
/// Everywhere else the first open is already the buffered one.
const DIRECT_CAPABLE: bool = cfg!(any(windows, target_os = "linux"));

/// What one call assembles, besides the interval it cut.
pub(super) struct Window {
    pub taken: Taken,
    /// The whole-file version when it is synchronously knowable: the
    /// window kept the whole file, or the index has it (D10).
    pub version: Option<String>,
    /// An entry a whole read built for free, for the index (§8-29-7).
    pub offer: Option<(PathBuf, Entry)>,
    /// A path worth a background fill, when the index lacks it.
    pub fill: Option<PathBuf>,
}

/// The floor a miss may list up to, decided lazily: ordinary paths pay
/// the first segment's real-location only on the error paths, packages
/// bring the floor their resolution already computed (§8-29-4).
pub(super) enum FloorPlan {
    Address(Address),
    Ready(Floor),
}

impl FloorPlan {
    fn resolve(&self, root: &Path) -> Floor {
        match self {
            FloorPlan::Address(addr) => Floor::of_address(root, addr),
            FloorPlan::Ready(floor) => floor.clone(),
        }
    }
}

/// What every step of one read carries: the call's own facts and the
/// floor its errors are answered from.
#[derive(Clone, Copy)]
struct Reading<'a> {
    asked: &'a str,
    city: &'a CityRoot,
    bound: &'a crate::tools::chosen_path::ReadBound,
    interval: &'a Interval,
    index: &'a Arc<Index>,
    hot: &'a Hot,
    floor: &'a LazyFloor<'a>,
}

/// Bytes already read from a file, from `base` on; `eof` when the file
/// ended inside them.
struct Chunk {
    base: u64,
    bytes: Vec<u8>,
    eof: bool,
}

/// The floor, computed the first time an error path asks for it.
struct LazyFloor<'a> {
    plan: FloorPlan,
    root: &'a Path,
    cell: OnceCell<Floor>,
}

impl LazyFloor<'_> {
    fn get(&self) -> &Floor {
        self.cell.get_or_init(|| self.plan.resolve(self.root))
    }
}

/// One call's facts once the file is open.
struct Job<'a> {
    asked: &'a str,
    interval: &'a Interval,
    real: PathBuf,
    size: u64,
    modified: SystemTime,
    hit: Option<Hit>,
}

/// The judged file, open and verified.
struct Door {
    handle: Handle,
    direct: Directness,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Directness {
    Direct,
    Buffered,
}

/// Where the scan starts: the file's head, or the byte a line starts
/// at when the index has the file (§8-29-7).
#[derive(Clone, Copy)]
enum From {
    Start,
    Line(u64),
}

/// Chunks the door already read, so the scan does not read them again.
enum Pre {
    /// Nothing: every chunk comes from the handle.
    None,
    /// A first chunk that must answer on its own: the leaf that read it
    /// holds no handle to carry on with.
    Only {
        base: u64,
        bytes: Vec<u8>,
        eof: bool,
    },
}

/// What kind of file a read landed on: a city path that has not been
/// landed yet, or a package file whose resolution already landed it.
pub(super) enum FileRef {
    /// The address a model chose, admitted as spelled. Where it lands
    /// is answered by whoever opens it: the leaf at open time, the door
    /// by `land` (§8-29-9).
    City(Address),
    /// A package's file, landed by the package's own resolution.
    Landed(Located),
}

/// Everything one call needs, so `read` takes one parameter.
pub(super) struct Call<'a> {
    pub asked: &'a str,
    pub file: FileRef,
    pub plan: FloorPlan,
    pub city: &'a CityRoot,
    pub bound: &'a crate::tools::chosen_path::ReadBound,
    pub interval: &'a Interval,
    pub index: &'a Arc<Index>,
    pub hot: &'a Hot,
}

/// Reads the window the call asks for, or answers the miss.
pub(super) fn read(call: Call<'_>) -> Result<Window, AxError> {
    let Call {
        asked,
        file,
        plan,
        city,
        bound,
        interval,
        index,
        hot,
    } = call;
    let floor = LazyFloor {
        plan,
        root: city.written(),
        cell: OnceCell::new(),
    };
    let reading = Reading {
        asked,
        city,
        bound,
        interval,
        index,
        hot,
        floor: &floor,
    };
    match file {
        FileRef::City(addr) => {
            // The path the model wrote, with the city's spelling of the
            // root in front: the leaf opens it, links and all, and the
            // open handle is the judgement's subject (§8-29-9).
            let written = addr
                .as_str()
                .split('/')
                .fold(city.written().to_path_buf(), |path, segment| {
                    path.join(segment)
                });
            // Anything but a plain success falls back to `land`: a miss
            // must list the directory the judgement admits, and a
            // directory a link now points at was never judged — the door
            // answers both.
            match leaf_open(&written, CHUNK) {
                Leafed::Absent | Leafed::Refused => {
                    let located = crate::tools::chosen_path::land(city, &addr, "read", &**bound)?;
                    door_read(&reading, located)
                }
                Leafed::Read {
                    bytes,
                    eof,
                    size,
                    modified,
                    final_path,
                } => {
                    // The open pinned the file, so a swap after it moves
                    // nothing this call holds; the address the opened
                    // file really has is admitted before the bytes are
                    // assembled, and a refusal drops them.
                    let real = admit_landed(&reading, &addr, final_path)?;
                    leaf_then(
                        &reading,
                        &real,
                        Stamp { size, modified },
                        Chunk {
                            base: 0,
                            bytes,
                            eof,
                        },
                    )
                }
            }
        }
        FileRef::Landed(at) => door_read(&reading, at),
    }
}

/// The one judgement, applied to the address the opened file really
/// has: `admit` judged the written address already, and the leaf's
/// final path is admitted here the way `land` admits a resolved one
/// (§8-30-1). The handle pins the file, so there is no window in which
/// the path could move between the answer and the read.
fn admit_landed(
    reading: &Reading<'_>,
    addr: &Address,
    final_path: PathBuf,
) -> Result<PathBuf, AxError> {
    let Reading {
        asked, city, bound, ..
    } = *reading;
    let outside = || {
        AxError::failure(
            AxCode::GateDenied,
            "read",
            format!("{asked} leads out of the city through a link"),
        )
        .with_recovery("name a path whose every link stays inside the city")
    };
    let inside = city.spelled_under(&final_path).ok_or_else(outside)?;
    if inside != addr.as_str() {
        crate::tools::chosen_path::admit(&inside, "read", &**bound)?;
    }
    Ok(final_path)
}

/// The read once the file's place is settled: the leaf where the
/// platform has one, the safe door elsewhere.
fn door_read(reading: &Reading<'_>, at: Located) -> Result<Window, AxError> {
    let Reading {
        asked,
        interval,
        index,
        hot,
        floor,
        ..
    } = *reading;
    let real = match at {
        Located::Present(path) => path,
        Located::Absent(path) => {
            return Err(miss::unread(
                asked,
                &path,
                floor.get(),
                &std::io::ErrorKind::NotFound.into(),
            ));
        }
    };
    // One open answers the window and the file's own facts where the
    // leaf exists (D13). A refusal is answered again by the safe door,
    // which owns the miss and the directory faces. A pass in flight is
    // asked for its facts alone, so a continuation is not read twice.
    match leaf_open(&real, if hot.is_empty() { CHUNK } else { 0 }) {
        Leafed::Absent => {}
        Leafed::Refused => {}
        Leafed::Read {
            bytes,
            eof,
            size,
            modified,
            final_path,
        } => {
            // The open answered where the file really is. A link put on
            // the judged path since `land` resolved it leads somewhere
            // else, and that somewhere was never judged.
            if !same_location(&final_path, &real) {
                return Err(changed_after_judgement(asked));
            }
            return leaf_then(
                reading,
                &real,
                Stamp { size, modified },
                Chunk {
                    base: 0,
                    bytes,
                    eof,
                },
            );
        }
    }
    let door = door_open(asked, &real, floor)?;
    let meta = door
        .handle
        .as_file()
        .metadata()
        .map_err(|err| miss::unread(asked, &real, floor.get(), &err))?;
    let size = meta.len();
    let modified = meta
        .modified()
        .map_err(|err| miss::unread(asked, &real, floor.get(), &err))?;
    // The pass may already hold these bytes: serving them costs no
    // device read at all (D14).
    let stamp = Stamp { size, modified };
    if let Some(covered) = hot.covered(&real, stamp, 0, CHUNK) {
        let eof = covered
            .base
            .saturating_add(u64::try_from(covered.bytes.len()).unwrap_or(0))
            >= size;
        return answer_from(
            reading,
            &real,
            stamp,
            Chunk {
                base: covered.base,
                bytes: covered.bytes,
                eof,
            },
        );
    }
    let hit = index.lookup(&real, size, modified, interval.offset);
    let mut job = Job {
        asked,
        interval,
        real,
        size,
        modified,
        hit,
    };
    // A ranged read the index knows lands on the answer (§8-29-7); one
    // past the end needs no bytes at all.
    if let Some(hit) = job.hit.as_ref().filter(|_| interval.offset > 0) {
        let total = u64::try_from(hit.total_lines).unwrap_or(u64::MAX);
        if interval.offset >= total {
            return Ok(Window {
                taken: Taken {
                    text: String::new(),
                    total_lines: Some(hit.total_lines),
                    next_offset: None,
                },
                version: Some(hit.version.clone()),
                offer: None,
                fill: None,
            });
        }
        if let Some(start) = hit.line_start {
            let scan = scan_with_fallback(&door, &job, From::Line(start), floor)?;
            return settle_seek(scan, &mut job, index, |job| {
                scan_with_fallback(&door, job, From::Start, floor)
            });
        }
    }
    let scan = scan_with_fallback(&door, &job, From::Start, floor)?;
    assemble(scan, &job)
}

/// Whether two spellings name the same place. The system answers case
/// it stored, which is not always the case a caller wrote.
fn same_location(final_path: &Path, judged: &Path) -> bool {
    final_path
        .to_str()
        .zip(judged.to_str())
        .is_some_and(|(left, right)| left.eq_ignore_ascii_case(right))
}

/// The refusal for a file that moved between the judgement and the
/// open, which is the same one `still_judged` answers.
fn changed_after_judgement(asked: &str) -> AxError {
    AxError::failure(
        AxCode::GateDenied,
        "read",
        format!("{asked} changed after it was judged"),
    )
    .with_recovery("ask again once nothing is moving the directories on its path")
}

/// What one leaf open answered.
enum Leafed {
    /// This platform has no leaf: the caller's own door answers.
    Absent,
    /// The leaf said no. The caller answers again through its own door,
    /// which owns the miss and the directory faces — so a refusal is
    /// never assembled from bytes nobody judged.
    Refused,
    /// Facts and the first window, read in the one open.
    Read {
        bytes: Vec<u8>,
        /// The window came back shorter than asked: the file ended there.
        eof: bool,
        size: u64,
        modified: SystemTime,
        /// Where the opened file really is, which the caller compares
        /// against what it judged.
        final_path: PathBuf,
    },
}

/// One open answering facts and the window together (`crates/read_ffi/
/// Spec.lean` D1): the open pins the file, so a swap after it moves
/// nothing this call holds. `want` zero asks for the facts alone.
fn leaf_open(path: &Path, want: usize) -> Leafed {
    let sized = u32::try_from(want).unwrap_or(u32::MAX);
    BUFFER.with(|cell| {
        let mut held = cell.borrow_mut();
        let buf = held.slice();
        let ask = read_ffi::Ask {
            path,
            start: 0,
            want: sized,
            direct: DIRECT_CAPABLE,
        };
        match read_ffi::read(&ask, buf) {
            Ok(window) => {
                let n = window.written.min(buf.len());
                Leafed::Read {
                    bytes: buf.get(..n).unwrap_or_default().to_vec(),
                    eof: n < usize::try_from(sized).unwrap_or(usize::MAX),
                    size: window.size,
                    modified: window.modified,
                    final_path: window.final_path,
                }
            }
            Err(read_ffi::Refused::Unsupported) => Leafed::Absent,
            Err(_) => Leafed::Refused,
        }
    })
}

/// The answer once the leaf has opened the file: the pass's own bytes
/// when it holds them, or the window just read. The bytes the first open
/// read are spent rather than read again.
fn leaf_then(
    reading: &Reading<'_>,
    real: &Path,
    stamp: Stamp,
    first: Chunk,
) -> Result<Window, AxError> {
    let asked = reading.asked;
    let Chunk { base, bytes, eof } = first;
    // The pass continues: these bytes cost nothing to serve.
    if let Some(covered) = reading.hot.covered(real, stamp, 0, CHUNK) {
        let eof = covered
            .base
            .saturating_add(u64::try_from(covered.bytes.len()).unwrap_or(0))
            >= stamp.size;
        return answer_from(
            reading,
            real,
            stamp,
            Chunk {
                base: covered.base,
                bytes: covered.bytes,
                eof,
            },
        );
    }
    // A facts-only open left the window unread: read it now, in one
    // more open of the same file.
    let (bytes, eof) = if bytes.is_empty() && !eof {
        match leaf_open(real, CHUNK) {
            Leafed::Read {
                bytes,
                eof,
                final_path,
                ..
            } => {
                if !same_location(&final_path, real) {
                    return Err(changed_after_judgement(asked));
                }
                (bytes, eof)
            }
            Leafed::Absent | Leafed::Refused => (bytes, eof),
        }
    } else {
        (bytes, eof)
    };
    answer_from(reading, real, stamp, Chunk { base, bytes, eof })
}

/// Assembles one answer from bytes that are already in hand, and settles
/// what the pass holds afterwards: a truncated answer keeps its window,
/// a finished one releases it (D14).
fn answer_from(
    reading: &Reading<'_>,
    real: &Path,
    stamp: Stamp,
    chunk: Chunk,
) -> Result<Window, AxError> {
    let Reading {
        asked,
        interval,
        index,
        hot,
        floor,
        ..
    } = *reading;
    let Stamp { size, modified } = stamp;
    let Chunk { base, bytes, eof } = chunk;
    let mut job = Job {
        asked,
        interval,
        real: real.to_path_buf(),
        size,
        modified,
        hit: index.lookup(real, size, modified, interval.offset),
    };
    let sought = job.hit.as_ref().filter(|_| interval.offset > 0);
    if let Some(hit) = sought {
        let total = u64::try_from(hit.total_lines).unwrap_or(u64::MAX);
        if interval.offset >= total {
            hot.release();
            return Ok(Window {
                taken: Taken {
                    text: String::new(),
                    total_lines: Some(hit.total_lines),
                    next_offset: None,
                },
                version: Some(hit.version.clone()),
                offer: None,
                fill: None,
            });
        }
    }
    let from = match (sought, job.hit.as_ref()) {
        (Some(hit), Some(_)) => hit.line_start.map_or(From::Start, From::Line),
        _ => From::Start,
    };
    // Held in case this answer turns out to be a truncated one, in which
    // case these bytes are the head of the pass that follows (D14).
    let held = bytes.clone();
    let scanned = match scan(None, &job, from, Pre::Only { base, bytes, eof }) {
        Ok(scanned) if scanned.complete => scanned,
        // The leaf's one window did not answer, or the walk needs a
        // second one: the door carries on, holding a handle.
        Ok(_) | Err(_) => {
            let door = door_open(asked, real, floor)?;
            scan_with_fallback(&door, &job, from, floor)?
        }
    };
    let answer = match from {
        From::Line(_) => settle_seek(scanned, &mut job, index, |job| {
            let door = door_open(asked, real, floor)?;
            scan_with_fallback(&door, job, From::Start, floor)
        }),
        From::Start => assemble(scanned, &job),
    }?;
    // A truncated answer leaves a pass in flight; a complete one ends it.
    if answer.taken.next_offset.is_some() {
        hot.keep(real, stamp, base, &held);
    } else {
        hot.release();
    }
    Ok(answer)
}

/// The answer of a scan that started where the index's seek table put
/// it, when the table still describes the file. A file rewritten under
/// the same size and mtime is caught here, where the bytes show it: the
/// seek did not land after a newline, or the file ended at another line
/// count than the entry says. The entry is then forgotten and the answer
/// comes from `rescan`, a scan from the head with no index facts (§8-29-7).
fn settle_seek(
    scanned: Scanned,
    job: &mut Job,
    index: &Index,
    rescan: impl FnOnce(&Job) -> Result<Scanned, AxError>,
) -> Result<Window, AxError> {
    if let Some(hit) = job.hit.as_ref() {
        let total = u64::try_from(hit.total_lines).unwrap_or(u64::MAX);
        if scanned.landed && !(scanned.eof && scanned.doc_lines != total) {
            return assemble_seek(scanned, job, hit);
        }
    }
    index.forget(&job.real);
    job.hit = None;
    let scanned = rescan(job)?;
    assemble(scanned, job)
}

/// Opens the judged real path — direct first where the platform spells
/// it (D11), buffered as the retry — and verifies the opened handle is
/// the file that was judged. A directory is answered with its entries
/// and a miss with its neighbours; both are decided on the error paths,
/// so a successful read never pays a metadata call for them.
fn door_open(asked: &str, real: &Path, floor: &LazyFloor) -> Result<Door, AxError> {
    if DIRECT_CAPABLE {
        match open_direct(real).and_then(Handle::from_file) {
            Ok(handle) => {
                still_judged(asked, real, &handle, "read")?;
                return Ok(Door {
                    handle,
                    direct: Directness::Direct,
                });
            }
            Err(_) => {
                if real.is_dir() {
                    return Err(directory(asked, real, floor.get()));
                }
                let handle = std::fs::File::open(real)
                    .and_then(Handle::from_file)
                    .map_err(|err| miss::unread(asked, real, floor.get(), &err))?;
                still_judged(asked, real, &handle, "read")?;
                return Ok(Door {
                    handle,
                    direct: Directness::Buffered,
                });
            }
        }
    }
    let handle = std::fs::File::open(real)
        .and_then(Handle::from_file)
        .map_err(|err| {
            if real.is_dir() {
                directory(asked, real, floor.get())
            } else {
                miss::unread(asked, real, floor.get(), &err)
            }
        })?;
    still_judged(asked, real, &handle, "read")?;
    Ok(Door {
        handle,
        direct: Directness::Buffered,
    })
}

/// The unbuffered open, per platform (D11). Called only where
/// [`DIRECT_CAPABLE`] holds.
#[cfg(windows)]
fn open_direct(real: &Path) -> std::io::Result<std::fs::File> {
    use std::os::windows::fs::OpenOptionsExt;
    const NO_BUFFERING: u32 = 0x2000_0000;
    std::fs::OpenOptions::new()
        .read(true)
        .custom_flags(NO_BUFFERING)
        .open(real)
}

/// The Linux spelling of the same flag.
#[cfg(target_os = "linux")]
fn open_direct(real: &Path) -> std::io::Result<std::fs::File> {
    use std::os::unix::fs::OpenOptionsExt;
    const O_DIRECT: i32 = 0o40000;
    std::fs::OpenOptions::new()
        .read(true)
        .custom_flags(O_DIRECT)
        .open(real)
}

/// Elsewhere no unbuffered open is spelled; [`DIRECT_CAPABLE`] is false
/// there, so this never runs.
#[cfg(not(any(windows, target_os = "linux")))]
fn open_direct(_real: &Path) -> std::io::Result<std::fs::File> {
    Err(std::io::ErrorKind::Unsupported.into())
}

/// A directory is a question about what is in it: answered with its
/// entries, which is what a read of one asks for (§8-29-4).
fn directory(asked: &str, real: &Path, floor: &Floor) -> AxError {
    AxError::failure(
        AxCode::InvalidArgs,
        "read",
        format!("{asked} is a directory"),
    )
    .with_nearby(miss::entries(real, floor, ""))
    .with_recovery("read one of the entries in `nearby`, or find a file with `search`")
}

/// A read that failed after the open: a directory is answered with its
/// entries (Unix opens one and fails at the read), anything else is the
/// miss face or storage's (§8-29-4).
fn map_scan_error(asked: &str, real: &Path, floor: &LazyFloor, err: std::io::Error) -> AxError {
    if real.is_dir() {
        directory(asked, real, floor.get())
    } else {
        miss::unread(asked, real, floor.get(), &err)
    }
}

// The aligned buffer every scan reuses, one per thread: a wave runs
// reads on scoped threads, and each thread keeps its own.
thread_local! {
    static BUFFER: RefCell<Aligned> = RefCell::new(Aligned::new());
}

/// A chunk's room, aligned to the sector a direct read demands.
struct Aligned {
    raw: Vec<u8>,
    off: usize,
}

impl Aligned {
    fn new() -> Aligned {
        let raw = vec![0u8; CHUNK.saturating_add(SECTOR_USIZE)];
        let misaligned = raw.as_ptr().addr().rem_euclid(SECTOR_USIZE);
        let off = SECTOR_USIZE
            .saturating_sub(misaligned)
            .rem_euclid(SECTOR_USIZE);
        Aligned { raw, off }
    }

    fn slice(&mut self) -> &mut [u8] {
        self.raw
            .get_mut(self.off..self.off.saturating_add(CHUNK))
            .unwrap_or_default()
    }
}

/// One read at an absolute position.
#[cfg(windows)]
fn read_at(file: &std::fs::File, buf: &mut [u8], at: u64, want: usize) -> std::io::Result<usize> {
    use std::os::windows::fs::FileExt;
    file.seek_read(buf.get_mut(..want).unwrap_or_default(), at)
}

/// The Unix spelling of the same read.
#[cfg(unix)]
fn read_at(file: &std::fs::File, buf: &mut [u8], at: u64, want: usize) -> std::io::Result<usize> {
    use std::os::unix::fs::FileExt;
    file.read_at(buf.get_mut(..want).unwrap_or_default(), at)
}

/// How many bytes the next chunk asks for: the rest of the file, capped
/// at one chunk, rounded up to the sector when the read is direct. The
/// tail past the file's end reads back undefined and is cut away by the
/// caller, which knows the real size from the handle.
fn want(pos: u64, size: u64, direct: Directness) -> usize {
    let remaining = size.saturating_sub(pos);
    let chunk = u64::try_from(CHUNK).unwrap_or(u64::MAX);
    let wanted = match direct {
        Directness::Direct => remaining.min(chunk).next_multiple_of(SECTOR),
        Directness::Buffered => remaining.min(chunk),
    };
    usize::try_from(wanted).unwrap_or(CHUNK)
}

/// What a finished scan knows: the kept bytes, the document-wide line
/// count, and the cut accounting.
struct Scanned {
    /// The answer needs no more bytes: the cut holds, or the file ended.
    complete: bool,
    retained: Vec<u8>,
    doc_lines: u64,
    /// Line starts, recorded only on a scan from the head of a file the
    /// caller reads from line zero: the index entry's seek table
    /// (§8-29-7).
    line_offsets: Vec<u64>,
    eof: bool,
    delivered: u64,
    scanned: u64,
    /// A seek's first chunk shows a newline right before the line the
    /// index said starts there; always true for a scan from the head.
    landed: bool,
}

/// Reads chunks, counts lines, keeps the window. Retries buffered once
/// when the direct attempt fails anywhere (D11); the retried handle is
/// verified again, because reading happens only from a handle that was
/// judged.
fn scan_with_fallback(
    door: &Door,
    job: &Job,
    from: From,
    floor: &LazyFloor,
) -> Result<Scanned, AxError> {
    match scan(Some(door), job, from, Pre::None) {
        Ok(found) => Ok(found),
        Err(first) => {
            if door.direct == Directness::Buffered {
                return Err(map_scan_error(job.asked, &job.real, floor, first));
            }
            let handle = std::fs::File::open(&job.real)
                .and_then(Handle::from_file)
                .map_err(|err| map_scan_error(job.asked, &job.real, floor, err))?;
            still_judged(job.asked, &job.real, &handle, "read")?;
            let again = Door {
                handle,
                direct: Directness::Buffered,
            };
            scan(Some(&again), job, from, Pre::None)
                .map_err(|err| map_scan_error(job.asked, &job.real, floor, err))
        }
    }
}

/// The scan itself. Every byte is read at most once, a byte before the
/// wanted line is never kept, and the whole of the last chunk is
/// counted, so a file that goes on past the chunk a cut was made in is
/// still known to be longer than what was delivered.
fn scan(door: Option<&Door>, job: &Job, from: From, pre: Pre) -> std::io::Result<Scanned> {
    let offset = job.interval.offset;
    // A seek starts at the sector the byte before the wanted line falls
    // in, so that byte — the newline the seek must land after — is in
    // the first chunk; the bytes up to the line itself belong to the
    // line before it and are dropped below.
    let (mut pos, mut doc_lines, mut abs_start, mut pending) = match from {
        From::Start => (0u64, 0u64, 0u64, 0usize),
        From::Line(start) => {
            let before = start.saturating_sub(1);
            let aligned = before.saturating_sub(before % SECTOR);
            let skip = usize::try_from(start.saturating_sub(aligned)).unwrap_or(0);
            (aligned, offset, start, skip)
        }
    };
    let mut retained: Vec<u8> = Vec::with_capacity(CHUNK);
    let mut started = matches!(from, From::Line(_)) || offset == 0;
    let mut line_offsets: Vec<u64> = Vec::new();
    let mut cutter = Cutter::new(job.interval.limit);
    // Where the last delivered line ends, in the file's bytes: the
    // scan already knows it, so the answer is cut here rather than by a
    // second pass over the window.
    let mut delivered_end = 0u64;
    let record = matches!(from, From::Start) && offset == 0;
    let mut size = job.size;
    let mut pre = pre;
    let only = matches!(pre, Pre::Only { .. });
    let mut landed = true;
    let mut first = true;
    BUFFER.with(|cell| -> std::io::Result<()> {
        let mut held = cell.borrow_mut();
        let buf = held.slice();
        while pos < size {
            let valid = match std::mem::replace(&mut pre, Pre::None) {
                Pre::Only { base, bytes, eof } => {
                    // A chunk the door already read: it is copied into
                    // the scan's own buffer so the one walk below reads
                    // it the same way it reads every other chunk.
                    let n = bytes.len().min(buf.len());
                    if let (Some(into), Some(from)) = (buf.get_mut(..n), bytes.get(..n)) {
                        into.copy_from_slice(from);
                    }
                    if base != pos {
                        return Err(std::io::ErrorKind::InvalidData.into());
                    }
                    // A chunk shorter than one window ended the file.
                    if eof {
                        size = pos.saturating_add(u64::try_from(n).unwrap_or(0));
                    }
                    n
                }
                Pre::None => {
                    let Some(door) = door else {
                        return Err(std::io::ErrorKind::Unsupported.into());
                    };
                    let got = read_at(
                        door.handle.as_file(),
                        buf,
                        pos,
                        want(pos, size, door.direct),
                    )?;
                    let valid64 = u64::try_from(got)
                        .unwrap_or(u64::MAX)
                        .min(size.saturating_sub(pos));
                    if valid64 == 0 {
                        // The file shrank under the read: the shrunk
                        // point is its end now.
                        size = pos;
                        break;
                    }
                    usize::try_from(valid64).unwrap_or(got)
                }
            };
            let valid64 = u64::try_from(valid).unwrap_or(u64::MAX);
            let chunk = buf.get(..valid).unwrap_or_default();
            let base = pos;
            if let (true, From::Line(start)) = (std::mem::replace(&mut first, false), from)
                && start > 0
            {
                let at = usize::try_from(start.saturating_sub(1).saturating_sub(base)).ok();
                if at.and_then(|at| chunk.get(at)) != Some(&b'\n') {
                    landed = false;
                    break;
                }
            }
            let was_started = started;
            let mut start_in_chunk = 0usize;
            for at in memchr::memchr_iter(b'\n', chunk) {
                let nl = base.saturating_add(u64::try_from(at).unwrap_or(u64::MAX));
                // A seek reads from the sector boundary before the
                // wanted line, so the first newlines it meets end lines
                // that come before the answer: they are not the wanted
                // line, and counting them would shorten the answer by
                // exactly the lead-in.
                if nl.saturating_add(1) <= abs_start {
                    continue;
                }
                // The line [abs_start, nl + 1) is line number
                // `doc_lines`. Its start is recorded before `feed` may
                // close the cut, so the line that closes it is in the
                // table too.
                if record && !cutter.cut() {
                    line_offsets.push(abs_start);
                }
                if started {
                    let before = cutter.delivered;
                    cutter.feed(nl.saturating_add(1).saturating_sub(abs_start));
                    if cutter.delivered > before {
                        delivered_end = nl.saturating_add(1);
                    }
                }
                doc_lines = doc_lines.saturating_add(1);
                abs_start = nl.saturating_add(1);
                if !started && doc_lines == offset {
                    // The wanted line starts here, inside this chunk.
                    started = true;
                    start_in_chunk = usize::try_from(abs_start.saturating_sub(base)).unwrap_or(0);
                    pending = 0;
                }
            }
            // One place decides which bytes of this chunk are kept: the
            // seek's own lead-in on the first chunk, then the wanted
            // line's start, then all of it.
            let keep_from = if was_started {
                std::mem::replace(&mut pending, 0)
            } else if started {
                start_in_chunk
            } else {
                valid
            };
            let keep_to = if cutter.cut() {
                usize::try_from(delivered_end.saturating_sub(base))
                    .unwrap_or(valid)
                    .min(valid)
            } else {
                valid
            };
            if let Some(kept) = chunk.get(keep_from..keep_to) {
                retained.extend_from_slice(kept);
            }
            pos = pos.saturating_add(valid64);
            if cutter.cut() || only {
                break;
            }
        }
        Ok(())
    })?;
    // A trailing line without its newline still counts (§8-29-1); its
    // bytes were kept as the chunks passed.
    if pos == size && abs_start < size {
        if started {
            // These bytes are already in the window: the chunk that
            // carried them was appended whole because no cut had been
            // made yet when it passed. A line the cut turns away leaves
            // the window again.
            let tail = size.saturating_sub(abs_start);
            let open = !cutter.cut();
            let before = cutter.delivered;
            cutter.feed(tail);
            if open && cutter.delivered == before {
                let kept =
                    usize::try_from(tail).map_or(0, |tail| retained.len().saturating_sub(tail));
                retained.truncate(kept);
            }
        }
        if record && !cutter.cut() {
            line_offsets.push(abs_start);
        }
        doc_lines = doc_lines.saturating_add(1);
    }
    Ok(Scanned {
        complete: !landed || cutter.cut() || pos == size,
        retained,
        doc_lines,
        line_offsets,
        eof: pos == size,
        delivered: cutter.delivered,
        scanned: cutter.scanned,
        landed,
    })
}

/// The cut accounting, one rule for the stream and for whole texts in
/// memory (§8-29-1): at most `limit` lines, at most
/// `INTERVAL_CAP_BYTES` bytes, and the first line however long it is.
struct Cutter {
    limit: u64,
    scanned: u64,
    delivered: u64,
    spent: u64,
    cut: bool,
}

impl Cutter {
    fn new(limit: u16) -> Cutter {
        Cutter {
            limit: u64::from(limit),
            scanned: 0,
            delivered: 0,
            spent: 0,
            cut: false,
        }
    }

    /// One completed line, counted whether or not the answer still
    /// takes lines; delivery stops at the first ceiling the next line
    /// would cross.
    fn feed(&mut self, len: u64) {
        self.scanned = self.scanned.saturating_add(1);
        if self.cut {
            return;
        }
        if self.delivered >= self.limit {
            self.cut = true;
            return;
        }
        let after = self.spent.saturating_add(len);
        if after > self.budget() && self.delivered > 0 {
            self.cut = true;
            return;
        }
        self.spent = after;
        self.delivered = self.delivered.saturating_add(1);
        if self.delivered >= self.limit {
            self.cut = true;
        }
    }

    fn budget(&self) -> u64 {
        u64::try_from(kernel::consts_policy::INTERVAL_CAP_BYTES).unwrap_or(u64::MAX)
    }

    fn cut(&self) -> bool {
        self.cut
    }
}

/// The window's bytes as text, cut to the delivered lines. The window,
/// not the whole file, is what "is text" is judged on (§8-29-4).
fn validated(asked: &str, retained: Vec<u8>) -> Result<String, AxError> {
    String::from_utf8(retained).map_err(|_| {
        AxError::failure(AxCode::InvalidArgs, "read", format!("{asked} is not text"))
            .with_recovery("read hands back text; these bytes are not UTF-8")
    })
}

/// The stream mode's assembly: the interval, the fields the moment
/// knows, the index entry a whole scan built, and whether the index
/// should be filled behind the answer.
fn assemble(scan: Scanned, job: &Job) -> Result<Window, AxError> {
    let offset = job.interval.offset;
    let truncated = scan.delivered < scan.scanned || !scan.eof;
    let past_end = scan.eof && offset >= scan.doc_lines;
    let total_lines = if past_end || (truncated && scan.eof) {
        usize::try_from(scan.doc_lines).ok()
    } else if truncated {
        job.hit.as_ref().map(|hit| hit.total_lines)
    } else {
        None
    };
    // The whole file is in the window only when the scan reached its end
    // from line zero and withheld no line: a cut can see the end of the
    // file in the chunk it was made in and still keep less than all of it.
    let whole = scan.eof && offset == 0 && !truncated;
    let line_offsets = scan.line_offsets;
    let doc_lines = scan.doc_lines;
    let text = validated(job.asked, scan.retained)?;
    // The one moment the version costs nothing beyond the bytes held.
    let version = if whole {
        Some(crate::tools::version_of(text.as_bytes()))
    } else {
        job.hit.as_ref().map(|hit| hit.version.clone())
    };
    let next_offset = if truncated {
        usize::try_from(offset.saturating_add(scan.delivered)).ok()
    } else {
        None
    };
    let offer = if whole {
        version.clone().map(|version| {
            (
                job.real.clone(),
                Entry {
                    size: job.size,
                    modified: job.modified,
                    total_lines: usize::try_from(doc_lines).unwrap_or(usize::MAX),
                    version,
                    line_offsets,
                },
            )
        })
    } else {
        None
    };
    let fill = if !whole && job.hit.is_none() && Index::eligible(job.size) {
        Some(job.real.clone())
    } else {
        None
    };
    Ok(Window {
        taken: Taken {
            text,
            total_lines,
            next_offset,
        },
        version,
        offer,
        fill,
    })
}

/// The seek mode's assembly: the index already holds the total and the
/// version, so the answer is complete with one chunk read.
fn assemble_seek(scan: Scanned, job: &Job, hit: &Hit) -> Result<Window, AxError> {
    let text = validated(job.asked, scan.retained)?;
    let total = u64::try_from(hit.total_lines).unwrap_or(u64::MAX);
    let end = job.interval.offset.saturating_add(scan.delivered);
    let truncated = end < total;
    Ok(Window {
        taken: Taken {
            text,
            total_lines: if truncated {
                Some(hit.total_lines)
            } else {
                None
            },
            next_offset: if truncated {
                usize::try_from(end).ok()
            } else {
                None
            },
        },
        version: Some(hit.version.clone()),
        offer: None,
        fill: None,
    })
}

/// The cut of a text already in memory: catalog entries and Locator
/// bytes are whole documents, so their totals are always known and
/// their `more` is always false (§8-29-1).
pub(super) fn cut_known(text: &str, offset: u64, limit: u16) -> Taken {
    let total = text.split_inclusive('\n').count();
    let total64 = u64::try_from(total).unwrap_or(u64::MAX);
    if offset >= total64 {
        return Taken {
            text: String::new(),
            total_lines: Some(total),
            next_offset: None,
        };
    }
    let skip = usize::try_from(offset).unwrap_or(usize::MAX);
    let mut lines = text.split_inclusive('\n');
    // The byte the wanted line starts at: the answer is sliced from here,
    // not from the head of the text.
    let start = lines
        .by_ref()
        .take(skip)
        .fold(0usize, |at, line| at.saturating_add(line.len()));
    let mut cutter = Cutter::new(limit);
    let mut delivered_end = start;
    for line in lines {
        if cutter.cut() {
            break;
        }
        let before = cutter.delivered;
        cutter.feed(u64::try_from(line.len()).unwrap_or(u64::MAX));
        if cutter.delivered > before {
            delivered_end = delivered_end.saturating_add(line.len());
        }
    }
    let truncated = delivered_end < text.len();
    Taken {
        text: text
            .get(start..delivered_end)
            .unwrap_or_default()
            .to_owned(),
        total_lines: if truncated { Some(total) } else { None },
        next_offset: if truncated {
            usize::try_from(offset.saturating_add(cutter.delivered)).ok()
        } else {
            None
        },
    }
}
