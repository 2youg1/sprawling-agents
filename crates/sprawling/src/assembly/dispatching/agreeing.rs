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

use crate::assembly::CommandDesk;

use super::super::RunWorker;
use super::{Agreed, Assignment};

/// A run's identity, derived rather than drawn: the same job dispatched
/// at the same millisecond to the same address is the same run, and no
/// randomness enters the ledger's identifiers.
/// The identifier is the digest's first sixteen bytes, taken from the
/// hash itself rather than from its printed form: the round trip
/// through hexadecimal had two failure points that both answered zero,
/// so a digest this build could not print became run `00000…`
/// (sprawling-SPEC.md 8-73).
pub(in crate::assembly) fn run_id_for(job: &Locator, addr: &Address, now: TimeMs) -> RunId {
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
pub(crate) fn acp_dispatch(
    desk: &CommandDesk,
    body: &serde_json::Value,
    pairing: channels::Pairing,
) -> Result<channels::AcpProgress, AxError> {
    if matches!(pairing, channels::Pairing::Absent) {
        // The refusal says nothing about whether the address exists, the
        // building is real, or the token was close: an unpaired caller
        // learns one bit. protocol-SPEC.md section 9 gives this
        // judgement to the inbound middleware in `channels`; until that
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
    let protocol::Admitted::Dispatch { addr, task, goal } =
        protocol::admit(protocol::Incoming::parse(body)?)?;
    let idem = kernel::IdemKey::derive(
        &RunId::CITY,
        kernel::Seq::FIRST,
        format!("acp:{}:{task}", addr.as_str()).as_bytes(),
    );
    // An editor gets its answer from this function and then stops
    // listening, so there is no peer left for a later refusal to reach.
    // Saying that in the type beats a silent third meaning of `Reply`.
    desk.post(
        channels::Command::Dispatch {
            addr,
            task,
            goal,
            mode: channels::Mode::PlanGoal,
            idem,
            // An editor drives an address it already chose.
            session: None,
            // ...and says nothing about how hard to think, so the
            // layers above answer.
            effort: None,
            model: None,
        },
        channels::Reply::nowhere(),
    );
    Ok(channels::AcpProgress {
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
    /// Refuses a halted scope, the reserved subtree, rules that will not
    /// load, a tag with no model behind it, an endpoint that is no
    /// longer attached, and a confidential building whose model would
    /// leave this machine.
    pub(super) fn agree_to_work(&mut self, at: &Assignment) -> Result<Agreed, AxError> {
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
        let own = city::own_layer(&self.city_root, addr)?;
        let tag = self.tag_for(at.model.as_deref(), own.model())?;
        let chosen = self.credentials.book.select(tag, rules.policy())?;
        let model = chosen.entry.clone();
        let provider = chosen.endpoint.name.clone();
        let adapter = self.models.build(&chosen, self.redemption()?)?;
        // Every request the run sends goes through the keep-warm door,
        // so a landed run can have its prefix renewed (sprawling-SPEC
        // 8-112); under the default setting the door only forwards.
        let clock = std::sync::Arc::clone(&self.clock);
        let adapter = crate::assembly::keeping_warm::Door::new(
            adapter,
            city::keep_warm(&self.city_root, building.addr())?,
            Box::new(move || clock.now()),
        );
        let retries = chosen.endpoint.tuning.request_max_retries;
        Ok(Agreed {
            building,
            rules,
            model,
            provider,
            adapter,
            retries,
        })
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
    /// never had (sprawling-SPEC.md 8-40).
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
            let bytes = match std::fs::read(&path) {
                Ok(bytes) => bytes,
                Err(err) if err.kind() == std::io::ErrorKind::NotFound => Vec::new(),
                Err(err) => return Err(unreadable(&path, &err)),
            };
            let after = kernel::B3Hash::digest(&bytes);
            let before = self.governance.rules.get(&(scope.clone(), which)).copied();
            if before != Some(after) {
                let changed = RulesChanged {
                    scope,
                    which,
                    before,
                    after,
                    bytes: bytes.len(),
                };
                self.record(EventKind::RulesChanged, Payload::of(&changed)?)?;
            }
        }
        Ok(())
    }

    /// The tag whose registration a dispatch runs on, so the endpoint,
    /// the window and the policy check come with the model.
    ///
    /// A model the dispatch names runs under the tag that registered
    /// it. A dispatch that names none - a successor, a wake knock, a
    /// follow-up without `-m` - continues on the model its room froze
    /// while a tag still registers it, and otherwise runs on `main`:
    /// the session's shape check then refuses the moved model with the
    /// way out a person can take (sprawling-SPEC.md 8-79), which a
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

/// A document a run would stand under that this process cannot read.
fn unreadable(path: &std::path::Path, err: &std::io::Error) -> AxError {
    AxError::failure(
        AxCode::StorageFatal,
        "book what a run stands under",
        format!("{}: {err}", path.display()),
    )
    .with_recovery(
        "fix the file's permissions; a dispatch stands a run under these documents and          will not guess what they say",
    )
}
