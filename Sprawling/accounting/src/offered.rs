// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use agent_protocols::{AgentEntry, AgentSource, Catalog, Launch};
use kernel::{AxCode, AxError, B3Hash};

const KEPT: usize = 64;

#[derive(Clone, Default)]
pub struct Offered(Arc<Mutex<VecDeque<AgentEntry>>>);

impl std::fmt::Debug for Offered {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Offered").finish_non_exhaustive()
    }
}

impl Offered {

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

    fn remembered(&self, source: AgentSource, digest: &B3Hash) -> Option<AgentEntry> {
        self.held()
            .iter()
            .find(|entry| entry.source == source && entry.launch.digest() == *digest)
            .cloned()
    }

    fn held(&self) -> MutexGuard<'_, VecDeque<AgentEntry>> {
        self.0.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

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
