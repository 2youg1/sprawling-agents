// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The release archive as the install action meets it: the digest
//! published with it, the unpack, and the executable that comes out
//! (citysim D3, `tools/citysim/spec/BenchStartup.lean` §8-5).
//!
//! Shape: adapter. `sha2` and `zip` are the release's own spelling of
//! "the digest" and "the archive format", and each is named once in the
//! workspace manifest, so the step `cargo xtask package` writes and the
//! step this reading prices cannot come to disagree about either.
//!
//! The digest is not a second content hash beside the city's: it
//! addresses an artifact of the outside world, which is what
//! `install.sh` and `install.ps1` verify before they unpack anything.

use std::io::Write;
use std::path::{Path, PathBuf};

use kernel::{AxCode, AxError};
use sha2::{Digest, Sha256};

use super::footprint;

/// The archive's SHA-256, read from disk the way the install scripts
/// hash it (`sha256sum`, `Get-FileHash`).
pub(super) fn digest_of(archive: &Path) -> Result<Vec<u8>, AxError> {
    let bytes = std::fs::read(archive).map_err(|err| {
        AxError::failure(
            AxCode::StorageFatal,
            "read the archive",
            format!("{}: {err}", archive.display()),
        )
        .with_recovery("run just bench-startup, which places the archive before measuring")
    })?;
    Ok(Sha256::digest(&bytes).to_vec())
}

/// What the install path answers when the bytes and the published digest
/// disagree. The archive is never unpacked after this.
pub(super) fn digest_mismatch(archive: &Path) -> AxError {
    AxError::failure(
        AxCode::DigestSuspect,
        "verify the archive digest",
        format!(
            "{}: the bytes do not match the digest published with them",
            archive.display()
        ),
    )
    .with_recovery("nothing is installed from an archive that fails its digest; fetch it again")
}

/// Unpacks the archive into `into` and answers the executable it holds,
/// found by the name a shell would type.
pub(super) fn unpack(archive: &Path, into: &Path) -> Result<PathBuf, AxError> {
    let file = std::fs::File::open(archive).map_err(|err| {
        AxError::failure(
            AxCode::StorageFatal,
            "open the archive",
            format!("{}: {err}", archive.display()),
        )
        .with_recovery("run just bench-startup, which places the archive before measuring")
    })?;
    let mut zip = zip::read::ZipArchive::new(file).map_err(|err| {
        AxError::failure(
            AxCode::StorageFatal,
            "read the archive",
            format!("{}: {err}", archive.display()),
        )
        .with_recovery("the archive is corrupt; run just bench-startup to place it again")
    })?;
    zip.extract(into).map_err(|err| {
        AxError::failure(
            AxCode::StorageFatal,
            "unpack the archive",
            format!("{}: {err}", archive.display()),
        )
        .with_recovery("free space under the scratch directory and run again")
    })?;
    let stem = crate::executable_name();
    footprint::find_named(into, &stem)?.ok_or_else(|| {
        AxError::failure(
            AxCode::PathNotFound,
            "find the installed binary",
            format!("no file named {stem} came out of {}", archive.display()),
        )
        .with_recovery("the archive holds no executable under the name a shell would type")
    })
}

/// The fixture the install action starts from: the shipped binary as one
/// deflated zip entry under one directory, which is the shape
/// `cargo xtask package` gives a release archive.
///
/// Written before any stamp: the install reading starts at "the archive is in
/// place", and network transfer is out of the reading. Reading a release
/// archive and writing this fixture are the same file's business, so the
/// member a shell would find and the member this fixture writes cannot
/// come apart.
///
/// The timestamp is the zip format's own epoch rather than the clock: a
/// fixture that carried the wall clock would make two runs of one binary
/// differ in the bytes this action hashes.
pub(crate) fn fixture(scratch: &Path, binary: &Path) -> Result<PathBuf, AxError> {
    let bytes = std::fs::read(binary).map_err(|err| {
        AxError::failure(
            AxCode::StorageFatal,
            "read the shipped binary",
            format!("{}: {err}", binary.display()),
        )
        .with_recovery("run just bench-startup, which builds the binary it measures first")
    })?;
    let archive = scratch.join("archive.zip");
    let member = format!("sprawling/{}", crate::executable_name());
    let file = std::fs::File::create(&archive).map_err(|err| {
        AxError::failure(
            AxCode::StorageFatal,
            "create the fixture archive",
            format!("{}: {err}", archive.display()),
        )
        .with_recovery("free space on the temporary drive and run again")
    })?;
    let mut zip = zip::ZipWriter::new(file);
    let options = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated)
        .last_modified_time(zip::DateTime::default());
    zip.start_file(member, options)
        .map_err(|err| fixture_failed(&archive, err))?;
    zip.write_all(&bytes)
        .map_err(|err| fixture_failed(&archive, err))?;
    zip.finish().map_err(|err| fixture_failed(&archive, err))?;
    Ok(archive)
}

fn fixture_failed(archive: &Path, err: impl std::fmt::Display) -> AxError {
    AxError::failure(
        AxCode::StorageFatal,
        "write the fixture archive",
        format!("{}: {err}", archive.display()),
    )
    .with_recovery("the zip fixture is a defect in bench_startup if the disk is not full")
}
