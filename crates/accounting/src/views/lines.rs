// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The fold every query is answered from, and the lines a page reads off
//! it.
//!
//! **Why it is a projection and not part of the assembly point.** Nothing
//! here decides anything or reaches a provider: it folds records into the
//! answers a client asks for, and `Views::rebuild` throws the whole thing
//! away and folds the ledger again to get the same bytes. That is
//! ARCHITECTURE.md section 9 shape 7, while `bin::assembly` is an
//! adapter - and a file holding two shapes is what section 9 says a split
//! looks like.
//!
//! **What it deliberately does not hold.** The plans are
//! `crate::plan_view`'s and are read through it; a second parse here
//! would be a second answer to "what is stuck and why", and only one of
//! them would be folding the records that say why. What waits in a room
//! is folded from signal records rather than read off a queue, because a
//! queue answers by being consumed and a view that consumed what it
//! showed would change the thing it reports on.

use std::path::{Path, PathBuf};

use kernel::event::record::{AssetArchived, DiscardRestored, FileDiscarded};
use kernel::{Address, AxError, EventRecord, RunId};

// Where a city keeps its ledger and how a building reads off disk are
// `bin::assembly`'s: it forms the city that laid them out. Borrowed
// rather than copied, so "where the ledger lives" keeps one answer.
/// What one address is governed by, value by value, with the file each
/// value came from.
///
/// The ladder is climbed once, by the module that owns it, and the
/// rung it answers with is carried through rather than re-derived: a
/// page told only the resolved setting would have to read all three
/// files and climb the same ladder a second time, and two climbs of
/// one ladder are two answers to one question.
///
/// The figures an endpoint nobody tuned is called with are read out of
/// `gateway::EndpointTuning::DEFAULTS`, which is their one home; a
/// form that printed its own numbers into empty boxes is what this
/// answer exists to retire.
pub(crate) fn config_answer(
    city_root: &Path,
    addr: &Address,
) -> Result<wire::ConfigAnswer, AxError> {
    let defaults = gateway::EndpointTuning::DEFAULTS;
    let domain = wire::SecondDomain {
        min: kernel::consts_policy::CTX_REMINDER_SECOND_MIN,
        max: kernel::consts_policy::CTX_REMINDER_SECOND_MAX,
    };
    Ok(wire::ConfigAnswer {
        addr: addr.clone(),
        effort: city::settled_effort(city_root, addr)?.map(|(effort, layer)| wire::SettledEffort {
            effort,
            from: rung_of(layer),
        }),
        second: city::settled_second(city_root, addr)?.map_or(
            wire::SettledSecond {
                percent: kernel::consts_policy::CTX_REMINDER_SECOND_DEFAULT,
                from: wire::ConfigLayer::Default,
                domain,
            },
            |(threshold, layer)| wire::SettledSecond {
                percent: u64::from(threshold),
                from: rung_of(layer),
                domain,
            },
        ),
        first: None,
        tuning: wire::TuningDefaults {
            from: wire::ConfigLayer::Default,
            timeout_ms: defaults.timeout_ms,
            request_max_retries: defaults.retries.stated(),
            stream_idle_timeout_ms: defaults.stream_idle_timeout_ms,
            proxying: kernel::Proxying::default(),
        },
    })
}

/// The one place the ladder's rung becomes the wire's. Exhaustive, so
/// a rung added to the ladder is a compiler error here rather than a
/// page that silently reports the wrong file.
fn rung_of(layer: city::Layer) -> wire::ConfigLayer {
    match layer {
        city::Layer::City => wire::ConfigLayer::City,
        city::Layer::Building => wire::ConfigLayer::Building,
        city::Layer::Resident => wire::ConfigLayer::Resident,
    }
}

/// The settings page's read of the endpoint book.
pub(crate) fn endpoints_answer(book: &gateway::EndpointBook) -> wire::EndpointsAnswer {
    let endpoints = book
        .endpoints()
        .map(|endpoint| wire::EndpointSummary {
            name: endpoint.name.clone(),
            label: endpoint.label().to_owned(),
            base_url: endpoint.base_url.clone(),
            dialect: endpoint.dialect,
            connection_kind: endpoint.connection_kind.as_str().to_owned(),
            models: endpoint
                .models
                .iter()
                .map(|row| wire::ModelFactsSummary {
                    id: row.id.clone(),
                    context_tokens: row.context_tokens.and_then(kernel::Window::new),
                    max_output_tokens: row.max_output_tokens,
                    input_modalities: row.input_modalities.clone(),
                    input_price: row.input_price.clone(),
                    output_price: row.output_price.clone(),
                })
                .collect(),
            local: endpoint.is_local(),
            has_credential: endpoint.has_credential(),
        })
        .collect();
    let chosen = book
        .choices()
        .map(|(tag, endpoint, entry)| wire::ChosenSummary {
            tag,
            endpoint: endpoint.to_owned(),
            model: entry.id.clone(),
            max_output_tokens: entry.max_output_tokens,
        })
        .collect();
    wire::EndpointsAnswer { endpoints, chosen }
}

/// The vendors this city knows by host, copied row for row from the
/// preset table (`crates/wire/Spec.lean` §8-51). A row the normaliser refuses
/// is a defect of the table, and the answer names itself unavailable
/// with the row's refusal rather than listing the rest as if whole.
pub(crate) fn known_hosts_answer() -> wire::Answer {
    let hosts = match gateway::known_hosts() {
        Ok(hosts) => hosts,
        Err(fault) => {
            return wire::Answer::Unavailable {
                query: format!("KnownHosts({})", fault.subject()),
            };
        }
    };
    wire::Answer::KnownHosts(wire::KnownHostsAnswer {
        hosts: hosts
            .into_iter()
            .map(|row| wire::KnownHost {
                host: row.host.to_owned(),
                faces: row
                    .faces
                    .into_iter()
                    .map(|(dialect, base_url)| wire::KnownFace { dialect, base_url })
                    .collect(),
            })
            .collect(),
    })
}

/// The harness page: every official harness in the roster, the command
/// that starts it, and whether the search the served city handed in
/// finds that command's program (`crates/wire/Spec.lean` §8-52, accounting-SPEC.md
/// 8-10).
pub(crate) fn harnesses_answer(find: fn(&str) -> Option<PathBuf>) -> wire::Answer {
    wire::Answer::Harnesses(wire::HarnessesAnswer {
        harnesses: agent_protocols::Harness::ALL
            .iter()
            .map(|harness| {
                let launch = harness.launch();
                wire::HarnessLine {
                    name: harness.as_str().to_owned(),
                    launch: std::iter::once(launch.program.name())
                        .chain(launch.args.iter().copied())
                        .map(str::to_owned)
                        .collect(),
                    found: find(launch.program.name()).is_some(),
                    docs: harness.docs().to_owned(),
                }
            })
            .collect(),
    })
}

/// The building a `pursuit_changed` record is about.
///
/// # Errors
/// Refuses a record with no address: the step it records belongs to no
/// building, and a fold that skipped it would keep whatever goal the
/// step changed.
pub fn pursued(record: &EventRecord) -> Result<Address, AxError> {
    record.addr().cloned().ok_or_else(|| {
        AxError::failure(
            kernel::AxCode::WireMismatch,
            "read a pursuit_changed line",
            format!("line {} names no building", record.seq().value()),
        )
        .with_recovery("replay with the build that wrote this record")
    })
}

/// Every building the city has, in reading order.
///
/// The plans themselves are `crate::plan_view`'s: reading them here as
/// well would be a second parse of the same file, and the two would
/// disagree the first time one of them was invalidated and the other was
/// not.
pub(crate) fn buildings_of(city_root: &Path) -> Vec<Address> {
    let mut found = city::buildings(city_root).unwrap_or_default();
    found.sort_by(|left, right| left.as_str().cmp(right.as_str()));
    found
}

/// One signal, as a room's queue would show it. `None` for a record
/// this version cannot read as a signal: a view skips what it cannot
/// read rather than inventing a row for it.
/// The waiting row a `signal_enqueued` line adds, and the room it waits
/// in, read through the struct its writer wrote.
///
/// # Errors
/// Refuses a line this build cannot read as a signal: skipping it would
/// leave a waiting signal out of the view.
pub(crate) fn signal_line(record: &EventRecord) -> Result<(Address, wire::SignalLine), AxError> {
    let signal = collab::Signal::from_payload(record.data())?;
    Ok((
        signal.room().clone(),
        wire::SignalLine {
            id: signal.id().as_str().to_owned(),
            kind: signal.kind().as_str().to_owned(),
            from: signal.from().to_owned(),
            at: record.t(),
        },
    ))
}

/// The rows one `file_discarded` record states: one per path, each with
/// the record's way back. A record this version cannot read states no
/// rows, for the reason [`signal_line`] gives.
pub(crate) fn discard_lines(record: &EventRecord) -> Vec<wire::DiscardLine> {
    let Ok(FileDiscarded { paths, restoration }) = record.data().read() else {
        return Vec::new();
    };
    paths
        .into_iter()
        .map(|path| wire::DiscardLine {
            path,
            restoration: restoration.clone(),
            at: record.t(),
            restored: false,
        })
        .collect()
}

/// The paths one `discard_restored` record put back; none when this
/// version cannot read it.
pub(crate) fn restored_paths(record: &EventRecord) -> Vec<String> {
    record
        .data()
        .read::<DiscardRestored>()
        .map_or_else(|_| Vec::new(), |restored| restored.paths)
}

/// One shelf entry, as the registry shows it. `None` for a record with
/// no room or one this version cannot read.
pub(crate) fn registry_line(record: &EventRecord) -> Option<wire::RegistryLine> {
    let AssetArchived { kind, subject, .. } = record.data().read().ok()?;
    Some(wire::RegistryLine {
        addr: record.addr().cloned()?,
        kind,
        subject,
        at: record.t(),
    })
}

pub(crate) fn summarize(run: RunId, hot: &storage::RunHot) -> wire::RunSummary {
    wire::RunSummary {
        run,
        who: hot.who.clone(),
        frozen: matches!(hot.phase, storage::RunPhase::Frozen),
        last_seq: hot.last_seq,
        last_kind: hot.last_kind,
        addr: hot.addr.clone(),
        started: hot.started,
        completion: hot.completion.clone(),
        pr: hot.pr.clone(),
        ask: hot.ask.clone(),
        task: hot.task.clone(),
        goal: hot.goal.clone(),
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, reason = "test code")]
mod tests {
    use super::config_answer;
    use kernel::Address;

    /// A value no file states is still answered, with the default named
    /// as the layer it came from and the domain a file may state; a page
    /// told `null` has to keep its own copy of both to draw anything.
    #[test]
    fn an_unstated_second_rung_is_answered_with_the_default_as_its_layer() {
        let dir = tempfile::tempdir().unwrap();
        let addr = Address::parse("lab/room1").unwrap();
        let answer = serde_json::to_value(config_answer(dir.path(), &addr).unwrap()).unwrap();
        assert_eq!(
            answer.get("second"),
            Some(&serde_json::json!({
                "percent": kernel::consts_policy::CTX_REMINDER_SECOND_DEFAULT,
                "from": "default",
                "domain": {
                    "min": kernel::consts_policy::CTX_REMINDER_SECOND_MIN,
                    "max": kernel::consts_policy::CTX_REMINDER_SECOND_MAX,
                },
            }))
        );
    }

    /// Both rungs the context ring draws travel on the one answer it
    /// reads; the page keeps no copy of the first (`crates/wire/Spec.lean`
    /// §8-77).
    #[test]
    fn the_first_rung_is_answered_beside_the_second() {
        let dir = tempfile::tempdir().unwrap();
        let addr = Address::parse("lab/room1").unwrap();
        let answer = config_answer(dir.path(), &addr).unwrap();
        assert_eq!(
            answer.first,
            Some(kernel::consts_policy::CTX_REMINDER_FIRST_PERCENT)
        );
    }

    /// The figures an untuned endpoint is called with name their layer
    /// too: no file on the ladder states them, so the answer says they
    /// are this build's own rather than leaving the page to guess.
    #[test]
    fn the_tuning_defaults_are_answered_with_the_default_as_their_layer() {
        let dir = tempfile::tempdir().unwrap();
        let addr = Address::parse("lab/room1").unwrap();
        let answer = serde_json::to_value(config_answer(dir.path(), &addr).unwrap()).unwrap();
        assert_eq!(
            answer.pointer("/tuning/from"),
            Some(&serde_json::json!("default"))
        );
    }
}
