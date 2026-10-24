// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The documents whose default text this build carries, and whether a
//! copy in the city is the person's own or a default an earlier build
//! laid down.
//!
//! A city keeps a copy of each of these documents so a person has a
//! file to open and change, and the copy on disk is the exact bytes the
//! model reads. An earlier build's default that nobody changed is moved
//! forward to this build's default when the city opens; a copy somebody
//! changed is left as it is and named, so the person learns that their
//! version no longer tracks the binary.
//!
//! Specified by `crates/city/spec/SpineFiles.lean` §8-42 (city D28).

use std::path::{Path, PathBuf};

use kernel::{AxCode, AxError, B3Hash};

use crate::governed::Governed;

/// A document whose default text this build carries.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Shipped {
    /// The city's own instructions, at the city root.
    City,
    /// Who the Mayor is, in the reserved subtree.
    Mayor,
    /// Who the clerk is, in the reserved subtree.
    Clerk,
}

impl Shipped {
    /// Every document this module keeps, in the order `follow_defaults`
    /// visits them.
    pub const ALL: [Shipped; 3] = [Shipped::City, Shipped::Mayor, Shipped::Clerk];

    /// This build's default text.
    #[must_use]
    pub fn text(self) -> &'static str {
        match self {
            Shipped::City => crate::spine_files::CITY_TEMPLATE,
            Shipped::Mayor => crate::spine_files::hall::MAYOR_TEMPLATE,
            Shipped::Clerk => crate::spine_files::hall::CLERK_TEMPLATE,
        }
    }

    /// Where the city keeps its copy.
    #[must_use]
    pub fn path(self, city_root: &Path) -> PathBuf {
        match self {
            Shipped::City => city_root.join(crate::spine_files::CITY_FILE),
            Shipped::Mayor => Governed::Mayor.path(city_root),
            Shipped::Clerk => Governed::Clerk.path(city_root),
        }
    }

    /// The BLAKE3 of every earlier default of this document, as lowercase
    /// hex: each version the repository's history holds under
    /// `crates/city/templates/` and, before that, `docs/City.md` and
    /// `docs/templates/`.
    ///
    /// A change to a default adds the digest of the text it replaces
    /// here, in the same commit; a default missing from this list is
    /// judged edited, and every city that holds it keeps the old words.
    fn earlier(self) -> &'static [&'static str] {
        match self {
            Shipped::City => EARLIER_CITY,
            Shipped::Mayor => EARLIER_MAYOR,
            Shipped::Clerk => EARLIER_CLERK,
        }
    }
}

const EARLIER_CITY: &[&str] = &[
    "2d54b834bccff77e28aac8a2e7abe8771ca98493beb6f2c3a534f5749d671984",
    "35db44e818760f17dacd2e08adc49c5496aa8a8bc36404dde19ee7c5cda4739b",
    "46513cb4abd937d4630b0ed1d778c0cef976c8e04586607c64e99f24c051793f",
    "528f018f465e7cc153a8f047934aa9298bc910f375e9ee2f7ac4ea3aa2ac24e1",
    "703c855b9a71fef0116204466e90d06b92415245a182563a4f93b0e4ebd04115",
    "868c7c3df7596cdf7edcfb6fa2514b5df484969d8d814c9160749bddbe8aa269",
    "87d6cd1280345a4e23655041641f738c8adc3e6fd562d7a0586089e0ca979212",
    "a718a4eaab8cde1411f7742c0b75f351dd7c9ba7bde40fa8ef1988a27426e892",
    "ae62c34b67fa94876522064da10b9958901bc62180cb254b374991b6f0d6ec03",
    "bf1f1811136e52e5566585fe6b6b9af9144feb58bc9d6ae8cbc89b3352fc7d4f",
    "d8585c605b06379a0f32199fd8e724b42e78647caf0f5fa86198a48e23518730",
    "e9e26276182c5efe4688ba80a8e2c9df0899e2fc97c9f3cbbad7c1be570933eb",
    "fc76b10d85a4a6ebac674e96aff07a351cacd210843c47f6eed23bc566ca4d76",
];

const EARLIER_MAYOR: &[&str] = &[
    "1604f5928fb4e5242f35d0f1a4ef75954de734d48cb06513e557914254f16c4c",
    "4740013d950965e13fce8331ddb052b875163d02fd9382f606a7e92d889e7221",
    "a1575724ede60b0f523ac642a75a35523ab194ac8020863079c046ec1499c545",
    "e6b9a3b0b06b02303e9d2b1348c627b22bf7ef7b821c54af0701c8ac1ad31fb1",
    "eb0e044f83d340a3fc7814011f5bac71c6f553caf128427ebbd557bef3cd8cf7",
    "fe5edebae9ed0250f52a06c0a7926bc68c14c18da91d74d3db0c5499ad63022d",
];

const EARLIER_CLERK: &[&str] = &[
    "4f7d37cd9a0004d681667f692165aa11c4a048e0cc88e6dada6a4424cc6fc0e5",
    "5fe21e3a891ccaa9171e1f9e25a6200f51d1cf8e22a352fcfa2b74dffedbe2b4",
    "6677ebe02ea85f282ebc9ef39307e4da4daa3dd4666005a9ad76ca477ed58ba9",
    "bf0f79d5ec94f3f0770682e394642007dcaf427dd402893c4684c68efd6fd63b",
    "cbc391e252d67022115ae43e707d7b5d4ccf32051641c38a93810454296ec62d",
    "cf7b27efb42b24cd3a5f10872062b02bf9936fe7514b8012ea36601953e4aacc",
    "dd9ee213d03caa843e8b408ebb653726f31d45d0cb22e28fab8a2bf0a6a9292f",
];

/// What a copy in the city is, measured against this build's default.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CopyStanding {
    /// No copy on disk.
    Absent,
    /// The copy is this build's default.
    Current,
    /// The copy is an earlier build's default that nobody changed.
    Earlier,
    /// Somebody changed the copy; it is theirs.
    Edited,
}

/// Judges one copy against this build's default.
///
/// The body is judged and an identity area above it is not: the
/// identity cards write the names a person gives into the top of
/// `MAYOR.md` (`crates/city/spec/Identity.lean` §8-33), and naming the
/// Mayor is not editing who the Mayor is.
#[must_use]
pub fn standing(doc: Shipped, copy: Option<&[u8]>) -> CopyStanding {
    let Some(copy) = copy else {
        return CopyStanding::Absent;
    };
    let body = crate::identity::persona(copy.to_vec());
    if body == doc.text().as_bytes() {
        return CopyStanding::Current;
    }
    let digest = B3Hash::digest(&body).to_string();
    if doc.earlier().contains(&digest.as_str()) {
        CopyStanding::Earlier
    } else {
        CopyStanding::Edited
    }
}

/// A copy somebody changed, which `follow_defaults` leaves as it is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Edited {
    pub doc: Shipped,
    pub path: PathBuf,
}

/// Lays down every copy that is missing, moves every earlier default
/// nobody changed forward to this build's, and answers which copies
/// somebody changed.
///
/// The only writer of these three documents' defaults: forming a city
/// and opening one both come here, so a city formed today and one formed
/// by an earlier build hold the same bytes for every document nobody
/// changed. A copy with an identity area keeps its area; only the body
/// under it moves.
///
/// # Errors
/// `E_STORAGE_FATAL` naming the path, for a copy that is there and will
/// not read, and for one that will not write.
pub fn follow_defaults(city_root: &Path) -> Result<Vec<Edited>, AxError> {
    let mut edited = Vec::new();
    for doc in Shipped::ALL {
        let path = doc.path(city_root);
        let copy = read_copy(&path)?;
        match standing(doc, copy.as_deref()) {
            CopyStanding::Current => {}
            CopyStanding::Edited => edited.push(Edited { doc, path }),
            CopyStanding::Absent | CopyStanding::Earlier => {
                crate::document::replace(&path, &moved_forward(doc, copy.as_deref()))?;
            }
        }
    }
    Ok(edited)
}

/// The copy's identity area, if it has one, over this build's default.
fn moved_forward(doc: Shipped, copy: Option<&[u8]>) -> Vec<u8> {
    let area = copy
        .and_then(|copy| {
            let body = crate::identity::persona(copy.to_vec());
            copy.get(..copy.len().saturating_sub(body.len()))
        })
        .unwrap_or_default();
    let mut out = area.to_vec();
    out.extend_from_slice(doc.text().as_bytes());
    out
}

/// The copy on disk, or `None` when there is none.
///
/// # Errors
/// `E_STORAGE_FATAL` naming the path for every failure except a file
/// that is not there.
fn read_copy(path: &Path) -> Result<Option<Vec<u8>>, AxError> {
    match std::fs::read(path) {
        Ok(bytes) => Ok(Some(bytes)),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(err) => Err(AxError::failure(
            AxCode::StorageFatal,
            "read a document whose default this build carries",
            format!("{}: {err}", path.display()),
        )
        .with_recovery(
            "make the file readable, or remove it so the city lays down this build's \
             default, then open the city again",
        )),
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
mod tests;
