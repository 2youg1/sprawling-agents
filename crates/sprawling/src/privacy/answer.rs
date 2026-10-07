// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The privacy page's answer, put together from the controls table, the
//! original items, what the host read and what the history released
//! (`crates/sprawling/spec/Privacy/Service.lean`). No IO: the reads and the
//! identity check happen in [`super::service`] before anything here runs.

use wire::{
    PrivacyAnswer, PrivacyControl, PrivacyControlEntry, PrivacyCurrent, PrivacyEditions,
    PrivacyHistory, PrivacyHost, PrivacyIntent, PrivacyNotWrittenEntry, PrivacyOriginal,
    PrivacyOriginalLine, PrivacyOutcome, PrivacyTarget, PrivacyValue,
};

use super::controls::definition;
use super::fault::ReadFault;
use super::originals::{Disposition, original};
use super::state::{Holdings, Intent};
use super::target::Snapshot;

/// The whole answer: every control in page order with what `read` gave
/// for it, which is nothing when the host is not Windows and nothing was
/// read, and every original item that is not written.
pub(super) fn answer(
    host: PrivacyHost,
    mut read: impl FnMut(PrivacyControl) -> Option<Result<Snapshot, ReadFault>>,
    history: PrivacyHistory,
    outcomes: Vec<PrivacyOutcome>,
) -> PrivacyAnswer {
    let edition = match &host {
        PrivacyHost::Windows(windows) => windows.edition,
        PrivacyHost::Unreadable { .. } | PrivacyHost::NotWindows => None,
    };
    let controls = PrivacyControl::ALL
        .into_iter()
        .map(|control| {
            let row = definition(control);
            let current = read(control);
            PrivacyControlEntry {
                control,
                category: row.category,
                originals: PrivacyOriginal::ALL
                    .into_iter()
                    .filter(|&item| {
                        matches!(original(item).disposition, Disposition::Writes(writes) if writes == control)
                    })
                    .map(line)
                    .collect(),
                target: PrivacyTarget::from(&row.target),
                scope: row.target.kind().scope(),
                editions: PrivacyEditions {
                    honoured: row.editions.honoured.to_vec(),
                    ignored: row.editions.ignored.to_vec(),
                },
                host_fit: row.editions.fit(edition),
                build_effect: row.build_effect,
                written: match &current {
                    Some(Ok(snapshot)) => row.written.target(snapshot).as_ref().map(PrivacyValue::from),
                    Some(Err(_)) | None => None,
                },
                current: match current {
                    None => PrivacyCurrent::NotRead,
                    Some(Ok(snapshot)) => PrivacyCurrent::Read {
                        value: PrivacyValue::from(&snapshot),
                    },
                    Some(Err(ReadFault::AccessDenied)) => PrivacyCurrent::AccessDenied,
                    Some(Err(ReadFault::Failed(error))) => PrivacyCurrent::Failed { error },
                },
            }
        })
        .collect();
    let not_written = PrivacyOriginal::ALL
        .into_iter()
        .filter_map(|item| match original(item).disposition {
            Disposition::Writes(_) => None,
            Disposition::NotWritten {
                reason,
                alternatives,
            } => Some(PrivacyNotWrittenEntry {
                line: line(item),
                reason,
                alternatives: alternatives.to_vec(),
            }),
        })
        .collect();
    PrivacyAnswer {
        host,
        controls,
        not_written,
        history,
        outcomes,
    }
}

/// What a history released to the account asking: each control's latest
/// owned change and the operation that has no conclusion.
pub(super) fn disclosed(holdings: &Holdings<'_, ()>) -> PrivacyHistory {
    PrivacyHistory::Disclosed {
        owned: holdings.owned().map(intent).collect(),
        unresolved: holdings.unresolved().map(intent),
    }
}

fn intent(intent: &Intent) -> PrivacyIntent {
    PrivacyIntent {
        operation: intent.operation.get(),
        control: intent.control,
        original: PrivacyValue::from(&intent.original),
        modified: PrivacyValue::from(&intent.modified),
        restore_of: intent.restore_of.map(std::num::NonZeroU64::get),
    }
}

fn line(item: PrivacyOriginal) -> PrivacyOriginalLine {
    PrivacyOriginalLine {
        item,
        text: original(item).text.to_owned(),
    }
}
