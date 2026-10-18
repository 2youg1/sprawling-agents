// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! What the leaf must agree with, checked against the safe path it
//! replaces (`crates/read_ffi/Spec.lean` §2, §3).

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    reason = "test code"
)]

use std::io::Write as _;

use crate::Ask;
#[cfg(not(windows))]
use crate::Refused;

/// A file of `len` bytes under `dir`, written and closed.
fn fixture(dir: &std::path::Path, name: &str, len: usize) -> std::path::PathBuf {
    let path = dir.join(name);
    let mut file = std::fs::File::create(&path).unwrap_or_else(|e| panic!("create: {e}"));
    let body: Vec<u8> = (0..len)
        .map(|i| u8::try_from(i % 251).unwrap_or(0))
        .collect();
    file.write_all(&body)
        .unwrap_or_else(|e| panic!("write: {e}"));
    path
}

/// A buffer whose first byte is on a sector boundary, which a direct
/// read demands.
struct Aligned {
    raw: Vec<u8>,
    off: usize,
}

impl Aligned {
    fn new(len: usize) -> Aligned {
        let raw = vec![0u8; len + 4096];
        let off = (4096 - raw.as_ptr().addr() % 4096) % 4096;
        Aligned { raw, off }
    }

    fn slice(&mut self, len: usize) -> &mut [u8] {
        let al = self.raw.get_mut(self.off..).unwrap_or_default();
        al.get_mut(..len).unwrap_or_default()
    }
}

/// What the leaf answers about a file is what the safe path answers:
/// the same real location, the same size, the same stamp.
#[test]
fn the_leaf_names_the_file_the_safe_path_names() {
    let dir = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));
    let path = fixture(dir.path(), "window.txt", 64);
    let mut into = Aligned::new(64);
    let asked = Ask {
        path: &path,
        start: 0,
        want: 0,
        direct: false,
    };
    let answer = crate::read(&asked, into.slice(0));

    #[cfg(not(windows))]
    {
        assert_eq!(answer.err(), Some(Refused::Unsupported));
    }
    #[cfg(windows)]
    {
        let window = answer.unwrap_or_else(|e| panic!("leaf: {e}"));
        let real = std::fs::canonicalize(&path).unwrap_or_else(|e| panic!("canonical: {e}"));
        assert_eq!(window.final_path, real, "the leaf names another place");
        assert_eq!(
            window.size,
            std::fs::metadata(&path)
                .unwrap_or_else(|e| panic!("meta: {e}"))
                .len()
        );
        assert_eq!(window.written, 0, "facts alone read no bytes");
    }
}

/// The window is the file's own bytes from the byte the caller asked
/// for, and a request longer than the file answers short rather than
/// failing.
#[test]
fn the_window_is_the_bytes_that_are_there() {
    let dir = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));
    let path = fixture(dir.path(), "window.txt", 40);
    let body = std::fs::read(&path).unwrap_or_else(|e| panic!("read: {e}"));
    let mut into = Aligned::new(8192);
    let asked = Ask {
        path: &path,
        start: 0,
        want: 8192,
        direct: false,
    };
    let answer = crate::read(&asked, into.slice(8192));

    #[cfg(not(windows))]
    {
        assert_eq!(answer.err(), Some(Refused::Unsupported));
    }
    #[cfg(windows)]
    {
        let window = answer.unwrap_or_else(|e| panic!("leaf: {e}"));
        let kept = into
            .raw
            .get(into.off..into.off + window.written)
            .unwrap_or_default();
        assert_eq!(kept, body, "the window is not the file's bytes");
        assert_eq!(window.written, body.len(), "a short file answers short");
    }
}

/// The two facts the cache compares travel together: a file that is
/// written again is answered with a stamp of its own, so an entry keyed
/// on the old one stops matching.
#[test]
fn a_written_file_gets_a_stamp_of_its_own() {
    let dir = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));
    let path = fixture(dir.path(), "window.txt", 8);
    let mut into = Aligned::new(64);
    let ask = |start: u64| Ask {
        path: &path,
        start,
        want: 0,
        direct: false,
    };
    let first = crate::read(&ask(0), into.slice(0));

    #[cfg(not(windows))]
    {
        assert_eq!(first.err(), Some(Refused::Unsupported));
    }
    #[cfg(windows)]
    {
        let first = first.unwrap_or_else(|e| panic!("leaf: {e}"));
        std::thread::sleep(std::time::Duration::from_millis(20));
        fixture(dir.path(), "window.txt", 16);
        let second = crate::read(&ask(0), into.slice(0)).unwrap_or_else(|e| panic!("leaf: {e}"));
        assert_eq!(second.size, 16);
        assert_ne!(
            (first.size, first.modified),
            (second.size, second.modified),
            "a file written again kept the facts of the old one"
        );
    }
}
