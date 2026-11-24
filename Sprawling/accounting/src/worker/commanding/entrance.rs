// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::collections::{BTreeMap, BTreeSet};

use kernel::{AxError, EventKind, IdemKey, Payload};

pub(in crate::worker) const IDEM_FIELD: &str = "idem";

#[derive(Default, serde::Serialize, serde::Deserialize)]
pub(in crate::worker) struct Entrance {
    seen: BTreeSet<IdemKey>,
    speech_accepted: BTreeSet<IdemKey>,

    #[serde(
        serialize_with = "crate::worker::folds::as_json_text",
        deserialize_with = "crate::worker::folds::from_json_text"
    )]
    refused: BTreeMap<IdemKey, AxError>,

    #[serde(skip)]
    carrying: Option<IdemKey>,
}

impl Entrance {
    pub(in crate::worker) fn answered(&self, key: &IdemKey) -> Option<Result<(), AxError>> {
        self.seen
            .contains(key)
            .then(|| self.refused.get(key).cloned().map_or(Ok(()), Err))
    }

    pub(in crate::worker) fn begin(&mut self, key: IdemKey) {
        self.carrying = Some(key);
    }

    pub(in crate::worker) fn settle(&mut self, outcome: &Result<(), AxError>) {
        let Some(key) = self.carrying.take() else {
            return;
        };
        self.seen.insert(key);
        if let Err(err) = outcome {
            self.refused.insert(key, err.clone());
        }
    }

    pub(in crate::worker) fn carrying(&self) -> Option<IdemKey> {
        self.carrying
    }

    pub(in crate::worker) fn carry(&mut self, key: Option<IdemKey>) {
        self.carrying = key;
    }

    pub(in crate::worker) fn refused_later(&mut self, key: IdemKey, refusal: &AxError) {
        self.seen.insert(key);
        self.refused.insert(key, refusal.clone());
    }

    pub(in crate::worker) fn accepted_speech(&self, key: IdemKey) -> bool {
        self.speech_accepted.contains(&key)
    }

    pub(in crate::worker) fn absorb(&mut self, kind: EventKind, data: &Payload) {
        let Some(value) = data.as_map().get(IDEM_FIELD) else {
            return;
        };

        if let Ok(key) = serde_json::from_value::<IdemKey>(value.clone()) {
            self.seen.insert(key);
            if matches!(kind, EventKind::RunStarted | EventKind::SteerReceived) {
                self.speech_accepted.insert(key);
            }
        }
    }
}

pub(in crate::worker) fn stamped(key: Option<IdemKey>, data: Payload) -> Result<Payload, AxError> {
    let Some(key) = key else {
        return Ok(data);
    };
    let mut map = data.as_map().clone();
    map.insert(
        IDEM_FIELD.to_owned(),
        serde_json::Value::String(key.to_string()),
    );
    Payload::new(map)
}

pub(in crate::worker) fn repeated(name: &str) -> String {
    format!("{name} arrived again under a key this city has answered; the first answer stands")
}
