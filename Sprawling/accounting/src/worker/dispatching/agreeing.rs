// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use kernel::event::Scope;
use kernel::event::record::{GoverningDocument, RulesChanged};
use kernel::{Address, AxCode, AxError, EventKind, Payload};
use kernel::{Locator, RunId, TimeMs};
use std::path::{Path, PathBuf};

use crate::worker::CommandDesk;

use super::super::RunWorker;
use super::{Agreed, Assignment, HarnessSeat, Seat};

pub(in crate::worker) fn run_id_for(job: &Locator, addr: &Address, now: TimeMs) -> RunId {
    let seed = format!("{job}|{}|{}", addr.as_str(), now.value());
    let digest = kernel::B3Hash::digest(seed.as_bytes());
    let mut bytes = [0u8; 16];
    for (slot, byte) in bytes.iter_mut().zip(digest.as_bytes()) {
        *slot = *byte;
    }
    RunId::from_bytes(bytes)
}

pub fn acp_dispatch(
    desk: &CommandDesk,
    body: &serde_json::Value,
    pairing: wire::Pairing,
) -> Result<wire::AcpProgress, AxError> {
    if matches!(pairing, wire::Pairing::Absent) {
        return Err(AxError::failure(
            AxCode::GateDenied,
            "admit an external request",
            "not paired with this city".to_owned(),
        )
        .with_recovery("pair the client from the settings page, then send the token it shows"));
    }

    let agent_protocols::Admitted::Dispatch { addr, task, goal } =
        agent_protocols::admit(agent_protocols::Incoming::parse(body)?)?;
    let idem = kernel::IdemKey::derive(
        &RunId::CITY,
        kernel::Seq::FIRST,
        format!("acp:{}:{task}", addr.as_str()).as_bytes(),
    );

    desk.post(
        wire::Command::Dispatch {
            addr,
            task,
            goal,
            policy: wire::RunPolicy::of(wire::Mode::Work),
            idem,

            session: None,
            resume: None,
            images: Vec::new(),

            effort: None,
            model: None,
        },
        wire::Reply::nowhere(),
    );
    Ok(wire::AcpProgress {
        run: idem.to_string(),
        turns: 0,
        finished: false,
    })
}

impl RunWorker {
    pub(super) fn agree_to_work(&mut self, at: &mut Assignment) -> Result<Seat, AxError> {
        let addr = &at.addr;
        if let Some(scope) = self.halted_by(addr) {
            return Err(AxError::failure(
                AxCode::GateDenied,
                "dispatch work",
                addr.as_str().to_owned(),
            )
            .with_recovery(format!(
                "{scope} is halted; release it to let work in again. Runs already going are \
                 unaffected - stopping one is `cancel`"
            )));
        }

        let (building, rules, judged) = match seat_from_files(&self.city_root, &Asked::of(at))? {
            RoomSeat::Harness(seat) => return Ok(Seat::Harness(seat)),
            RoomSeat::Model {
                building,
                rules,
                judged,
            } => (building, rules, judged),
        };
        let own = city::own_layer(&self.city_root, addr)?;
        let (chosen, routing) = self.route_model(at, &rules, &own)?;
        let model = chosen.entry.clone();
        let provider = chosen.endpoint.name.clone();
        let adapter = self.models.build(&chosen, self.redemption()?)?;
        let retries = chosen.endpoint.tuning.request_max_retries;

        let clock = std::sync::Arc::clone(&self.clock);
        let adapter = crate::worker::keeping_warm::Door::new(
            adapter,
            city::keep_warm(&self.city_root, &judged)?,
            Box::new(move || clock.now()),
            self.warm.lead_of(&provider),
        );
        Ok(Seat::Model(Agreed {
            building,
            rules,
            model,
            routing,
            provider,
            adapter,
            retries,
        }))
    }

    pub(super) fn book_rules(&mut self, building: &city::Building) -> Result<(), AxError> {
        let mut documents = vec![(
            Scope::City,
            GoverningDocument::Config,
            city::config_path(&self.city_root, building.addr(), city::Layer::City)?,
        )];
        let root = building.root(&self.city_root);
        if root.try_exists().map_err(|err| unreadable(&root, &err))? {
            let scope = Scope::Building(building.addr().clone());
            documents.push((
                scope.clone(),
                GoverningDocument::Config,
                city::config_path(&self.city_root, building.addr(), city::Layer::Building)?,
            ));
            documents.push((
                scope,
                GoverningDocument::Rules,
                city::rules_path(&self.city_root, building.addr()),
            ));
        }
        for (scope, which, path) in documents {
            self.book_document(scope, which, &path)?;
        }
        Ok(())
    }

    pub(in crate::worker) fn book_document(
        &mut self,
        scope: Scope,
        which: GoverningDocument,
        path: &std::path::Path,
    ) -> Result<(), AxError> {
        let bytes = match std::fs::read(path) {
            Ok(bytes) => bytes,
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => Vec::new(),
            Err(err) => return Err(unreadable(path, &err)),
        };
        let after = kernel::B3Hash::digest(&bytes);
        let before = self.governance.rules.get(&(scope.clone(), which)).copied();
        if before == Some(after) {
            return Ok(());
        }
        let changed = RulesChanged {
            scope,
            which,
            before,
            after,
            bytes: bytes.len(),
        };
        self.record(EventKind::RulesChanged, Payload::of(&changed)?)
    }

    pub(super) fn tag_for(
        &self,
        named: Option<&str>,
        frozen: Option<&str>,
    ) -> Result<kernel::ModelTag, AxError> {
        let Some(id) = named else {
            return Ok(frozen
                .and_then(|id| self.tag_registering(id))
                .unwrap_or(kernel::ModelTag::Main));
        };
        self.tag_registering(id).ok_or_else(|| {
            AxError::failure(
                AxCode::ConfigInvalid,
                "dispatch work",
                format!("no tag registers the model {id}"),
            )
            .with_recovery("register it under a tag on the settings page, then dispatch again")
        })
    }

    fn tag_registering(&self, id: &str) -> Option<kernel::ModelTag> {
        self.credentials
            .book
            .choices()
            .find_map(|(tag, _, entry)| (entry.id == id).then_some(tag))
    }
}

struct Asked<'a> {
    addr: &'a Address,
    session: Option<&'a kernel::SessionName>,
    model: Option<&'a str>,
}

impl<'a> Asked<'a> {
    fn of(at: &'a Assignment) -> Asked<'a> {
        Asked {
            addr: &at.addr,
            session: at.session.as_ref(),
            model: at.model.as_deref(),
        }
    }

    fn handed_down(room: &'a Address) -> Asked<'a> {
        Asked {
            addr: room,
            session: None,
            model: None,
        }
    }
}

enum RoomSeat {
    Harness(HarnessSeat),
    Model {
        building: city::Building,
        rules: city::BuildingRules,

        judged: Address,
    },
}

fn seat_from_files(city_root: &Path, asked: &Asked<'_>) -> Result<RoomSeat, AxError> {
    let building = city::Building::of(asked.addr)?;
    let rules = city::load(city_root, building.addr())?;

    let judged = if super::session::opens_a_room(asked.addr, asked.session) {
        building.addr().clone()
    } else {
        asked.addr.clone()
    };
    let Some((word, layer)) = city::settled_harness(city_root, &judged)? else {
        return Ok(RoomSeat::Model {
            building,
            rules,
            judged,
        });
    };
    let file = city::config_path(city_root, &judged, layer)?;
    let roster = crate::roster::roster(city_root)?;
    let named = Named {
        word: &word,
        file: &file,
    };
    let harness = seated_harness(asked, named, &rules, &roster)?;
    Ok(RoomSeat::Harness(HarnessSeat {
        building,
        rules,
        harness,
    }))
}

pub(in crate::worker) fn start_check(city_root: PathBuf) -> collab::StartCheck {
    collab::StartCheck::new(move |room: &Address| {
        seat_from_files(&city_root, &Asked::handed_down(room)).map(|_seated| ())
    })
}

struct Named<'a> {
    word: &'a str,
    file: &'a std::path::Path,
}

fn seated_harness(
    asked: &Asked<'_>,
    named: Named<'_>,
    rules: &city::BuildingRules,
    roster: &agent_protocols::Roster,
) -> Result<agent_protocols::Consented, AxError> {
    let Named { word, file } = named;
    let subject = |named: &str| format!("{}: {named}", asked.addr.as_str());
    let harness = match roster.seat(word) {
        Ok(agent) => agent,
        Err(agent_protocols::Unseated::Refused(refusal)) => return Err(refusal),
        Err(agent_protocols::Unseated::Unknown) => {
            return Err(
                AxError::failure(AxCode::ConfigInvalid, "dispatch work", subject(word))
                    .with_nearby(roster.words())
                    .with_recovery(format!(
                        "write the id of an `[[agent]]` row or of a built-in entry under \
                         `[resident] harness` in {}",
                        file.display()
                    )),
            );
        }
    };
    if rules.policy().confidential {
        return Err(AxError::failure(
            AxCode::GateDenied,
            "dispatch work",
            subject(harness.entry().id.as_str()),
        )
        .with_recovery(format!(
            "a harness sends the room to its own vendor and a confidential building's data \
             does not leave; take `[resident] harness` out of {}, or drop `confidential = true`",
            file.display()
        )));
    }
    if let Some(model) = asked.model {
        return Err(AxError::failure(
            AxCode::ConfigInvalid,
            "dispatch work",
            subject(&format!(
                "{model} into a room whose resident is {}",
                harness.entry().id.as_str()
            )),
        )
        .with_recovery(format!(
            "dispatch without naming a model, or take `[resident] harness` out of {} to run \
             {model} here",
            file.display()
        )));
    }
    Ok(harness)
}

fn unreadable(path: &std::path::Path, err: &std::io::Error) -> AxError {
    AxError::failure(
        AxCode::StorageFatal,
        "book what a run stands under",
        format!("{}: {err}", path.display()),
    )
    .with_recovery(
        "fix the file's permissions; a dispatch stands a run under these documents and \
         will not guess what they say",
    )
}
