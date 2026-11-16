// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::collections::{BTreeMap, BTreeSet};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use kernel::{Address, AxCode, AxError};

const ACTION: &str = "read a project's conventions";

pub(crate) struct Convention {
    pub(crate) path: PathBuf,
    pub(crate) heading: String,
    pub(crate) whose: &'static str,
}

pub(crate) enum Reading {

    Sent(Vec<u8>),

    Absent,

    Withheld(String),
}

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

pub(crate) fn read_convention(
    city_root: &Path,
    building: &Address,
    path: &Path,
) -> Result<Reading, AxError> {
    match open_convention(city_root, read_bound(city_root, building.clone()), path)? {
        ConventionOpen::Opened(mut opened) => {
            let mut bytes = Vec::new();
            opened
                .read_to_end(&mut bytes)
                .map_err(|err| unread(path, &err))?;
            Ok(Reading::Sent(bytes))
        }
        ConventionOpen::Absent => Ok(Reading::Absent),
        ConventionOpen::Withheld(reason) => Ok(Reading::Withheld(reason)),
    }
}

enum ConventionOpen {
    Opened(runtime::Opened),
    Absent,
    Withheld(String),
}

fn open_convention(
    city_root: &Path,
    bound: runtime::ReadBound,
    path: &Path,
) -> Result<ConventionOpen, AxError> {
    match std::fs::symlink_metadata(path) {
        Ok(entry) if entry.is_dir() => {
            return Ok(ConventionOpen::Withheld(format!(
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
            return Ok(ConventionOpen::Absent);
        }
        Err(err) => return Err(unread(path, &err)),
    }
    let asked = addressed(city_root, path)?;
    let reader = runtime::BoundReader::new(
        city_root,
        bound,
        &kernel::layout::CityLayout::new(city_root).cas(),
    );
    match reader.open(asked.as_str(), ACTION) {
        Ok(opened) => Ok(ConventionOpen::Opened(opened)),
        Err(refused) if matches!(refused.code(), AxCode::GateDenied | AxCode::InvalidArgs) => {
            Ok(ConventionOpen::Withheld(refused.subject().to_owned()))
        }
        Err(other) => Err(other),
    }
}

fn read_nested(
    scope: &RuleScope,
    path: &Path,
    keep: usize,
    halted: &mut dyn FnMut() -> bool,
) -> Option<(String, u64)> {
    let mut opened = match open_convention(&scope.city_root, Arc::clone(&scope.bound), path) {
        Ok(ConventionOpen::Opened(opened)) => opened,
        Ok(ConventionOpen::Absent | ConventionOpen::Withheld(_)) | Err(_) => return None,
    };
    let capacity = opened
        .byte_len()
        .ok()
        .and_then(|len| usize::try_from(len).ok())
        .map_or(keep, |len| len.min(keep));
    let mut text = Vec::with_capacity(capacity);
    let mut pending = Vec::new();
    let mut total = 0u64;
    let mut buffer = [0u8; 8_192];
    loop {
        if halted() {
            return None;
        }
        let read = match opened.read(&mut buffer) {
            Ok(read) => read,
            Err(_) => return None,
        };
        if read == 0 {
            break;
        }
        total = total.saturating_add(u64::try_from(read).unwrap_or(u64::MAX));
        let retained = keep.saturating_sub(text.len()).min(read);
        text.extend_from_slice(buffer.get(..retained)?);
        pending.extend_from_slice(buffer.get(..read)?);
        match std::str::from_utf8(&pending) {
            Ok(_) => pending.clear(),
            Err(err) if err.error_len().is_some() => return None,
            Err(err) => {
                let valid = err.valid_up_to();
                pending.drain(..valid);
            }
        }
    }
    if halted() || !pending.is_empty() {
        return None;
    }

    let boundary = std::str::from_utf8(&text).map_or_else(|err| err.valid_up_to(), |_| text.len());
    text.truncate(boundary);
    let dropped = total.saturating_sub(u64::try_from(text.len()).unwrap_or(u64::MAX));
    String::from_utf8(text).ok().map(|text| (text, dropped))
}

pub(crate) fn read_bound(city_root: &Path, home: Address) -> runtime::ReadBound {
    let rules = city::RulesCache::new(city_root);
    Arc::new(move |target: &Address| {
        kernel::address::may_read(&home, target, || {
            let holder = city::Building::of(target)?;
            rules.load(holder.addr()).map(|r| r.policy().confidential)
        })
    })
}

#[derive(Clone)]
pub(crate) struct RuleScope {
    pub(crate) city_root: PathBuf,
    pub(crate) building: Address,
    pub(crate) room: Address,
    pub(crate) bound: runtime::ReadBound,
}

pub(crate) fn nested_rules(
    scope: RuleScope,
    context: &runtime::RuleContext,
    skip: &BTreeSet<Address>,
    halted: &mut dyn FnMut() -> bool,
) -> Vec<runtime::RuleCandidate> {
    let runtime::RuleContext { admitted, landed } = context;
    if !admitted.is_within(&scope.building) || !landed.is_within(&scope.building) {
        return Vec::new();
    }
    let Some((directory, _)) = admitted.as_str().rsplit_once('/') else {
        return Vec::new();
    };
    let Ok(directory) = Address::parse(directory) else {
        return Vec::new();
    };
    if directory == scope.room || !directory.is_within(&scope.room) {
        return Vec::new();
    }
    let Ok(paths) = city::room_agents_paths(&scope.city_root, &scope.building, &directory) else {
        return Vec::new();
    };
    let mut candidates = Vec::new();
    let mut remaining = kernel::consts_policy::INTERVAL_CAP_BYTES;
    for (room, path) in paths.into_iter().filter(|(room, _)| {
        room != &scope.room && room.is_within(&scope.room) && !skip.contains(room)
    }) {
        if halted() {
            return Vec::new();
        }
        if remaining == 0 {
            break;
        }
        let Some((text, dropped)) = read_nested(&scope, &path, remaining, halted) else {
            continue;
        };
        remaining = remaining.saturating_sub(text.len());
        candidates.push(runtime::RuleCandidate {
            room,
            text,
            dropped,
        });
        if dropped > 0 {
            break;
        }
    }
    if halted() {
        return Vec::new();
    }
    candidates
}

#[derive(Default)]
pub(crate) struct RuleDelivery {
    pub(crate) delivered: BTreeSet<Address>,
    pending: BTreeMap<String, BTreeSet<Address>>,
}

impl RuleDelivery {

    pub(crate) fn seen(&self) -> BTreeSet<Address> {
        self.delivered
            .iter()
            .chain(self.pending.values().flatten())
            .cloned()
            .collect()
    }

    pub(crate) fn answer(
        &mut self,
        call: &str,
        candidates: Vec<runtime::RuleCandidate>,
    ) -> Vec<runtime::RuleCandidate> {
        let seen = self.seen();
        let cap = kernel::consts_policy::INTERVAL_CAP_BYTES;
        let mut used = 0usize;
        let mut selected = Vec::new();
        let mut seen = seen;
        for candidate in candidates {
            if seen.contains(&candidate.room) {
                continue;
            }
            let overhead = runtime::rendering::rule_overhead(candidate.room.as_str());
            let remaining = cap.saturating_sub(used).saturating_sub(overhead);
            if cap.saturating_sub(used) < overhead {
                break;
            }
            let original = candidate
                .text
                .len()
                .saturating_add(usize::try_from(candidate.dropped).unwrap_or(usize::MAX));
            let whole = candidate.dropped == 0 && candidate.text.len() <= remaining;
            if whole {
                used = used
                    .saturating_add(candidate.text.len())
                    .saturating_add(overhead);
                seen.insert(candidate.room.clone());
                selected.push(candidate);
                continue;
            }
            let room = runtime::elision::marker_room(original);
            if remaining < room {
                break;
            }
            let front = remaining.saturating_sub(room);
            let boundary = runtime::elision::boundary_before(&candidate.text, front);
            let dropped = original.saturating_sub(boundary);
            let mut text = candidate
                .text
                .get(..boundary)
                .unwrap_or_default()
                .to_owned();
            text.push_str(&runtime::elision::marker(kernel::ByteLen::new(
                u64::try_from(dropped).unwrap_or(u64::MAX),
            )));
            seen.insert(candidate.room.clone());
            selected.push(runtime::RuleCandidate {
                room: candidate.room,
                text,
                dropped: u64::try_from(dropped).unwrap_or(u64::MAX),
            });
            break;
        }
        let rooms = selected
            .iter()
            .map(|candidate| candidate.room.clone())
            .collect();
        self.pending.insert(call.to_owned(), rooms);
        selected
    }

    pub(crate) fn replay(&mut self, call: &str, result: &kernel::Payload) {
        let rooms = result
            .as_map()
            .get("rules")
            .and_then(serde_json::Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(runtime::RuleCandidate::from_json)
            .map(|candidate| candidate.room)
            .collect();
        self.pending.insert(call.to_owned(), rooms);
    }

    pub(crate) fn complete_wave(&mut self, shortened: &BTreeSet<String>) {
        for (call, rooms) in std::mem::take(&mut self.pending) {
            if !shortened.contains(&call) {
                self.delivered.extend(rooms);
            }
        }
    }
}

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

#[cfg(test)]
mod tests;
