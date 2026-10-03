// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The fold that turns verified lines into a playback bundle's tables
//! (`crates/accounting/spec/Playback.lean` §8-12).
//!
//! Every line up to the cutoff is folded into the run lineage and the
//! key-moment ends, because a run's state and a moment's closing are
//! facts as of the cutoff, not as of the window. Every table the bundle
//! carries is then built from the lines the reader may see alone: a
//! withheld line adds to a count and to nothing else
//! (`crates/accounting/spec/Playback/Project.lean`). A committed
//! checkpoint's evidence is asked of the history folded up to its line,
//! and its diff and calls are read last, from the repository and through
//! the walk's index (`crates/accounting/spec/Playback/Traced.lean` §8-17, §8-25).

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use kernel::event::record::{BuildingCreated, RunStarted};
use kernel::layout::CityLayout;
use kernel::{Address, AxCode, AxError, EventKind, EventRecord, RunId, RunPolicy, Seq};
use storage::LedgerIndex;

use super::document::{
    Billed, Checkpoint, Closed, Costs, Decimal, Document, Event, Holds, KindCount, Phase, Reason,
    Run, Source, Withheld,
};
use super::links::{Key, Links, Role, Seen, Visibility};
use super::reader::{Readership, Sight};
use super::select::Selection;
use super::traced::{Evidence, Known, checkpoint_of};
use super::walk::Walked;
use crate::lineage::{Lineage, RunLine};

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
    /// What a committed checkpoint's base, diff and trace are read from.
    evidence: Evidence,
    /// The policy each run's visible `run_started` recorded.
    policies: BTreeMap<RunId, RunPolicy>,
    /// The `tool_called` lines the reader may not see, which a commit's
    /// trace may name.
    hidden_calls: BTreeSet<Seq>,
    scanning: Scanning,
    /// How many times this projection ran `kernel::secret::scan`.
    scans: u64,
    /// Where a deferred line's bytes are read back from by offset.
    ledger: PathBuf,
}

/// Which lines the projection runs the credential scan on
/// (`crates/accounting/spec/Playback/Project.lean`, D47).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Scanning {
    /// Every line whose buildings all read `Open`: the reference the lazy
    /// scan is compared against.
    #[cfg(test)]
    Full,
    /// Only the lines whose fate some table reads.
    Lazy,
}

/// What becomes of one line: shown, closed by a building, withheld
/// because the credential scan matched it, or, outside the selection,
/// open until a table reads it (D47).
#[derive(Debug, PartialEq, Eq)]
enum Fate {
    Shown,
    Closed(Address, Reason),
    Credential,
    Deferred,
}

#[derive(Default)]
struct Tally {
    events: u64,
    kinds: BTreeMap<EventKind, u64>,
    buildings: BTreeMap<Address, Reason>,
    credential: u64,
}

impl<'selection> Projection<'selection> {
    pub(super) fn new(
        selection: &'selection Selection,
        readership: Readership,
        city_root: &Path,
        scanning: Scanning,
    ) -> Self {
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
            evidence: Evidence::new(city_root),
            policies: BTreeMap::new(),
            hidden_calls: BTreeSet::new(),
            scanning,
            scans: 0,
            ledger: CityLayout::new(city_root).ledger(),
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
        self.evidence.absorb(&record)?;
        self.lineage.apply(&record)?;
        let room = record.addr().map(building_of);
        self.learn_buildings(&record, room.as_ref())?;
        let touches = self.links.touches(&record)?;
        let mut touched = self.links.inherited(&touches);
        touched.extend(room.iter().cloned());
        self.named_in_payload(&record, &mut touched);
        let in_range = self.selection.admits(&record);
        let fate = match self.readership.sight(&touched) {
            Sight::Closed(building, reason) => Fate::Closed(building, reason),
            Sight::Open if self.scans_now(&record, in_range) => {
                if carries_credential(&mut self.scans, raw) {
                    Fate::Credential
                } else {
                    Fate::Shown
                }
            }
            Sight::Open => Fate::Deferred,
        };
        let seen = Seen {
            visibility: match fate {
                Fate::Shown => Visibility::Visible,
                Fate::Closed(..) | Fate::Credential => Visibility::Hidden,
                Fate::Deferred => Visibility::Deferred,
            },
            in_range,
        };
        self.remember(&record, fate == Fate::Shown)?;
        let pairs_something = touches.iter().any(|touch| touch.role != Role::Member);
        let closes_a_call = touches
            .iter()
            .find(|touch| touch.role == Role::Closes && matches!(touch.key, Key::Call(..)))
            .map(|touch| touch.key.clone());
        self.links
            .note(&record, touches, (&touched, room.as_ref()), seen)?;
        let settled = closes_a_call.and_then(|key| self.links.settled_unselected(&key));
        if let Some(Some(opened)) = settled {
            self.candidates.remove(&opened);
        }
        match (seen.in_range, fate) {
            (true, Fate::Shown) => self.take(&record, raw)?,
            (true, Fate::Closed(building, reason)) => {
                self.withheld.hide(record.kind());
                self.withheld.buildings.entry(building).or_insert(reason);
            }
            // A selected line is always scanned; a deferred one here is
            // withheld rather than shown unscanned.
            (true, Fate::Credential | Fate::Deferred) => {
                self.withheld.hide(record.kind());
                self.withheld.credential = self.withheld.credential.saturating_add(1);
            }
            (false, Fate::Shown | Fate::Deferred) if pairs_something && settled.is_none() => {
                self.candidates.insert(record.seq(), event(&record, raw)?);
            }
            (false, Fate::Shown | Fate::Closed(..) | Fate::Credential | Fate::Deferred) => {}
        }
        Ok(())
    }

    /// Whether a line whose buildings all read `Open` is scanned as it
    /// is folded: it is selected, or a table reads its fate without
    /// pairing it to a selected line.
    fn scans_now(&self, record: &EventRecord, in_range: bool) -> bool {
        match self.scanning {
            #[cfg(test)]
            Scanning::Full => true,
            Scanning::Lazy => {
                in_range
                    || matches!(
                        record.kind(),
                        EventKind::RunStarted | EventKind::RunForked | EventKind::ToolCalled
                    )
            }
        }
    }

    /// The document, with `source` as the caller built it, and how many
    /// lines the projection scanned for credentials; a commit's lines
    /// are read through `index`, the one the walk built.
    ///
    /// # Errors
    /// A run's count of unanswered questions past what the bundle writes.
    pub(super) fn finish(
        mut self,
        source: Source,
        index: &LedgerIndex,
    ) -> Result<(Document, u64), AxError> {
        self.settle_deferred(index)?;
        self.attach_evidence(index);
        let mut outside = BTreeSet::new();
        let moments = self.links.moments(&mut outside);
        let messages = self.links.messages(&mut outside);
        let calls = self.links.calls(&mut outside);
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
            .map(|line| run_row(&line, &self.links, self.policies.get(&line.run).copied()))
            .collect::<Result<Vec<_>, _>>()?;
        let report = self.attribution.report();
        let document = Document {
            schema: super::SCHEMA.to_owned(),
            source,
            events: self.events,
            context,
            unknown: self.unknown,
            runs,
            moments,
            messages,
            calls,
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
        };
        Ok((document, self.scans))
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
            if let Holds::Committed { oid, .. } = &holds {
                self.evidence.hold(record.seq(), *oid);
            }
            self.checkpoints.push(Checkpoint {
                seq: Decimal(record.seq().value()),
                run: record.run(),
                holds,
            });
        }
        Ok(())
    }

    /// What a later table needs of one line beyond its own fate: the
    /// policy a visible `run_started` records, and a call the reader may
    /// not see, which a commit's trace may still name.
    fn remember(&mut self, record: &EventRecord, visible: bool) -> Result<(), AxError> {
        if visible
            && record.kind() == EventKind::RunStarted
            && let Some(policy) = record.data().read::<RunStarted>()?.policy
        {
            self.policies.insert(record.run(), policy);
        }
        if !visible && record.kind() == EventKind::ToolCalled {
            self.hidden_calls.insert(record.seq());
        }
        Ok(())
    }

    /// Scans the deferred lines a table reads, the far ends of selected
    /// pairs: from the copy the context already holds, or read back by
    /// offset through `index`.
    ///
    /// # Errors
    /// A line the index locates and the segment cannot give back.
    fn settle_deferred(&mut self, index: &LedgerIndex) -> Result<(), AxError> {
        let mut reader = index.reader(&self.ledger);
        let candidates = &self.candidates;
        let scans = &mut self.scans;
        self.links.resolve(|seq| match candidates.get(&seq) {
            Some(held) => Ok(carries_credential(scans, held.line.as_bytes())),
            None => reader
                .line_at(seq)
                .map(|raw| carries_credential(scans, &raw))
                .map_err(storage::StorageError::into_ax),
        })
    }

    /// Reads each committed checkpoint's base, diff and trace: from what
    /// the history said at its line, from the city's repository, and from
    /// the lines before it, through `index`.
    fn attach_evidence(&mut self, index: &LedgerIndex) {
        let known = Known {
            events: &self.events,
            hidden: &self.hidden_calls,
            links: &self.links,
            index,
        };
        self.evidence
            .attach(&mut self.checkpoints, &known, &mut self.readership);
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

/// Whether `raw` carries a credential by `kernel::secret::scan`, counted
/// in `scans`.
fn carries_credential(scans: &mut u64, raw: &[u8]) -> bool {
    *scans = scans.saturating_add(1);
    !kernel::secret::scan(raw).is_empty()
}

/// The building an address lies in. An address in the reserved subtree
/// belongs to no building and stands for itself, so the reader's rules
/// cannot read for it and it closes.
pub(super) fn building_of(addr: &Address) -> Address {
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

fn run_row(line: &RunLine, links: &Links, policy: Option<RunPolicy>) -> Result<Run, AxError> {
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
        policy,
    })
}
