// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The fold that turns verified lines into a playback bundle's tables
//! (accounting-SPEC.md 8-12).
//!
//! Every line up to the cutoff is folded into the run lineage and the
//! key-moment ends, because a run's state and a moment's closing are
//! facts as of the cutoff, not as of the window. Every table the bundle
//! carries is then built from the lines the reader may see alone: a
//! withheld line adds to a count and to nothing else
//! (`crates/accounting/spec/Playback/Project.lean`).

use std::collections::{BTreeMap, BTreeSet};

use kernel::event::record::{BuildingCreated, CheckpointCommitted};
use kernel::{Address, AxCode, AxError, EventKind, EventRecord, RunId, Seq};

use super::document::{
    Billed, Checkpoint, Closed, Costs, Decimal, Document, Event, Holds, KindCount, Phase, Reason,
    Run, Source, Withheld,
};
use super::links::{Links, Role, Seen};
use super::reader::{Readership, Sight};
use super::select::Selection;
use super::walk::Walked;
use crate::lineage::{Lineage, RunLine};
use crate::views::commits::commit_facts;

pub(super) struct Projection<'selection> {
    selection: &'selection Selection,
    readership: Readership,
    lineage: Lineage,
    /// Every building this history has named so far: an address in a
    /// payload counts as touching a building only when it starts with one.
    buildings: BTreeSet<Address>,
    links: Links,
    events: Vec<Event>,
    /// Visible lines outside the selection that may be the far end of a
    /// selected line's pair.
    candidates: BTreeMap<Seq, Event>,
    runs: BTreeSet<RunId>,
    unknown: Vec<Decimal>,
    withheld: Tally,
    checkpoints: Vec<Checkpoint>,
    attribution: storage::Attribution,
}

/// What becomes of one line: shown, closed by a building, or withheld
/// because the credential scan matched it.
#[derive(Debug, PartialEq, Eq)]
enum Fate {
    Shown,
    Closed(Address, Reason),
    Credential,
}

#[derive(Default)]
struct Tally {
    events: u64,
    kinds: BTreeMap<EventKind, u64>,
    buildings: BTreeMap<Address, Reason>,
    credential: u64,
}

impl<'selection> Projection<'selection> {
    pub(super) fn new(selection: &'selection Selection, readership: Readership) -> Self {
        Projection {
            selection,
            readership,
            lineage: Lineage::default(),
            buildings: BTreeSet::new(),
            links: Links::default(),
            events: Vec::new(),
            candidates: BTreeMap::new(),
            runs: BTreeSet::new(),
            unknown: Vec::new(),
            withheld: Tally::default(),
            checkpoints: Vec::new(),
            attribution: storage::Attribution::new(),
        }
    }

    /// Folds one verified line in, in seq order.
    ///
    /// # Errors
    /// A payload the lineage, the links or the checkpoint table must read
    /// and cannot, and a line that is not UTF-8.
    pub(super) fn apply(&mut self, raw: &[u8], walked: Walked) -> Result<(), AxError> {
        let record = match walked {
            Walked::Known(record) => record,
            Walked::Unknown(seq) => {
                if self.selection.holds_seq(seq) {
                    self.unknown.push(Decimal(seq.value()));
                }
                return Ok(());
            }
        };
        self.lineage.apply(&record)?;
        let room = record.addr().map(building_of);
        self.learn_buildings(&record, room.as_ref())?;
        let touches = self.links.touches(&record)?;
        let mut touched = self.links.inherited(&touches);
        touched.extend(room.iter().cloned());
        self.named_in_payload(&record, &mut touched);
        let fate = match self.readership.sight(&touched) {
            Sight::Closed(building, reason) => Fate::Closed(building, reason),
            Sight::Open if kernel::secret::scan(raw).is_empty() => Fate::Shown,
            Sight::Open => Fate::Credential,
        };
        let seen = Seen {
            visible: fate == Fate::Shown,
            in_range: self.selection.admits(&record),
        };
        let pairs_something = touches.iter().any(|touch| touch.role != Role::Member);
        self.links
            .note(&record, touches, (&touched, room.as_ref()), seen)?;
        match (seen.in_range, fate) {
            (true, Fate::Shown) => self.take(&record, raw)?,
            (true, Fate::Closed(building, reason)) => {
                self.withheld.hide(record.kind());
                self.withheld.buildings.entry(building).or_insert(reason);
            }
            (true, Fate::Credential) => {
                self.withheld.hide(record.kind());
                self.withheld.credential = self.withheld.credential.saturating_add(1);
            }
            (false, Fate::Shown) if pairs_something => {
                self.candidates.insert(record.seq(), event(&record, raw)?);
            }
            (false, Fate::Shown | Fate::Closed(..) | Fate::Credential) => {}
        }
        Ok(())
    }

    /// The document, with `source` as the caller built it.
    ///
    /// # Errors
    /// A run's count of unanswered questions past what the bundle writes.
    pub(super) fn finish(self, source: Source) -> Result<Document, AxError> {
        let mut outside = BTreeSet::new();
        let moments = self.links.moments(&mut outside);
        let messages = self.links.messages(&mut outside);
        let context = self
            .candidates
            .into_iter()
            .filter(|(seq, _)| outside.contains(seq))
            .map(|(_, event)| event)
            .collect();
        let runs = self
            .lineage
            .lines()
            .filter(|line| self.runs.contains(&line.run))
            .map(|line| run_row(&line, &self.links))
            .collect::<Result<Vec<_>, _>>()?;
        let report = self.attribution.report();
        Ok(Document {
            schema: super::SCHEMA.to_owned(),
            source,
            events: self.events,
            context,
            unknown: self.unknown,
            runs,
            moments,
            messages,
            checkpoints: self.checkpoints,
            costs: Costs {
                billed_usd_micros: Decimal(report.total.get()),
                by_run: report
                    .by_run
                    .into_iter()
                    .map(|(run, billed)| Billed {
                        run,
                        usd_micros: Decimal(billed.get()),
                    })
                    .collect(),
                unpriced_calls: Decimal(report.unpriced.calls),
                unpriced_tokens: Decimal(report.unpriced.tokens),
            },
            withheld: Withheld {
                events: Decimal(self.withheld.events),
                kinds: self
                    .withheld
                    .kinds
                    .into_iter()
                    .map(|(kind, count)| KindCount {
                        kind,
                        count: Decimal(count),
                    })
                    .collect(),
                buildings: self
                    .withheld
                    .buildings
                    .into_iter()
                    .map(|(building, reason)| Closed { building, reason })
                    .collect(),
                credential: Decimal(self.withheld.credential),
            },
        })
    }

    /// A selected line the reader sees: into the events, and into every
    /// table it moves.
    fn take(&mut self, record: &EventRecord, raw: &[u8]) -> Result<(), AxError> {
        self.events.push(event(record, raw)?);
        if record.run() != RunId::CITY {
            self.runs.insert(record.run());
        }
        self.attribution
            .apply(record)
            .map_err(storage::StorageError::into_ax)?;
        if let Some(holds) = checkpoint_of(record)? {
            self.checkpoints.push(Checkpoint {
                seq: Decimal(record.seq().value()),
                run: record.run(),
                holds,
            });
        }
        Ok(())
    }

    /// Adds the building a line is addressed in, and the one a
    /// `building_created` raises, to the buildings this history has named.
    fn learn_buildings(
        &mut self,
        record: &EventRecord,
        room: Option<&Address>,
    ) -> Result<(), AxError> {
        self.buildings.extend(room.cloned());
        if record.kind() == EventKind::BuildingCreated {
            let created = record.data().read::<BuildingCreated>()?;
            self.buildings.insert(building_of(&created.addr));
        }
        Ok(())
    }

    /// Adds to `touched` every building a string in the payload names by
    /// an address that starts with it.
    fn named_in_payload(&self, record: &EventRecord, touched: &mut BTreeSet<Address>) {
        let mut pending: Vec<&serde_json::Value> = record.data().as_map().values().collect();
        while let Some(value) = pending.pop() {
            match value {
                serde_json::Value::String(text) => {
                    if let Ok(addr) = Address::parse(text) {
                        let building = building_of(&addr);
                        if self.buildings.contains(&building) {
                            touched.insert(building);
                        }
                    }
                }
                serde_json::Value::Array(items) => pending.extend(items),
                serde_json::Value::Object(map) => pending.extend(map.values()),
                serde_json::Value::Null
                | serde_json::Value::Bool(_)
                | serde_json::Value::Number(_) => {}
            }
        }
    }
}

impl Tally {
    fn hide(&mut self, kind: EventKind) {
        self.events = self.events.saturating_add(1);
        let count = self.kinds.entry(kind).or_insert(0);
        *count = count.saturating_add(1);
    }
}

/// The building an address lies in. An address in the reserved subtree
/// belongs to no building and stands for itself, so the reader's rules
/// cannot read for it and it closes.
fn building_of(addr: &Address) -> Address {
    city::Building::of(addr).map_or_else(|_| addr.clone(), |building| building.addr().clone())
}

fn event(record: &EventRecord, raw: &[u8]) -> Result<Event, AxError> {
    let line = std::str::from_utf8(raw).map_err(|err| {
        AxError::failure(
            AxCode::CasCorrupt,
            "export a playback bundle",
            format!("seq {}", record.seq().value()),
        )
        .with_recovery(format!(
            "the line is not UTF-8 ({err}); verify the ledger with sprawling replay"
        ))
    })?;
    Ok(Event {
        seq: Decimal(record.seq().value()),
        moment: record.moment().map(|moment| Decimal(moment.value())),
        line: line.to_owned(),
    })
}

fn run_row(line: &RunLine, links: &Links) -> Result<Run, AxError> {
    let unanswered = u64::try_from(line.unanswered).map_err(|_| {
        AxError::failure(
            AxCode::InvalidArgs,
            "export a playback bundle",
            format!("run {}", line.run),
        )
        .with_recovery("the count of unanswered questions does not fit; report it as a defect")
    })?;
    Ok(Run {
        run: line.run,
        addr: line.addr.clone(),
        session: line.session.map(|seq| Decimal(seq.value())),
        parent: line.parent.map(|run| links.related(run)),
        forked_at: line.forked_at.map(|seq| Decimal(seq.value())),
        predecessor: line.predecessor.map(|run| links.related(run)),
        first_seq: Decimal(line.first_seq.value()),
        last_seq: Decimal(line.last_seq.value()),
        state: line.state.as_ref().map(|phase| match phase {
            storage::RunPhase::Active => Phase::Active,
            storage::RunPhase::Frozen => Phase::Frozen,
        }),
        unanswered: Decimal(unanswered),
    })
}

/// What a line says about a commit or a job pin: the pin read by its own
/// type, and every commit as `commit_facts`, the one place that tells
/// which lines name a commit and which oid, identifies it
/// (accounting-SPEC.md 8-12, decision 24(f)).
fn checkpoint_of(record: &EventRecord) -> Result<Option<Holds>, AxError> {
    let named = commit_facts(record).map(|(oid, _)| oid);
    if record.kind() == EventKind::CheckpointCommitted {
        return Ok(
            match (record.data().read::<CheckpointCommitted>()?, named) {
                (CheckpointCommitted::JobPinned { job }, _) => Some(Holds::Pinned { job }),
                (CheckpointCommitted::Committed(commit), Some(oid)) => Some(Holds::Committed {
                    oid,
                    scope: commit.scope,
                    files: commit.files,
                }),
                (CheckpointCommitted::Committed(_), None) => None,
            },
        );
    }
    Ok(named
        .filter(|_| record.kind() == EventKind::PrMerged)
        .map(|oid| Holds::Merged { oid }))
}
