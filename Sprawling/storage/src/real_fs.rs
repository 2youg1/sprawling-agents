// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::io;
use std::io::{Read as _, Seek as _, SeekFrom};
use std::path::{Path, PathBuf};

use crate::vfs::Vfs;

pub(crate) struct RealFs {
    open: Option<HeldWriter>,
}

struct HeldWriter {
    path: PathBuf,
    file: std::fs::File,
    mode: WriteMode,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum WriteMode {
    Append,
    Positional,
}

impl RealFs {
    pub(crate) fn new() -> RealFs {
        RealFs { open: None }
    }

    fn writer(&mut self, path: &Path, mode: WriteMode) -> io::Result<&mut std::fs::File> {
        if self
            .open
            .as_ref()
            .is_none_or(|held| held.path != path || held.mode != mode)
        {
            let mut options = std::fs::OpenOptions::new();
            match mode {
                WriteMode::Append => options.create(true).append(true),
                WriteMode::Positional => options.create(true).write(true),
            };
            write_through(&mut options);
            let file = options.open(path)?;
            self.open = Some(HeldWriter {
                path: path.to_path_buf(),
                file,
                mode,
            });
        }
        match self.open.as_mut() {
            Some(held) => Ok(&mut held.file),

            None => Err(io::Error::other(
                "the segment handle vanished after opening it",
            )),
        }
    }

    fn release(&mut self, path: &Path) {
        if self.open.as_ref().is_some_and(|held| held.path == path) {
            self.open = None;
        }
    }
}

impl Vfs for RealFs {
    fn create_dir_all(&mut self, dir: &Path) -> io::Result<()> {
        std::fs::create_dir_all(dir)
    }

    fn list(&self, dir: &Path) -> io::Result<Vec<PathBuf>> {
        let mut files = Vec::new();
        for entry in std::fs::read_dir(dir)? {
            let path = entry?.path();
            if path.is_file() {
                files.push(path);
            }
        }
        files.sort();
        Ok(files)
    }

    fn list_dirs(&self, dir: &Path) -> io::Result<Vec<PathBuf>> {
        let mut dirs = Vec::new();
        for entry in std::fs::read_dir(dir)? {
            let path = entry?.path();
            if path.is_dir() {
                dirs.push(path);
            }
        }
        dirs.sort();
        Ok(dirs)
    }

    fn read(&self, path: &Path) -> io::Result<Vec<u8>> {
        std::fs::read(path)
    }

    fn size(&self, path: &Path) -> io::Result<u64> {
        std::fs::metadata(path).map(|meta| meta.len())
    }

    fn read_at(&self, path: &Path, offset: u64, len: u64) -> io::Result<Vec<u8>> {
        let mut file = std::fs::File::open(path)?;
        file.seek(SeekFrom::Start(offset))?;

        let mut bytes = Vec::new();
        file.take(len).read_to_end(&mut bytes)?;
        Ok(bytes)
    }

    fn append(&mut self, path: &Path, bytes: &[u8]) -> io::Result<()> {
        use std::io::Write as _;
        self.writer(path, WriteMode::Append)?.write_all(bytes)
    }

    fn write_at(&mut self, path: &Path, offset: u64, bytes: &[u8]) -> io::Result<()> {
        use std::io::Write as _;
        let file = self.writer(path, WriteMode::Positional)?;
        file.seek(SeekFrom::Start(offset))?;
        file.write_all(bytes)
    }

    fn truncate(&mut self, path: &Path, len: u64) -> io::Result<()> {
        self.release(path);
        let file = std::fs::OpenOptions::new().write(true).open(path)?;
        file.set_len(len)
    }

    fn sync_data(&mut self, path: &Path) -> io::Result<()> {
        let mode = match self.open.as_ref() {
            Some(held) if held.path == path => held.mode,
            Some(_) | None => WriteMode::Append,
        };
        self.writer(path, mode)?.sync_data()
    }

    fn rename(&mut self, from: &Path, to: &Path) -> io::Result<()> {
        self.release(from);
        self.release(to);
        match std::fs::rename(from, to) {
            #[cfg(windows)]
            Err(refused) if refused.kind() == io::ErrorKind::PermissionDenied => {
                clear_read_only(to, refused)?;
                std::fs::rename(from, to)
            }
            done => done,
        }
    }

    #[cfg(windows)]
    fn sync_dir(&mut self, _dir: &Path) -> io::Result<()> {
        Ok(())
    }

    #[cfg(not(windows))]
    fn sync_dir(&mut self, dir: &Path) -> io::Result<()> {
        std::fs::File::open(dir)?.sync_all()
    }

    fn remove_file(&mut self, path: &Path) -> io::Result<()> {
        self.release(path);
        std::fs::remove_file(path)
    }

    fn copy_permissions(&mut self, from: &Path, to: &Path) -> io::Result<()> {
        self.release(to);
        std::fs::set_permissions(to, std::fs::metadata(from)?.permissions())
    }

    fn make_readonly(&mut self, path: &Path) -> io::Result<()> {
        self.release(path);
        let mut permissions = std::fs::metadata(path)?.permissions();
        permissions.set_readonly(true);
        std::fs::set_permissions(path, permissions)
    }

    fn exists(&self, path: &Path) -> bool {
        path.is_file()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SegmentDurability {
    SyncData,

    #[expect(
        dead_code,
        reason = "a candidate arm; the release-build reading chooses between the arms"
    )]
    WriteThrough,
}

#[cfg(windows)]
pub(crate) const SEGMENT_DURABILITY: SegmentDurability = SegmentDurability::SyncData;

#[cfg(target_os = "linux")]
pub(crate) const SEGMENT_DURABILITY: SegmentDurability = SegmentDurability::SyncData;

#[cfg(not(any(windows, target_os = "linux")))]
pub(crate) const SEGMENT_DURABILITY: SegmentDurability = SegmentDurability::SyncData;

#[cfg(windows)]
const FILE_FLAG_WRITE_THROUGH: u32 = 0x8000_0000;

#[cfg(all(
    target_os = "linux",
    any(
        target_arch = "x86_64",
        target_arch = "x86",
        target_arch = "aarch64",
        target_arch = "arm",
        target_arch = "riscv64"
    )
))]
const O_DSYNC: i32 = 0o10000;

#[cfg(windows)]
fn write_through(options: &mut std::fs::OpenOptions) {
    use std::os::windows::fs::OpenOptionsExt as _;
    match SEGMENT_DURABILITY {
        SegmentDurability::SyncData => {}
        SegmentDurability::WriteThrough => {
            options.custom_flags(FILE_FLAG_WRITE_THROUGH);
        }
    }
}

#[cfg(all(
    target_os = "linux",
    any(
        target_arch = "x86_64",
        target_arch = "x86",
        target_arch = "aarch64",
        target_arch = "arm",
        target_arch = "riscv64"
    )
))]
fn write_through(options: &mut std::fs::OpenOptions) {
    use std::os::unix::fs::OpenOptionsExt as _;
    match SEGMENT_DURABILITY {
        SegmentDurability::SyncData => {}
        SegmentDurability::WriteThrough => {
            options.custom_flags(O_DSYNC);
        }
    }
}

#[cfg(not(any(
    windows,
    all(
        target_os = "linux",
        any(
            target_arch = "x86_64",
            target_arch = "x86",
            target_arch = "aarch64",
            target_arch = "arm",
            target_arch = "riscv64"
        )
    )
)))]
fn write_through(_options: &mut std::fs::OpenOptions) {
    const {
        assert!(
            matches!(SEGMENT_DURABILITY, SegmentDurability::SyncData),
            "this platform has no write-through arm"
        );
    }
}

#[cfg(windows)]
fn clear_read_only(to: &Path, refused: io::Error) -> io::Result<()> {
    let mut bits = match std::fs::metadata(to) {
        Ok(held) if held.permissions().readonly() => held.permissions(),
        Ok(_) | Err(_) => return Err(refused),
    };
    #[expect(
        clippy::permissions_set_readonly_false,
        reason = "Windows only: this clears the read-only attribute and grants nothing else"
    )]
    bits.set_readonly(false);
    std::fs::set_permissions(to, bits)
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
    use std::fs;

    #[test]
    fn a_handle_kept_across_a_rename_or_a_removal_would_write_into_the_wrong_file() {
        let dir = tempfile::tempdir().unwrap();
        let mut fs = RealFs::new();
        let half_written = dir.path().join("half-written");
        let named = dir.path().join("named");

        fs.append(&half_written, b"first\n").unwrap();
        fs.sync_data(&half_written).unwrap();
        fs.rename(&half_written, &named).unwrap();

        fs.append(&half_written, b"second\n").unwrap();
        fs.sync_data(&half_written).unwrap();
        assert_eq!(
            fs::read(&named).unwrap(),
            b"first\n",
            "the renamed file is closed"
        );
        assert_eq!(fs::read(&half_written).unwrap(), b"second\n");

        fs.remove_file(&named).unwrap();
        fs.append(&named, b"third\n").unwrap();
        fs.sync_data(&named).unwrap();
        assert_eq!(fs::read(&named).unwrap(), b"third\n");

        fs.truncate(&named, 2).unwrap();
        fs.append(&named, b"ird\n").unwrap();
        assert_eq!(fs::read(&named).unwrap(), b"third\n");
    }

    #[test]
    fn the_handle_follows_the_file_when_a_wave_rolls_to_a_new_segment() {
        let dir = tempfile::tempdir().unwrap();
        let mut fs = RealFs::new();
        let first = dir.path().join("first");
        let second = dir.path().join("second");
        fs.append(&first, b"a").unwrap();
        fs.append(&second, b"b").unwrap();
        fs.append(&first, b"c").unwrap();
        fs.sync_data(&first).unwrap();
        fs.sync_data(&second).unwrap();
        assert_eq!(fs::read(&first).unwrap(), b"ac");
        assert_eq!(fs::read(&second).unwrap(), b"b");
    }
}
