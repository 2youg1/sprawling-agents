// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::path::Path;

use kernel::Address;

pub(crate) fn read_building(
    city_root: &Path,
    addr: &Address,
    plan: crate::plan_view::PlanReading,
) -> Option<wire::BuildingAnswer> {
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

    let rooms: Vec<String> = city::rooms(city_root, addr)
        .unwrap_or_default()
        .iter()
        .map(|room| room.name().to_owned())
        .collect();
    let mut docs = Vec::new();

    if let Ok(bytes) = std::fs::read(city::rules_path(city_root, addr)) {
        docs.push(doc_from(city::RULES_FILE.to_owned(), &bytes));
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
        .map(|entry| wire::ArchiveLine {
            kind: entry.kind.as_str().to_owned(),
            day: entry.day,
            subject: entry.subject,
        })
        .collect();

    let own = city::config_path(city_root, addr, city::Layer::Building)
        .ok()
        .and_then(|path| std::fs::read_to_string(path).ok())
        .and_then(|text| city::ConfigLayer::parse(&text).ok())
        .unwrap_or_default();
    Some(wire::BuildingAnswer {
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
            .and_then(|mcp| mcp.definitions.as_ref())
            .map(|definitions| {
                definitions
                    .iter()
                    .map(|definition| definition.server.clone())
                    .collect()
            })
            .unwrap_or_default(),
    })
}

pub(super) fn doc_from(name: String, bytes: &[u8]) -> wire::BuildingDoc {
    let size = u64::try_from(bytes.len()).unwrap_or(u64::MAX);
    let head = usize::try_from(size.min(documents::WINDOW_BYTES_MAX))
        .map_or(bytes, |cut| bytes.get(..cut).unwrap_or(bytes));
    wire::BuildingDoc {
        name,
        text: String::from_utf8_lossy(head).into_owned(),
        bytes: size,
        truncated: size > documents::WINDOW_BYTES_MAX,
    }
}

pub(super) fn doc_order(name: &str) -> (u8, String) {
    let rank = match name {
        city::ROADMAP_FILE => 0,
        "Memo.md" => 1,
        "Handoff.md" => 2,
        city::RULES_FILE => 3,
        city::URBANITE_FILE => 4,
        _ => 5,
    };
    (rank, name.to_owned())
}
