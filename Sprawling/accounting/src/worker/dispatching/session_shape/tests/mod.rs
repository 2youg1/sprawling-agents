// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

#![allow(
    clippy::float_arithmetic,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::string_slice,
    clippy::arithmetic_side_effects,
    reason = "test code"
)]

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex, PoisonError};

use crate::worker::fixture::*;
use crate::worker::*;

type Row = (String, String, u64);

type Prefixes = Arc<Mutex<Vec<Vec<Row>>>>;

fn watch(worker: &mut RunWorker, seen: &Prefixes) {
    let seen = Arc::clone(seen);
    worker.observe(Box::new(move |record: &EventRecord| {
        if record.kind() != EventKind::PromptAssembled {
            return;
        }
        let rows = record
            .data()
            .as_map()
            .get("segments")
            .and_then(serde_json::Value::as_array)
            .map(|rows| rows.iter().filter_map(row_of).collect())
            .unwrap_or_default();
        seen.lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push(rows);
    }));
}

fn row_of(row: &serde_json::Value) -> Option<Row> {
    Some((
        row.get("slot")?.as_str()?.to_owned(),
        row.get("hash")?.as_str()?.to_owned(),
        row.get("len")?.as_u64()?,
    ))
}

fn bytes_of(city_root: &std::path::Path, rows: &[Row]) -> BTreeMap<String, Vec<u8>> {
    let store = storage::Cas::open(&kernel::layout::CityLayout::new(city_root).cas()).unwrap();
    rows.iter()
        .map(|(slot, hash, _)| {
            let hash: kernel::B3Hash =
                serde_json::from_value(serde_json::Value::String(hash.clone())).unwrap();
            (slot.clone(), store.get(&hash).unwrap())
        })
        .collect()
}

fn ask(addr: &Address, effort: Option<kernel::Effort>, key: &[u8]) -> wire::Command {
    wire::Command::Dispatch {
        addr: addr.clone(),
        task: "write one line".to_owned(),
        goal: "the line is written".to_owned(),
        policy: kernel::RunPolicy::of(kernel::Mode::Work),
        idem: kernel::IdemKey::derive(&RunId::CITY, kernel::Seq::FIRST, key),
        session: None,
        resume: None,
        images: Vec::new(),
        effort,
        model: None,
    }
}

fn ask_on(addr: &Address, model: &str, key: &[u8]) -> wire::Command {
    let mut dispatch = ask(addr, None, key);
    if let wire::Command::Dispatch { model: named, .. } = &mut dispatch {
        *named = Some(model.to_owned());
    }
    dispatch
}

fn choose(worker: &mut RunWorker, model: &str, key: &[u8]) {
    worker
        .handle(wire::Command::SelectModel {
            endpoint: wire::ProviderName::parse("house").unwrap(),
            model: model.to_owned(),
            tag: kernel::ModelTag::Main,
            context_tokens: kernel::Window::new(32_768),
            max_output_tokens: kernel::Ceiling::new(4_096),
            input: None,
            idem: kernel::IdemKey::derive(&RunId::CITY, kernel::Seq::FIRST, key),
        })
        .unwrap();
}

fn set_temperature(worker: &mut RunWorker, temperature: &str) {
    let endpoint = worker
        .credentials
        .book
        .endpoints()
        .find(|endpoint| endpoint.name == "house")
        .unwrap()
        .clone();
    let mut tuning = crate::tuning::tuning_as_attached(&endpoint.tuning);
    tuning.accounts = None;
    tuning.generation = vec![kernel::ModelGeneration {
        model: "m-local".to_owned(),
        settings: kernel::GenerationSettings {
            temperature: Some(temperature.to_owned()),
            ..kernel::GenerationSettings::default()
        },
        ..kernel::ModelGeneration::default()
    }];
    worker
        .handle(wire::Command::AttachEndpoint {
            name: wire::ProviderName::parse("house").unwrap(),
            base_url: endpoint.base_url,
            dialect: endpoint.dialect,
            secret: None,
            auth_header: None,
            admit: endpoint
                .models
                .iter()
                .filter(|row| row.listing == kernel::event::record::Listing::Listed)
                .map(|row| row.id.clone())
                .collect(),
            named: endpoint
                .models
                .iter()
                .filter(|row| row.listing == kernel::event::record::Listing::Named)
                .map(|row| row.id.clone())
                .collect(),
            tuning,
            idem: kernel::IdemKey::derive(&RunId::CITY, kernel::Seq::FIRST, temperature.as_bytes()),
        })
        .unwrap();
}

fn open_lab(dir: &std::path::Path) {
    crate::worker::fixture::init_city(dir).unwrap();
    city::create_building(
        dir,
        &Address::parse("lab").unwrap(),
        city::BuildingTemplate::Minimal,
    )
    .unwrap();
}

mod model;
mod resume;
mod shape;
