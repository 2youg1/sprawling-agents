// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The conventions a run at an address is sent, and how each one is
//! read: through the read bound its building gives, the judgement the
//! `read` tool gives a path (`crates/accounting/Spec.lean` §8-38).
//!
//! The freeze and the layers answer both read here, so what a page says
//! the next run receives is what the next run receives. A prefix is sent
//! to a provider, and a run can make links in its room: reading these
//! files past the read bound would be a second path out of it.

use std::collections::BTreeSet;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use kernel::{Address, AxCode, AxError};

/// What the person reads in a refusal of one convention file.
const ACTION: &str = "read a project's conventions";

/// One `AGENTS.md` a slot may carry: where it is, the heading it is
/// given, and whose conventions the heading says they are.
pub(crate) struct Convention {
    pub(crate) path: PathBuf,
    pub(crate) heading: String,
    pub(crate) whose: &'static str,
}

/// How one convention file reads for the next run.
pub(crate) enum Reading {
    /// Sent, with these bytes.
    Sent(Vec<u8>),
    /// Not there.
    Absent,
    /// There and not sent, for this reason: a directory, or a link the
    /// read bound refuses to follow.
    Withheld(String),
}

/// The words a nested rule arrives under, the same ones the resident
/// slot gives the run's own chain of rooms, so a model reads one grammar
/// whether a rule came with the run or with a file it opened.
const NESTED_WHOSE: &str = "How work is done in this part of the project, from the project itself.";

/// The building slot's conventions: the workspace's copy first and the
/// building's after it, so the one nearest the work is read last.
/// Matched by exact name in the building's own root: an address a reader
/// cannot predict from the rule is an address nobody can check.
pub(crate) fn building_conventions(city_root: &Path, building: &Address) -> [Convention; 2] {
    [
        Convention {
            path: city::city_agents_path(city_root),
            heading: city::AGENTS_FILE.to_owned(),
            whose: "How work is done across the workspace this city formed around, from the \
                    workspace itself.",
        },
        Convention {
            path: city::agents_path(city_root, building),
            heading: format!("{}/{}", building.as_str(), city::AGENTS_FILE),
            whose: "How work is done in this project, from the project itself.",
        },
    ]
}

/// The conventions of every room between the building's root and `addr`,
/// for the end of the resident slot, the one nearest the building first.
///
/// # Errors
/// A level the address grammar refuses, which a parsed `addr` cannot
/// produce.
pub(crate) fn room_conventions(
    city_root: &Path,
    addr: &Address,
    building: &Address,
) -> Result<Vec<Convention>, AxError> {
    Ok(city::room_agents_paths(city_root, building, addr)?
        .into_iter()
        .map(|(room, path)| Convention {
            path,
            heading: format!("{}/{}", room.as_str(), city::AGENTS_FILE),
            whose: "How work is done in this part of the project, from the project itself.",
        })
        .collect())
}

/// The bytes one convention file is sent as: its heading, whose words it
/// is, and its text.
///
/// Written once because two places send these files — the frozen prefix
/// and a note on a read (`crates/runtime/spec/Tools/Read.lean` §8-29-7) —
/// and a heading that moved in one of them would be a second grammar for
/// the same document.
pub(crate) fn convention_block(heading: &str, whose: &str, text: &[u8]) -> Vec<u8> {
    let mut block = format!(
        "## {heading}\n\n{whose} Where this and the building's rules above disagree, \
         the rules decide: they are what the city enforces.\n\n"
    )
    .into_bytes();
    block.extend_from_slice(text);
    block
}

/// The project's rules between `building`'s root and the directory of
/// `at`, for a read that opened a file there: every `AGENTS.md` this run
/// has not been given yet, in reading order, as one note.
///
/// **The frozen prefix cannot do this.** It is assembled once, from the
/// city's and the building's files and the run's own chain of rooms,
/// because a system block that moved mid-run would miss the provider's
/// cached prefix on every request after it. A rule file deeper in the
/// tree is a fact the run meets later, so it travels where a new fact
/// already travels: at the end of the conversation, on the answer that
/// met it.
///
/// **Same reading, same door.** Each file is read by
/// [`read_convention`], the judgement the `read` tool gives a path, so a
/// link out of the building is withheld here exactly as it is withheld
/// from the prefix. A file that is not there, or withheld, is not marked
/// delivered and costs one stat the next time that part is touched.
///
/// **One ceiling for the note**: the same 64 KiB a read's own body is
/// limited to. Whole files are added in order while they fit; the first
/// file of a call that alone exceeds the ceiling is cut with the
/// elision marker and carried, so a rule the model just walked into is
/// never silently dropped. A file that does not fit after another one
/// has is left for a later read.
///
/// # Errors
/// As [`read_convention`] and [`Address::parse`].
pub(crate) fn nested_rules(
    read_root: &Path,
    building: &Address,
    at: &Address,
    delivered: &mut BTreeSet<Address>,
) -> Result<Option<runtime::Note>, AxError> {
    if !at.is_within(building) {
        return Ok(None);
    }
    let Some((dir, _file)) = at.as_str().rsplit_once('/') else {
        return Ok(None);
    };
    let dir = Address::parse(dir)?;
    let ceiling = kernel::consts_policy::INTERVAL_CAP_BYTES;
    let mut text = String::new();
    let mut carried = Vec::new();
    for (room, path) in city::room_agents_paths(read_root, building, &dir)? {
        if delivered.contains(&room) {
            continue;
        }
        let Reading::Sent(bytes) = read_convention(read_root, building, &path)? else {
            continue;
        };
        let mut block = convention_block(
            &format!("{}/{}", room.as_str(), city::AGENTS_FILE),
            NESTED_WHOSE,
            &bytes,
        );
        block.push(b'\n');
        if !text.is_empty() && text.len().saturating_add(block.len()) > ceiling {
            break;
        }
        let mut block = String::from_utf8_lossy(&block).into_owned();
        if block.len() > ceiling {
            block = runtime::elision::splice(
                &block,
                ceiling,
                block.len(),
                runtime::elision::Elided::Tail,
            )
            .text;
        }
        text.push_str(&block);
        carried.push(room);
    }
    if carried.is_empty() {
        return Ok(None);
    }
    for room in &carried {
        delivered.insert(room.clone());
    }
    Ok(Some(runtime::Note {
        delivered: carried,
        text,
    }))
}

/// Reads one convention file for a run of `building`.
///
/// The entry itself is looked at first: not there is `Absent`, and a
/// directory is withheld, where reading it would refuse every run of the
/// room. Anything else is opened through `runtime::BoundReader` with the
/// building as home, so a link that leads out of the city, into a
/// confidential building or into a reserved subtree is withheld with the
/// bound's own reason.
///
/// # Errors
/// `E_STORAGE_FATAL` naming the path for a file that is there and will
/// not open or read: the project wrote those conventions down, and a run
/// sent without them would follow a prompt nobody chose.
pub(crate) fn read_convention(
    city_root: &Path,
    building: &Address,
    path: &Path,
) -> Result<Reading, AxError> {
    match std::fs::symlink_metadata(path) {
        Ok(entry) if entry.is_dir() => {
            return Ok(Reading::Withheld(format!(
                "{} is a directory, not a file",
                city::AGENTS_FILE
            )));
        }
        Ok(_) => {}
        Err(err)
            if matches!(
                err.kind(),
                std::io::ErrorKind::NotFound | std::io::ErrorKind::NotADirectory
            ) =>
        {
            return Ok(Reading::Absent);
        }
        Err(err) => return Err(unread(path, &err)),
    }
    let asked = addressed(city_root, path)?;
    let reader = runtime::BoundReader::new(
        city_root,
        read_bound(city_root, building.clone()),
        &kernel::layout::CityLayout::new(city_root).cas(),
    );
    match reader.open(asked.as_str(), ACTION) {
        Ok(mut opened) => {
            let mut bytes = Vec::new();
            opened
                .read_to_end(&mut bytes)
                .map_err(|err| unread(path, &err))?;
            Ok(Reading::Sent(bytes))
        }
        Err(refused) if matches!(refused.code(), AxCode::GateDenied | AxCode::InvalidArgs) => {
            Ok(Reading::Withheld(refused.subject().to_owned()))
        }
        Err(other) => Err(other),
    }
}

/// What a run of `home` may read by a path: the read bound, closed over
/// the building and this city's rules (`crates/city/Spec.lean` §8-2).
///
/// Another building's rules are read each time a path lands in it, not
/// here, so a run that stays in its own building never pays for them,
/// and a building made confidential after this dispatch is closed from
/// that moment (city D2). What was read stays while the file's stamp
/// holds (city D3).
pub(crate) fn read_bound(city_root: &Path, home: Address) -> runtime::ReadBound {
    let rules = city::RulesCache::new(city_root);
    Arc::new(move |target: &Address| {
        kernel::address::may_read(&home, target, || {
            let holder = city::Building::of(target)?;
            rules.load(holder.addr()).map(|r| r.policy().confidential)
        })
    })
}

/// The city's own spelling of a path inside it.
///
/// The path comes from `city`, which is the authority for where a
/// building's files sit; this only turns it back into the address a
/// reader opens it by, so nothing here repeats that layout.
///
/// # Errors
/// `E_INVALID_ARGS` for a path outside the city root or one the address
/// grammar refuses.
pub(crate) fn addressed(city_root: &Path, path: &Path) -> Result<Address, AxError> {
    let relative = path.strip_prefix(city_root).map_err(|_| {
        AxError::failure(
            AxCode::InvalidArgs,
            "address a prefix source document",
            format!("{} is outside the city", path.display()),
        )
        .with_recovery(
            "move this document under the city root, or drop it from the prefix: an \
             address is relative to the city and cannot leave it",
        )
    })?;
    let spelled = relative
        .to_str()
        .ok_or_else(|| {
            AxError::failure(
                AxCode::InvalidArgs,
                "address a prefix source document",
                format!("{} is not utf-8", relative.display()),
            )
            .with_recovery(
                "rename this document to letters, digits, dashes and slashes: an \
                 address is UTF-8 text",
            )
        })?
        .replace('\\', "/");
    Address::parse(&spelled)
}

/// The refusal for a convention file that is there and does not read.
fn unread(path: &Path, err: &std::io::Error) -> AxError {
    AxError::failure(
        AxCode::StorageFatal,
        ACTION,
        format!("{}: {err}", path.display()),
    )
    .with_recovery(
        "every run here is sent that file; make it readable, or take it away to send the \
             prompt without it",
    )
}
