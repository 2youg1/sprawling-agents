// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::io;
use std::path::{Path, PathBuf};

pub(crate) trait Vfs: Send {
    fn create_dir_all(&mut self, dir: &Path) -> io::Result<()>;

    fn list(&self, dir: &Path) -> io::Result<Vec<PathBuf>>;

    fn list_dirs(&self, dir: &Path) -> io::Result<Vec<PathBuf>>;
    fn read(&self, path: &Path) -> io::Result<Vec<u8>>;

    fn size(&self, path: &Path) -> io::Result<u64>;

    fn read_at(&self, path: &Path, offset: u64, len: u64) -> io::Result<Vec<u8>>;

    fn append(&mut self, path: &Path, bytes: &[u8]) -> io::Result<()>;

    fn write_at(&mut self, path: &Path, offset: u64, bytes: &[u8]) -> io::Result<()>;

    fn truncate(&mut self, path: &Path, len: u64) -> io::Result<()>;
    fn sync_data(&mut self, path: &Path) -> io::Result<()>;

    fn rename(&mut self, from: &Path, to: &Path) -> io::Result<()>;

    fn sync_dir(&mut self, dir: &Path) -> io::Result<()>;
    fn remove_file(&mut self, path: &Path) -> io::Result<()>;

    fn copy_permissions(&mut self, from: &Path, to: &Path) -> io::Result<()>;

    fn make_readonly(&mut self, _path: &Path) -> io::Result<()> {
        Ok(())
    }
    fn exists(&self, path: &Path) -> bool;
}
