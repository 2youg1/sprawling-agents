// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::collections::BTreeMap;

use kernel::{AxCode, AxError, B3Hash, EventRecord, Seq, TimeMs};

use crate::frames::Query;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CityLabel(String);

impl CityLabel {

    pub fn parse(raw: &str) -> Result<Self, AxError> {
        if raw.is_empty() || raw.chars().any(char::is_control) {
            return Err(AxError::failure(
                AxCode::ConfigInvalid,
                "label an upstream city",
                "the label is empty or holds control characters",
            )
            .with_recovery("give the city a short single-line name"));
        }
        Ok(Self(raw.to_owned()))
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone)]
pub struct Upstream {
    pub label: CityLabel,

    pub address: String,

    pub token_digest: Option<B3Hash>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Sighting {
    pub city: CityLabel,
    pub event: EventRecord,
}

#[derive(Debug, Default)]
pub struct Aggregate {
    upstreams: BTreeMap<CityLabel, Upstream>,
}

impl Aggregate {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn attach(&mut self, upstream: Upstream) {
        self.upstreams.insert(upstream.label.clone(), upstream);
    }

    pub fn detach(&mut self, label: &CityLabel) -> bool {
        self.upstreams.remove(label).is_some()
    }

    pub fn cities(&self) -> impl Iterator<Item = &Upstream> {
        self.upstreams.values()
    }

    pub fn ask(&self, label: &CityLabel, query: Query) -> Result<Forwarded, AxError> {
        let upstream = self.upstreams.get(label).ok_or_else(|| {
            AxError::failure(
                AxCode::ConfigInvalid,
                "forward a query to another city",
                format!("no city is attached under the label `{}`", label.as_str()),
            )
            .with_recovery("attach the city with its address and pairing code first")
        })?;
        Ok(Forwarded {
            address: upstream.address.clone(),
            query,
        })
    }

    #[must_use]
    pub fn merge(streams: Vec<(CityLabel, Vec<EventRecord>)>) -> Vec<Sighting> {
        let mut merged: Vec<Sighting> = streams
            .into_iter()
            .flat_map(|(city, events)| {
                events.into_iter().map(move |event| Sighting {
                    city: city.clone(),
                    event,
                })
            })
            .collect();
        merged.sort_by_key(sort_key);
        merged
    }
}

fn sort_key(sighting: &Sighting) -> (TimeMs, CityLabel, Seq) {
    (
        sighting.event.t(),
        sighting.city.clone(),
        sighting.event.seq(),
    )
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Forwarded {
    pub address: String,
    pub query: Query,
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    reason = "test code"
)]
mod tests {
    use super::*;

    fn city(name: &str) -> CityLabel {
        CityLabel::parse(name).unwrap()
    }

    fn upstream(name: &str) -> Upstream {
        Upstream {
            label: city(name),
            address: format!("{name}.local:2501"),
            token_digest: None,
        }
    }

    #[test]
    fn attaching_the_same_label_twice_replaces_rather_than_duplicates() {
        let mut aggregate = Aggregate::new();
        aggregate.attach(upstream("studio"));
        let mut corrected = upstream("studio");
        corrected.address = "192.168.1.9:2501".to_owned();
        aggregate.attach(corrected);
        assert_eq!(aggregate.cities().count(), 1);
        let only = aggregate.cities().next().unwrap();
        assert_eq!(only.address, "192.168.1.9:2501");
    }

    #[test]
    fn asking_an_unattached_city_is_refused_not_guessed() {
        let aggregate = Aggregate::new();
        let err = aggregate
            .ask(&city("nowhere"), Query::CityView)
            .expect_err("an unknown label has no address to guess");
        assert_eq!(*err.code(), AxCode::ConfigInvalid);
        assert!(!err.recovery().is_empty());
    }

    fn event(at: u64, seq: u64) -> EventRecord {
        EventRecord::from_draft(
            kernel::EventDraft {
                run: kernel::RunId::from_bytes([4u8; 16]),
                t: TimeMs::new(at),
                who: "test".to_owned(),
                addr: None,
                kind: kernel::EventKind::RunStarted,
                data: kernel::Payload::empty(),
                ig: false,
            },
            Seq::new(seq),
            B3Hash::digest(b"prev"),
        )
    }

    #[test]
    fn merging_is_the_same_sequence_however_the_streams_arrive() {

        let left = (city("attic"), vec![event(10, 1), event(30, 2)]);
        let right = (city("studio"), vec![event(10, 7), event(20, 8)]);

        let one = Aggregate::merge(vec![left.clone(), right.clone()]);
        let other = Aggregate::merge(vec![right, left]);
        assert_eq!(one, other, "merge order cannot depend on arrival order");

        let trail: Vec<(&str, u64)> = one
            .iter()
            .map(|s| (s.city.as_str(), s.event.t().value()))
            .collect();
        assert_eq!(
            trail,
            [("attic", 10), ("studio", 10), ("studio", 20), ("attic", 30)]
        );
    }

    #[test]
    fn cities_iterate_in_label_order_regardless_of_attachment_order() {
        let mut aggregate = Aggregate::new();
        for name in ["studio", "attic", "shed"] {
            aggregate.attach(upstream(name));
        }
        let order: Vec<&str> = aggregate.cities().map(|u| u.label.as_str()).collect();
        assert_eq!(order, ["attic", "shed", "studio"]);
    }
}
