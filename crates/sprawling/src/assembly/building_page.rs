// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! One building, as the files in it say it is.

use std::path::Path;

use kernel::Address;

use super::name_of;

/// How much of one document travels to a page.
///
/// These files grow for as long as a building works, and the interface
/// reads them rather than edits them. A cut is stated on the answer, so
/// a reader who needs the rest knows there is a rest.
pub(crate) const DOC_BYTES_MAX: usize = 64 * 1024;

/// One building, as the files in it say it is.
///
/// The files are the authority, so the documents, the rooms and the
/// archive are read at the moment of asking rather than kept as a second
/// copy of what the disk says. The plan arrives already read, from the
/// one projection that reads it: a second parse here would be a second
/// answer to "what is stuck and why", and only one of them would be
/// folding the records that say why.
pub(crate) fn read_building(
    city_root: &Path,
    addr: &Address,
    plan: crate::plan_view::PlanReading,
) -> Option<channels::BuildingAnswer> {
    let root = city_root.join(addr.as_str());
    if !root.is_dir() {
        return None;
    }
    let crate::plan_view::PlanReading {
        progress,
        problems,
        rows: plan,
        blocked,
        ready: _,
    } = plan;
    // What counts as a room is `city::rooms`, which the model-facing
    // roster reads too: a page and an agent disagreeing about which
    // rooms a building has would be two answers to one question. A
    // directory this cannot read has no rooms to draw, which is what a
    // page owes its reader - the roster propagates the same failure
    // instead, because a resident told it is alone would act on it.
    let rooms: Vec<String> = city::rooms(city_root, addr)
        .unwrap_or_default()
        .iter()
        .map(|room| name_of(room).to_owned())
        .collect();
    let mut docs = Vec::new();
    // The rules, read by their own path: a building's rules live inside
    // a dot directory, and the walk below reads files rather than
    // directories, so a page that only walked would have quietly lost
    // the tab that shows what this building is allowed to do.
    if let Ok(bytes) = std::fs::read(city::building_path(city_root, addr)) {
        docs.push(doc_from(city::BUILDING_FILE.to_owned(), &bytes));
    }
    if let Ok(entries) = std::fs::read_dir(&root) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().into_owned();
            if name.starts_with('.') {
                continue;
            }
            if entry.path().is_dir() {
                continue;
            }
            if !name.ends_with(".md") {
                continue;
            }
            let Ok(bytes) = std::fs::read(entry.path()) else {
                continue;
            };
            docs.push(doc_from(name, &bytes));
        }
    }
    docs.sort_by_key(|doc| doc_order(&doc.name));
    let archive = city::archive_index(city_root, addr)
        .unwrap_or_default()
        .into_iter()
        .map(|entry| channels::ArchiveLine {
            kind: entry.kind.as_str().to_owned(),
            day: entry.day,
            subject: entry.subject,
        })
        .collect();
    // The building's own rung of the ladder, not the resolved value: a
    // form filled from the resolved value would write the city's
    // setting into the building the first time anybody pressed save.
    let own = city::config_path(city_root, addr, city::Layer::Building)
        .ok()
        .and_then(|path| std::fs::read_to_string(path).ok())
        .and_then(|text| city::ConfigLayer::parse(&text).ok())
        .unwrap_or_default();
    Some(channels::BuildingAnswer {
        addr: addr.clone(),
        progress,
        problems,
        plan,
        blocked,
        rooms,
        docs,
        archive,
        sandbox: own.sandbox().cloned(),
        mcp: own
            .mcp()
            .map(<[kernel::McpServer]>::to_vec)
            .unwrap_or_default(),
    })
}

/// One document as a page receives it, cut to what travels.
pub(super) fn doc_from(name: String, bytes: &[u8]) -> channels::BuildingDoc {
    let head = bytes.get(..bytes.len().min(DOC_BYTES_MAX)).unwrap_or(bytes);
    channels::BuildingDoc {
        name,
        text: String::from_utf8_lossy(head).into_owned(),
        bytes: u64::try_from(bytes.len()).unwrap_or(u64::MAX),
        truncated: bytes.len() > DOC_BYTES_MAX,
    }
}

/// Reading order for a building's documents: the plan, then the record of
/// decisions, then the handoff, then the rules. A person opening a
/// building wants to know what it is doing before they read what it is
/// allowed to do.
pub(super) fn doc_order(name: &str) -> (u8, String) {
    let rank = match name {
        city::ROADMAP_FILE => 0,
        "Memo.md" => 1,
        "Handoff.md" => 2,
        city::BUILDING_FILE => 3,
        city::URBANITE_FILE => 4,
        _ => 5,
    };
    (rank, name.to_owned())
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
