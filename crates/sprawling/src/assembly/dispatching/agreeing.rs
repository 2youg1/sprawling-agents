// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Who gets woken, and where the work lands.

use kernel::{Address, AxCode, AxError};
use kernel::{Locator, RunId, TimeMs};

use crate::serving::CommandDesk;

use super::super::RunWorker;
use super::Agreed;
use crate::assembly::credentials::dialect_headers;

/// A run's identity, derived rather than drawn: the same job dispatched
/// at the same millisecond to the same address is the same run, and no
/// randomness enters the ledger's identifiers.
pub(in crate::assembly) fn run_id_for(job: &Locator, addr: &Address, now: TimeMs) -> RunId {
    let seed = format!("{job}|{}|{}", addr.as_str(), now.value());
    let digest = kernel::B3Hash::digest(seed.as_bytes()).to_string();
    let mut bytes = [0u8; 16];
    for (slot, pair) in bytes.iter_mut().zip(digest.as_bytes().chunks_exact(2)) {
        let hex = std::str::from_utf8(pair).unwrap_or("00");
        *slot = u8::from_str_radix(hex, 16).unwrap_or(0);
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
    body: channels::AcpBody,
    authentic: bool,
) -> Result<channels::AcpProgress, AxError> {
    let incoming = protocol::Incoming {
        token: body.token,
        addr: Address::parse(&body.addr)?,
        task: body.task,
        goal: body.goal,
    };
    let protocol::Admitted::Dispatch { addr, task, goal } = protocol::admit(&incoming, authentic)?;
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
            mode: channels::ModeTag::parse("plan")?,
            budget: kernel::BudgetCap::default(),
            idem,
            // An editor drives an address it already chose.
            session: None,
            // ...and says nothing about how hard to think, so the
            // layers above answer.
            effort: None,
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
    /// longer attached, a confidential building whose model would leave
    /// this machine, and a subscription credential that will not renew.
    pub(super) fn agree_to_work(&mut self, addr: &Address) -> Result<Agreed, AxError> {
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
        let chosen = self.book.select(kernel::ModelTag::Main, rules.policy())?;
        // A subscription credential that expires mid-run is a run that
        // dies on its second turn, so it is renewed before the run
        // starts rather than after a call comes back refused. The
        // endpoint a login attached carries the provider's own name.
        self.renew_if_stale(&chosen.endpoint.name.clone())?;
        let chosen = self.book.select(kernel::ModelTag::Main, rules.policy())?;
        let model = chosen.entry.clone();
        let adapter = gateway::adapter_for(
            &chosen,
            self.resolver(),
            dialect_headers(chosen.endpoint.dialect),
        )?;
        Ok(Agreed {
            building,
            rules,
            model,
            adapter,
        })
    }
}
