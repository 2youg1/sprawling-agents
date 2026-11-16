// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use super::holding::Views;

#[derive(Debug, Clone, Copy, Default)]
pub(super) struct Reach {
    pub(super) registry: Option<fn() -> wire::ReleaseAnswer>,

    pub(super) upstream: Option<fn(&str) -> wire::DoctorUpstream>,

    pub(super) places: Option<fn(&agent_protocols::SetUpDir) -> Option<std::path::PathBuf>>,

    pub(super) github: Option<fn(&str) -> wire::GithubReading>,
}

impl Views {
    pub fn found_on_this_machine(&mut self, report: wire::DoctorAnswer) {
        self.machine = Some(report);
    }

    pub fn lend_the_vault(&mut self, vault: std::sync::Arc<std::sync::Mutex<gateway::Custodian>>) {
        self.vault = Some(vault);
    }

    pub fn ask_the_registry_through(&mut self, newest: fn() -> wire::ReleaseAnswer) {
        self.reach.registry = Some(newest);
    }

    pub fn ask_upstream_through(&mut self, newest: fn(&str) -> wire::DoctorUpstream) {
        self.reach.upstream = Some(newest);
    }

    pub fn look_for_agents_through(
        &mut self,
        place: fn(&agent_protocols::SetUpDir) -> Option<std::path::PathBuf>,
    ) {
        self.reach.places = Some(place);
    }

    pub fn remember_offers_in(&mut self, offered: crate::offered::Offered) {
        self.offered = Some(offered);
    }

    pub fn ask_github_through(&mut self, login: fn(&str) -> wire::GithubReading) {
        self.reach.github = Some(login);
    }

    pub fn watch_proof(&mut self, halt: storage::ChainHalt) {
        self.proof = Some(halt);
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::panic, reason = "test code")]
mod tests {
    use super::Views;
    use kernel::{AxCode, AxError};

    fn scripted() -> wire::ReleaseAnswer {
        wire::ReleaseAnswer::Refused {
            refusal: AxError::failure(AxCode::ToolUnavailable, "ask the registry", "scripted")
                .with_recovery("nothing: this registry is a script"),
        }
    }

    #[test]
    fn a_newest_release_is_asked_of_the_registry_the_views_were_handed() {
        let dir = tempfile::tempdir().unwrap();
        let mut views = Views::new(dir.path());
        views.ask_the_registry_through(scripted);

        assert_eq!(
            views.prepare(&wire::Query::NewestRelease).finish(),
            wire::Answer::Release(Box::new(scripted()))
        );
    }
}
