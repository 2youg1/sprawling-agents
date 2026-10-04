// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Background skill audits (`crates/sprawling/spec/Assembly.lean`, city D19).
//! The observer only queues notices; this thread writes through Relay.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::sync::mpsc;

use accounting::{Clock as _, worker::Relay};
use kernel::event::record::{ShelvedFrom, SkillAudited, SkillShelved};
use kernel::{
    AxCode, AxError, B3Hash, EventDraft, EventKind, EventRecord, Ledger as _, Payload, RunId, Seq,
};

mod clients;

/// Starts a city-lifetime consumer. Closing the sender ends it after the
/// current audit; a startup refusal is advisory and leaves serving intact.
pub(super) fn start(
    root: PathBuf,
    relay: Relay,
    proof: storage::ChainHalt,
) -> Result<mpsc::Sender<EventRecord>, AxError> {
    let (send, receive) = mpsc::channel();
    std::thread::Builder::new()
        .name("sprawling-skill-audit".to_owned())
        .spawn(move || {
            proof.await_verdict();
            if let Some(err) = proof.reason() {
                eprintln!("skill audits stopped: {err}");
                return;
            }
            if let Err(err) = serve(&root, relay, &receive) {
                eprintln!("skill audits stopped: {err}; {}", err.recovery());
            }
        })
        .map_err(|err| {
            AxError::failure(
                AxCode::ToolUnavailable,
                "start skill audits",
                err.to_string(),
            )
            .with_recovery("restart the server to audit the shelf; shelving continues")
        })?;
    Ok(send)
}

#[derive(Default)]
struct History {
    sources: BTreeMap<String, ShelvedFrom>,
    audited: BTreeSet<(String, B3Hash)>,
    attempted: BTreeSet<(String, B3Hash)>,
    through: Option<Seq>,
}

impl History {
    fn absorb(&mut self, record: &EventRecord) -> Result<(), AxError> {
        if self.through.is_some_and(|through| record.seq() <= through) {
            return Ok(());
        }
        self.through = Some(record.seq());
        if record.kind() == EventKind::SkillShelved {
            let shelved: SkillShelved = record.data().read()?;
            self.sources.insert(shelved.skill, shelved.source);
        } else if record.kind() == EventKind::SkillAudited {
            let audited: SkillAudited = record.data().read()?;
            if matches!(
                city::audit_state(
                    &audited.digest,
                    &[(audited.digest, audited.verdict, record.seq())],
                ),
                city::AuditState::Audited { .. }
            ) {
                self.audited.insert((audited.skill, audited.digest));
            }
        }
        Ok(())
    }

    fn scan(&mut self, root: &Path, relay: &mut Relay) -> Result<(), AxError> {
        let home = match accounting::home::Home::detect() {
            Ok(home) => home,
            Err(err) => {
                eprintln!("skill audit scan unavailable: {err}");
                return Ok(());
            }
        };
        let buildings = match city::buildings(root) {
            Ok(buildings) => buildings,
            Err(err) => {
                eprintln!("skill audit scan unavailable: {err}");
                return Ok(());
            }
        };
        for building in std::iter::once(None).chain(buildings.iter().map(Some)) {
            let library = match city::Library::scan(root, building, home.path()) {
                Ok(library) => library,
                Err(err) => {
                    eprintln!("skill audit shelf unavailable: {err}");
                    continue;
                }
            };
            for holding in library.all() {
                let address = holding.package.as_ref().or_else(|| holding.shelf.address());
                let Some(address) = address else {
                    continue;
                };
                let path = root.join(address.as_str());
                let digest = match city::skill_digest(&path) {
                    Ok(digest) => digest,
                    Err(err) => {
                        eprintln!("skill audit could not read {}: {err}", holding.name);
                        continue;
                    }
                };
                let key = (holding.name.clone(), digest);
                if self.audited.contains(&key) || !self.attempted.insert(key) {
                    continue;
                }
                let source = self
                    .sources
                    .get(&holding.name)
                    .cloned()
                    .unwrap_or(ShelvedFrom::Path);
                let request = city::AuditRequest {
                    skill: holding.name.clone(),
                    digest,
                    path,
                    source,
                };
                let report = clients::audit(&request);
                for err in report.failures {
                    eprintln!("skill audit: {err}; {}", err.recovery());
                }
                let drafts = report
                    .records
                    .iter()
                    .map(|record| {
                        Ok(EventDraft {
                            run: RunId::CITY,
                            t: super::SystemClock.now()?,
                            who: "city".to_owned(),
                            addr: None,
                            kind: EventKind::SkillAudited,
                            data: Payload::of(record)?,
                            ig: false,
                        })
                    })
                    .collect::<Result<Vec<_>, AxError>>()?;
                relay.append_all(drafts)?;
            }
        }
        Ok(())
    }
}

fn serve(
    root: &Path,
    mut relay: Relay,
    receive: &mpsc::Receiver<EventRecord>,
) -> Result<(), AxError> {
    let mut history = History::default();
    runtime::replay::fold_ledger_dir(&kernel::layout::CityLayout::new(root).ledger(), |record| {
        history.absorb(record)
    })?;
    history.scan(root, &mut relay)?;
    while let Ok(record) = receive.recv() {
        history.absorb(&record)?;
        history.scan(root, &mut relay)?;
    }
    Ok(())
}
