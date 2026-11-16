// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The agent offers the ACP page was answered, and the consent that
//! writes one of them into the city (`crates/accounting/spec/Views.lean`,
//! `crates/sprawling/spec/Serving.lean`).
//!
//! `AddAgent` carries only the digest of the launch the card showed. A
//! registry entry is taken again from the shipped snapshot, but a pasted
//! text and this machine's evidence are not in the command, so the views
//! remember the detected and pasted entries they answered, and the
//! consent takes the one whose digest the person pressed.

use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use agent_protocols::{AgentEntry, AgentSource, Catalog, Launch};
use kernel::{AxCode, AxError, B3Hash};

/// How many answered offers are remembered before the oldest is dropped:
/// more than one page shows at once, few enough that a stale card is
/// refused rather than kept for the life of the process.
const KEPT: usize = 64;

/// The detected and pasted offers answered most recently, shared by the
/// views that answer them and the listener that takes the consent.
#[derive(Clone, Default)]
pub struct Offered(Arc<Mutex<VecDeque<AgentEntry>>>);

impl std::fmt::Debug for Offered {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Offered").finish_non_exhaustive()
    }
}

impl Offered {
    /// Keeps `entries` as offered, each once, the newest last.
    pub(crate) fn remember<'e>(&self, entries: impl IntoIterator<Item = &'e AgentEntry>) {
        let mut held = self.held();
        for entry in entries {
            let digest = entry.launch.digest();
            held.retain(|kept| kept.launch.digest() != digest);
            held.push_back(entry.clone());
            while held.len() > KEPT {
                held.pop_front();
            }
        }
    }

    /// Writes the offer whose launch digest is `adding.spec_digest` into
    /// the city at `city_root`, its program resolved through `find`, and
    /// seats it in `adding.seat_here` when that is present.
    ///
    /// # Errors
    /// `E_CONFIG_INVALID` when no offer of that source has that digest -
    /// the card changed, or the city forgot it; `E_TOOL_UNAVAILABLE` when
    /// the program is not on this machine's search path; and what
    /// `city::write_agent` and `city::seat_agent` refuse.
    pub fn consent(
        &self,
        city_root: &Path,
        adding: &wire::AgentAdding,
        find: fn(&str) -> Option<PathBuf>,
    ) -> Result<(), AxError> {
        let entry = self.offer(&adding.spec_digest, adding.source)?;
        let launch = resolved(entry.launch, find)?;
        let row = city::AgentRow {
            id: entry.id.as_str().to_owned(),
            name: Some(entry.name),
            command: launch.program.clone(),
            args: launch.args.clone(),
            env: launch.env.clone(),
            source: match entry.source {
                AgentSource::Registry => city::AgentRowSource::Registry,
                AgentSource::Detected => city::AgentRowSource::Detected,
                AgentSource::Pasted => city::AgentRowSource::Pasted,
            },
            version: entry.version,
            launch_digest: Some(launch.digest()),
        };
        city::write_agent(city_root, &row)?;
        match &adding.seat_here {
            Some(room) => city::seat_agent(city_root, room, &row.id),
            None => Ok(()),
        }
    }

    /// The offer of `source` whose launch has `digest`.
    fn offer(&self, digest: &B3Hash, source: wire::AgentSource) -> Result<AgentEntry, AxError> {
        let found = match source {
            wire::AgentSource::Registry => Catalog::bundled()?
                .entries
                .into_iter()
                .find(|entry| entry.launch.digest() == *digest),
            wire::AgentSource::Detected => self.remembered(AgentSource::Detected, digest),
            wire::AgentSource::Pasted => self.remembered(AgentSource::Pasted, digest),
        };
        found.ok_or_else(|| {
            AxError::failure(
                AxCode::ConfigInvalid,
                "add an ACP agent",
                format!("no offer the city made has the launch {digest}"),
            )
            .with_recovery(
                "open the ACP page again and add the agent from the card it shows now; the card \
                 changed, or the city was restarted since it was shown",
            )
        })
    }

    /// The remembered offer of `source` whose launch has `digest`.
    fn remembered(&self, source: AgentSource, digest: &B3Hash) -> Option<AgentEntry> {
        self.held()
            .iter()
            .find(|entry| entry.source == source && entry.launch.digest() == *digest)
            .cloned()
    }

    /// The memory, taken back from a thread that panicked while holding
    /// it: what it holds is whole entries, so the worst a panic leaves is
    /// one offer fewer.
    fn held(&self) -> MutexGuard<'_, VecDeque<AgentEntry>> {
        self.0.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

/// `launch` with its program as an absolute path, so a program put on
/// the search path later cannot stand in for the one consented to.
fn resolved(launch: Launch, find: fn(&str) -> Option<PathBuf>) -> Result<Launch, AxError> {
    if Path::new(&launch.program).is_absolute() {
        return Ok(launch);
    }
    let Some(found) = find(&launch.program) else {
        return Err(AxError::failure(
            AxCode::ToolUnavailable,
            "add an ACP agent",
            format!("`{}` is not on this machine's search path", launch.program),
        )
        .with_recovery(
            "install it, or put the folder that holds it on PATH, then add the agent again",
        ));
    };
    let program = found.into_os_string().into_string().map_err(|unspelled| {
        AxError::failure(
            AxCode::ConfigInvalid,
            "add an ACP agent",
            format!("{} is not a path CONFIG.toml can hold", unspelled.display()),
        )
        .with_recovery("install the program under a folder whose name is plain text")
    })?;
    Ok(Launch { program, ..launch })
}
