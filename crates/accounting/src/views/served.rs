// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! What a served city hands its views that the ledger cannot.
//!
//! Everything else the views hold is folded from records, and a rebuild
//! reproduces it byte for byte. These two are facts about the running
//! process rather than about the history - what this machine had when
//! the city started, and the vault the worker opened - so they arrive
//! from outside, once, and a rebuild leaves them alone.
//!
//! They are here rather than in `views::holding` because that module's
//! subject is what a fold holds; a setter that no fold can reach is a
//! different responsibility, and one file holding both is what makes
//! the distinction easy to lose.

use super::holding::Views;

/// The ways past the history a served city lets its views reach: each a
/// read of this machine or the network, handed in once by the served city
/// and copied whole into a twin. Each is `None` in views nobody served,
/// which then answer `Unavailable` rather than reaching out.
#[derive(Debug, Clone, Copy, Default)]
pub(super) struct Reach {
    /// Asks the registry which release is newest.
    pub(super) registry: Option<fn() -> wire::ReleaseAnswer>,
    /// Asks one item's publisher for its newest release.
    pub(super) upstream: Option<fn(&str) -> wire::DoctorUpstream>,
    /// Asks this machine's search path for one program.
    pub(super) programs: Option<fn(&str) -> Option<std::path::PathBuf>>,
    /// Asks the GitHub CLI which login one host is signed in as.
    pub(super) github: Option<fn(&str) -> wire::GithubReading>,
}

impl Views {
    /// Takes what the doctor found, so a page can be told what this
    /// machine is missing.
    ///
    /// Probing is seconds of starting programs, and a read that did it
    /// would hold the one thread every other read is answered on.
    pub fn found_on_this_machine(&mut self, report: wire::DoctorAnswer) {
        self.machine = Some(report);
    }

    /// Takes the vault handle the worker opened, for the one read that
    /// needs a credential: reaching a tool server whose header or
    /// environment names one.
    ///
    /// Lent rather than opened here, for the reason the transcription
    /// door is lent it: a second handle on the same secrets would be a
    /// second door onto them.
    pub fn lend_the_vault(&mut self, vault: std::sync::Arc<std::sync::Mutex<gateway::Custodian>>) {
        self.vault = Some(vault);
    }

    /// Takes the one way this city asks the registry which release is
    /// newest, so a `NewestRelease` query reaches the network only
    /// through what the served city handed in.
    pub fn ask_the_registry_through(&mut self, newest: fn() -> wire::ReleaseAnswer) {
        self.reach.registry = Some(newest);
    }

    /// Takes the one way this city asks an item's publisher for its
    /// newest release, so an `UpstreamVersion` query leaves this machine
    /// only through what the served city handed in (`crates/sprawling/Spec.lean`
    /// §8-120).
    pub fn ask_upstream_through(&mut self, newest: fn(&str) -> wire::DoctorUpstream) {
        self.reach.upstream = Some(newest);
    }

    /// Takes the one way this city asks its search path for a program,
    /// so the harness page reads this machine only through what the
    /// served city handed in (`crates/accounting/spec/Views.lean` §8-10).
    pub fn find_programs_through(&mut self, find: fn(&str) -> Option<std::path::PathBuf>) {
        self.reach.programs = Some(find);
    }

    /// Takes the one way this city asks the GitHub CLI on this machine for
    /// the login a host is signed in as, so a `GithubLogin` query starts a
    /// program only through what the served city handed in
    /// (`crates/accounting/spec/Views/Answering/Github.lean` §8-18-3).
    pub fn ask_github_through(&mut self, login: fn(&str) -> wire::GithubReading) {
        self.reach.github = Some(login);
    }

    /// Takes the halt the served city's writer is held by until the proof
    /// of its history has a verdict, so the city page says whether the
    /// history is proved from the same verdict the writer obeys
    /// (`crates/sprawling/Spec.lean` §8-134).
    pub fn watch_proof(&mut self, halt: storage::ChainHalt) {
        self.proof = Some(halt);
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::panic, reason = "test code")]
mod tests {
    use super::Views;
    use kernel::{AxCode, AxError};

    /// What the scripted registry says, so the answer shows whose it is.
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

    /// How many programs the scripted search was asked for.
    static ASKED: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

    /// A search that finds every program, and counts the asking.
    fn found_everywhere(program: &str) -> Option<std::path::PathBuf> {
        ASKED.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        Some(std::path::PathBuf::from(program))
    }

    #[test]
    fn a_harness_is_looked_for_through_the_search_the_views_were_handed() {
        let dir = tempfile::tempdir().unwrap();
        let mut views = Views::new(dir.path());
        views.find_programs_through(found_everywhere);

        let wire::Answer::Harnesses(page) = views.prepare(&wire::Query::Harnesses).finish() else {
            panic!("Harnesses answers with the harness page");
        };
        let found: Vec<wire::HarnessState> =
            page.harnesses.into_iter().map(|line| line.state).collect();
        assert_eq!(
            (found, ASKED.load(std::sync::atomic::Ordering::Relaxed)),
            (
                vec![
                    wire::HarnessState::NotSetUp { looked: Vec::new() };
                    agent_protocols::Harness::ALL.len()
                ],
                agent_protocols::Harness::ALL.len()
            ),
        );
    }

    #[test]
    fn a_harness_page_nobody_served_answers_unavailable() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(
            Views::new(dir.path())
                .prepare(&wire::Query::Harnesses)
                .finish(),
            wire::Answer::Unavailable {
                query: "Harnesses".to_owned()
            },
        );
    }
}
