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
//! `accounting::plan_view`'s and are read through it; a second parse here
//! would be a second answer to "what is stuck and why", and only one of
//! them would be folding the records that say why. What waits in a room
//! is folded from signal records rather than read off a queue, because a
//! queue answers by being consumed and a view that consumed what it
//! showed would change the thing it reports on.

use std::path::Path;

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
) -> Result<channels::ConfigAnswer, kernel::AxError> {
    let defaults = gateway::EndpointTuning::DEFAULTS;
    let domain = channels::SecondDomain {
        min: kernel::consts_policy::CTX_REMINDER_SECOND_MIN,
        max: kernel::consts_policy::CTX_REMINDER_SECOND_MAX,
    };
    Ok(channels::ConfigAnswer {
        addr: addr.clone(),
        effort: city::settled_effort(city_root, addr)?.map(|(effort, layer)| {
            channels::SettledEffort {
                effort,
                from: rung_of(layer),
            }
        }),
        second: city::settled_second(city_root, addr)?.map_or(
            channels::SettledSecond {
                percent: kernel::consts_policy::CTX_REMINDER_SECOND_DEFAULT,
                from: channels::ConfigLayer::Default,
                domain,
            },
            |(threshold, layer)| channels::SettledSecond {
                percent: u64::from(threshold),
                from: rung_of(layer),
                domain,
            },
        ),
        tuning: channels::TuningDefaults {
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
fn rung_of(layer: city::Layer) -> channels::ConfigLayer {
    match layer {
        city::Layer::City => channels::ConfigLayer::City,
        city::Layer::Building => channels::ConfigLayer::Building,
        city::Layer::Resident => channels::ConfigLayer::Resident,
    }
}

/// The settings page's read of the endpoint book.
pub(crate) fn endpoints_answer(book: &gateway::EndpointBook) -> channels::EndpointsAnswer {
    let endpoints = book
        .endpoints()
        .map(|endpoint| channels::EndpointSummary {
            name: endpoint.name.clone(),
            label: endpoint.label().to_owned(),
            base_url: endpoint.base_url.clone(),
            dialect: endpoint.dialect,
            connection_kind: endpoint.connection_kind.as_str().to_owned(),
            models: endpoint
                .models
                .iter()
                .map(|row| channels::ModelFactsSummary {
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
        .map(|(tag, endpoint, entry)| channels::ChosenSummary {
            tag,
            endpoint: endpoint.to_owned(),
            model: entry.id.clone(),
            max_output_tokens: entry.max_output_tokens,
        })
        .collect();
    channels::EndpointsAnswer { endpoints, chosen }
}

/// The building a `pursuit_changed` record is about.
///
/// # Errors
/// Refuses a record with no address: the step it records belongs to no
/// building, and a fold that skipped it would keep whatever goal the
/// step changed.
pub(crate) fn pursued(record: &EventRecord) -> Result<Address, kernel::AxError> {
    record.addr().cloned().ok_or_else(|| {
        kernel::AxError::failure(
            kernel::AxCode::WireMismatch,
            "read a pursuit_changed line",
            format!("line {} names no building", record.seq().value()),
        )
        .with_recovery("replay with the build that wrote this record")
    })
}

/// Every building the city has, in reading order.
///
/// The plans themselves are `accounting::plan_view`'s: reading them here as
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
pub(crate) fn signal_line(
    record: &EventRecord,
) -> Result<(Address, channels::SignalLine), AxError> {
    let signal = collab::Signal::from_payload(record.data())?;
    Ok((
        signal.room().clone(),
        channels::SignalLine {
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
pub(crate) fn discard_lines(record: &EventRecord) -> Vec<channels::DiscardLine> {
    let Ok(FileDiscarded { paths, restoration }) = record.data().read() else {
        return Vec::new();
    };
    paths
        .into_iter()
        .map(|path| channels::DiscardLine {
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
pub(crate) fn registry_line(record: &EventRecord) -> Option<channels::RegistryLine> {
    let AssetArchived { kind, subject, .. } = record.data().read().ok()?;
    Some(channels::RegistryLine {
        addr: record.addr().cloned()?,
        kind,
        subject,
        at: record.t(),
    })
}

pub(crate) fn summarize(run: RunId, hot: &memory::RunHot) -> channels::RunSummary {
    channels::RunSummary {
        run,
        who: hot.who.clone(),
        frozen: matches!(hot.phase, memory::RunPhase::Frozen),
        last_seq: hot.last_seq,
        last_kind: hot.last_kind,
        addr: hot.addr.clone(),
        started: hot.started,
        completion: hot.completion.clone(),
        pr: hot.pr.clone(),
        ask: hot.ask.clone(),
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
}
