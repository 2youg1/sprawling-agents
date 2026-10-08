// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Taking on work: the run id a dispatch derives, the ACP request
//! turned into that dispatch, and `agree_to_work`, which decides whether
//! the city may take it before anything is written.

use kernel::event::Scope;
use kernel::event::record::{GoverningDocument, RulesChanged};
use kernel::{Address, AxCode, AxError, EventKind, Payload};
use kernel::{Locator, RunId, TimeMs};

use crate::worker::CommandDesk;

use super::super::RunWorker;
use super::{Agreed, Assignment, HarnessSeat, Seat};

/// A run's identity, derived rather than drawn: the same job dispatched
/// at the same millisecond to the same address is the same run, and no
/// randomness enters the ledger's identifiers.
/// The identifier is the digest's first sixteen bytes, taken from the
/// hash itself rather than from its printed form: the round trip
/// through hexadecimal had two failure points that both answered zero,
/// so a digest this build could not print became run `00000…`
/// (`crates/sprawling/Spec.lean` §8-73).
pub(in crate::worker) fn run_id_for(job: &Locator, addr: &Address, now: TimeMs) -> RunId {
    let seed = format!("{job}|{}|{}", addr.as_str(), now.value());
    let digest = kernel::B3Hash::digest(seed.as_bytes());
    let mut bytes = [0u8; 16];
    for (slot, byte) in bytes.iter_mut().zip(digest.as_bytes()) {
        *slot = *byte;
    }
    RunId::from_bytes(bytes)
}

/// An outside editor's request, turned into the city's usual dispatch.
///
/// Not a second control surface: the admission decides what a stranger
/// may learn, and everything after it is the path a person's dispatch
/// takes. The run identifier is minted when the worker takes the work,
/// so what an editor is told now is the honest thing - accepted, and
/// nothing finished yet.
pub fn acp_dispatch(
    desk: &CommandDesk,
    body: &serde_json::Value,
    pairing: wire::Pairing,
) -> Result<wire::AcpProgress, AxError> {
    if matches!(pairing, wire::Pairing::Absent) {
        // The refusal says nothing about whether the address exists, the
        // building is real, or the token was close: an unpaired caller
        // learns one bit. `crates/agent_protocols/Spec.lean` section 9 gives this
        // judgement to the inbound middleware in `wire`; until that
        // middleware refuses on the route, the door the request already
        // reached is the one place that can.
        return Err(AxError::failure(
            AxCode::GateDenied,
            "admit an external request",
            "not paired with this city".to_owned(),
        )
        .with_recovery("pair the client from the settings page, then send the token it shows"));
    }
    // The body travels as the JSON it arrived as: `Incoming::parse` is
    // the only constructor the inbound grammar has, and a struct read
    // here first would be a second reading of the same four keys.
    let agent_protocols::Admitted::Dispatch { addr, task, goal } =
        agent_protocols::admit(agent_protocols::Incoming::parse(body)?)?;
    let idem = kernel::IdemKey::derive(
        &RunId::CITY,
        kernel::Seq::FIRST,
        format!("acp:{}:{task}", addr.as_str()).as_bytes(),
    );
    // An editor gets its answer from this function and then stops
    // listening, so there is no peer left for a later refusal to reach.
    // Saying that in the type beats a silent third meaning of `Reply`.
    desk.post(
        wire::Command::Dispatch {
            addr,
            task,
            goal,
            policy: wire::RunPolicy::of(wire::Mode::Work),
            idem,
            // An editor drives an address it already chose.
            session: None,
            // ...and says nothing about how hard to think, so the
            // layers above answer.
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
    /// Every refusal this dispatch can owe before it costs anything.
    ///
    /// **Nothing here writes, and that is the whole of the phase.** A
    /// halted city that laid a job file down would leave a task in a
    /// room no run ever opened, and so would a city with no model behind
    /// the tag; the two are one rule, so they are answered in one place
    /// and the first thing written happens on the other side of it.
    ///
    /// The order of the judgements is the order a caller is entitled to
    /// them. A halted city answers "halted" rather than "no model is
    /// chosen", because the halt is the one a person can lift and the
    /// recovery line is the third part of what an `AxError` promises.
    ///
    /// # Errors
    /// Refuses a halted scope, the reserved subtree, and rules that will
    /// not load; for a room whose resident is a harness, a spelling that
    /// names none of the five, a confidential building and a dispatch
    /// that names a model; otherwise a tag with no model behind it, an
    /// endpoint that is no longer attached, and a confidential building
    /// whose model would leave this machine.
    pub(super) fn agree_to_work(&mut self, at: &Assignment) -> Result<Seat, AxError> {
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
        // The building's own rules decide which models this run may
        // reach, so they are read before one is chosen.
        let building = city::Building::of(addr)?;
        let rules = city::load(&self.city_root, building.addr())?;
        // Read before a model is chosen: a room a harness runs needs
        // none. A dispatch that opens a room is judged at its building,
        // because the new room's own layer is empty and the address's
        // own session record belongs to another room.
        let judged = if super::session::opens_a_room(addr, at.session.as_ref()) {
            building.addr()
        } else {
            addr
        };
        if let Some((word, layer)) = city::settled_harness(&self.city_root, judged)? {
            let file = city::config_path(&self.city_root, judged, layer)?;
            let roster = crate::roster::roster(&self.city_root)?;
            let named = Named {
                word: &word,
                file: &file,
            };
            let harness = seated_harness(at, named, &rules, &roster)?;
            return Ok(Seat::Harness(HarnessSeat {
                building,
                rules,
                harness,
            }));
        }
        let own = city::own_layer(&self.city_root, addr)?;
        let tag = self.tag_for(at.model.as_deref(), own.model())?;
        let chosen = self.credentials.book.select(tag, rules.policy())?;
        let model = chosen.entry.clone();
        let provider = chosen.endpoint.name.clone();
        let adapter = self.models.build(&chosen, self.redemption()?)?;
        // Every request the run sends goes through the keep-warm door,
        // so a landed run can have its prefix renewed (`crates/sprawling/Spec.lean`
        // §8-112); under the default setting the door only forwards.
        let clock = std::sync::Arc::clone(&self.clock);
        let adapter = crate::worker::keeping_warm::Door::new(
            adapter,
            city::keep_warm(&self.city_root, building.addr())?,
            Box::new(move || clock.now()),
        );
        let retries = chosen.endpoint.tuning.request_max_retries;
        Ok(Seat::Model(Agreed {
            building,
            rules,
            model,
            provider,
            adapter,
            retries,
        }))
    }

    /// Books what a run is about to stand under: the city's
    /// `CONFIG.toml` and, when the building exists, its own
    /// `CONFIG.toml` and `RULES.toml`.
    ///
    /// Each document is compared with what the governance fold last
    /// booked, and one that moved gets a `rules_changed` line before the
    /// run starts, so one line's `after` is the next line's `before` for
    /// as long as nothing wrote the file in between. An absent file is
    /// booked as zero bytes, the same reading `city::load` gives it. A
    /// building that does not exist opens no account: a mistyped name
    /// would otherwise leave the history holding a building the city
    /// never had (`crates/sprawling/Spec.lean` §8-40).
    ///
    /// # Errors
    /// `E_STORAGE_FATAL` for a document or a building root that cannot
    /// be read; propagates a history that will not take the line.
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

    /// Books one governing document: compared with what the governance
    /// fold last booked for it, and a `rules_changed` line when it moved.
    ///
    /// The one place such a line is written, so a dispatch that finds a
    /// document moved and a page that has just written it book the change
    /// in one shape, and the chain of `before` and `after` stays unbroken.
    ///
    /// # Errors
    /// `E_STORAGE_FATAL` for a document that cannot be read; propagates a
    /// history that will not take the line.
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

    /// The tag whose registration a dispatch runs on, so the endpoint,
    /// the window and the policy check come with the model.
    ///
    /// A model the dispatch names runs under the tag that registered
    /// it. A dispatch that names none - a successor, a wake knock, a
    /// follow-up without `-m` - continues on the model its room froze
    /// while a tag still registers it, and otherwise runs on `main`:
    /// the session's shape check then refuses the moved model with the
    /// way out a person can take (`crates/sprawling/Spec.lean` §8-79), which a
    /// refusal here could not name.
    ///
    /// # Errors
    /// Refuses a named id no tag registered, before anything is written.
    fn tag_for(
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

    /// The tag the model `id` is registered under, if any is.
    fn tag_registering(&self, id: &str) -> Option<kernel::ModelTag> {
        self.credentials
            .book
            .choices()
            .find_map(|(tag, _, entry)| (entry.id == id).then_some(tag))
    }
}

/// The word a room's configuration names its resident by, and the layer
/// file that wrote it, which is where a person changes it.
struct Named<'a> {
    word: &'a str,
    file: &'a std::path::Path,
}

/// The agent a room's configuration names, or the refusal a dispatch to
/// it is owed before anything is written: a word that names no agent, a
/// row changed after its consent, a confidential building, and a dispatch
/// that named a model (`crates/sprawling/Spec.lean` §8-124).
fn seated_harness(
    at: &Assignment,
    named: Named<'_>,
    rules: &city::BuildingRules,
    roster: &agent_protocols::Roster,
) -> Result<agent_protocols::Consented, AxError> {
    let Named { word, file } = named;
    let subject = |named: &str| format!("{}: {named}", at.addr.as_str());
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
    if let Some(model) = at.model.as_deref() {
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

/// A document a run would stand under that this process cannot read.
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
