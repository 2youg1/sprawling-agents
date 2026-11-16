// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

#[cfg(any(windows, target_os = "linux", target_os = "macos"))]
use kernel::B3Hash;
use kernel::{Address, AxError, ReadVerdict};

use super::miss::{self, Floor};
use crate::tools::chosen_path::{self, CityRoot};
#[cfg(any(windows, target_os = "linux", target_os = "macos"))]
use crate::tools::shown::Recall;
use crate::tools::shown::{Answer, ReadShown, Span};
use crate::tools::stamp::Stamp;

#[derive(Clone, Copy)]
pub(super) struct Request<'a> {
    pub(super) city: &'a CityRoot,
    pub(super) addr: &'a Address,
    pub(super) asked: &'a str,
    pub(super) bound: &'a dyn Fn(&Address) -> ReadVerdict,
    #[cfg_attr(
        not(any(windows, target_os = "linux", target_os = "macos", test)),
        expect(
            dead_code,
            reason = "unchanged evidence is queried only by handle fast paths"
        )
    )]
    pub(super) shown: &'a ReadShown,
    #[cfg_attr(
        not(any(windows, target_os = "linux", target_os = "macos", test)),
        expect(
            dead_code,
            reason = "unchanged evidence is queried only by handle fast paths"
        )
    )]
    pub(super) want: Span,
}

pub(super) enum Looked {
    Text {
        text: String,
        before: Stamp,
        after: Stamp,
        addr: Address,
    },
    #[cfg_attr(
        not(any(windows, target_os = "linux", target_os = "macos", test)),
        expect(
            dead_code,
            reason = "unchanged evidence is queried only by handle fast paths"
        )
    )]
    Unchanged {
        stamp: Stamp,
        addr: Address,
        answer: Answer,
    },
}

impl Looked {
    fn text(text: String, before: Stamp, after: Stamp, addr: Address) -> Looked {
        Looked::Text {
            text,
            before,
            after,
            addr,
        }
    }
}

pub(super) fn text_of(request: Request<'_>) -> Result<Looked, AxError> {
    match handle_read(request)? {
        Some(looked) => Ok(looked),
        None => judged_read(request.city, request.addr, request.asked, request.bound),
    }
}

pub(super) fn judged_read(
    city: &CityRoot,
    addr: &Address,
    asked: &str,
    bound: &dyn Fn(&Address) -> ReadVerdict,
) -> Result<Looked, AxError> {
    let judged = chosen_path::land(city, addr, "read", bound)?;
    let text = miss::text_at(
        asked,
        judged.location,
        &Floor::of_address(city.written(), addr),
    )?;
    Ok(Looked::text(text.0, text.1, text.2, judged.addr))
}

#[cfg(windows)]
fn handle_read(request: Request<'_>) -> Result<Option<Looked>, AxError> {
    let Ok(file) = std::fs::File::open(request.city.join(request.addr)) else {
        return Ok(None);
    };
    let Ok(real) = winx::file::get_file_path(&file) else {
        return Ok(None);
    };
    let landed = request
        .city
        .judged_location(request.addr, &real, "read", request.bound)?;
    Ok(read_handle(file, landed, request))
}

#[cfg(target_os = "linux")]
fn handle_read(request: Request<'_>) -> Result<Option<Looked>, AxError> {
    use std::os::fd::AsRawFd as _;

    let Ok(real) = std::fs::canonicalize(request.city.join(request.addr)) else {
        return Ok(None);
    };
    let landed = request
        .city
        .judged_location(request.addr, &real, "read", request.bound)?;
    let Ok(file) = std::fs::File::open(&real) else {
        return Ok(None);
    };
    let fd = std::path::Path::new("/proc/self/fd").join(file.as_raw_fd().to_string());
    match std::fs::read_link(fd) {
        Ok(now) if now == real => Ok(read_handle(file, landed, request)),

        Ok(_) | Err(_) => Ok(None),
    }
}

#[cfg(target_os = "macos")]
fn handle_read(request: Request<'_>) -> Result<Option<Looked>, AxError> {
    use std::os::unix::{ffi::OsStrExt as _, fs::MetadataExt as _};

    let Ok(real) = std::fs::canonicalize(request.city.join(request.addr)) else {
        return Ok(None);
    };
    request
        .city
        .judged_location(request.addr, &real, "read", request.bound)?;
    let Ok(file) = std::fs::File::open(&real) else {
        return Ok(None);
    };
    let (Ok(held), Ok(named)) = (file.metadata(), std::fs::metadata(&real)) else {
        return Ok(None);
    };
    if (held.dev(), held.ino()) != (named.dev(), named.ino()) {
        return Ok(None);
    }
    let actual = rustix::fs::getpath(&file).map_err(|err| {
        kernel::AxError::failure(kernel::AxCode::GateDenied, "read", err.to_string())
            .with_recovery("retry after the file location can be verified")
    })?;
    let actual = std::path::Path::new(std::ffi::OsStr::from_bytes(actual.as_bytes()));
    let landed = request
        .city
        .judged_location(request.addr, actual, "read", request.bound)?;
    Ok(read_handle(file, landed, request))
}

#[cfg(not(any(windows, target_os = "linux", target_os = "macos")))]
fn handle_read(_request: Request<'_>) -> Result<Option<Looked>, AxError> {
    Ok(None)
}

#[cfg(any(windows, target_os = "linux", target_os = "macos"))]
fn read_handle(file: std::fs::File, addr: Address, request: Request<'_>) -> Option<Looked> {
    let facts = Stamp::facts(&file).ok()?;
    if !facts.is_file() {
        return None;
    }
    let before = facts.stamp().clone();
    match request
        .shown
        .recall(request.addr, &before, request.asked, request.want)
    {
        Recall::Fresh => read_text(file, facts.len(), before)
            .map(|(text, before, after)| Looked::text(text, before, after, addr)),
        Recall::Repeat(answer) => Some(Looked::Unchanged {
            stamp: before,
            addr,
            answer,
        }),
        Recall::Compare { digest, answer } => {
            read_text(file, facts.len(), before).map(|(text, before, after)| {
                if before == after && B3Hash::digest(text.as_bytes()) == digest {
                    Looked::Unchanged {
                        stamp: after,
                        addr,
                        answer,
                    }
                } else {
                    Looked::text(text, before, after, addr)
                }
            })
        }
    }
}

#[cfg(any(windows, target_os = "linux", target_os = "macos"))]
fn read_text(mut file: std::fs::File, len: u64, before: Stamp) -> Option<(String, Stamp, Stamp)> {
    use std::io::Read as _;

    let capacity = usize::try_from(len).ok()?.checked_add(1)?;
    let mut bytes = Vec::with_capacity(capacity);
    file.read_to_end(&mut bytes).ok()?;
    let after = Stamp::of_file(&file).ok()?;
    Some((String::from_utf8(bytes).ok()?, before, after))
}
