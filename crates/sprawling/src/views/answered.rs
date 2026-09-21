// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! What is waiting for a person, and what they have already answered.
//!
//! Two arms of one fold, in one file because they are two halves of one
//! subject: an item arrives, and later a ruling closes it. Reading the
//! two side by side is what shows that the question is stored whole and
//! the answer is stored as the ruling plus the cluster it covered.

use kernel::{AxCode, AxError, EventRecord};

use super::holding::Views;

impl Views {
    /// Files one question that is waiting for a person.
    ///
    /// The payload *is* the item: it was written by serialising one, so
    /// it reads back as one. Rebuilding a lesser shape out of
    /// hand-picked fields is how this view came to show every waiting
    /// item as "(no summary recorded)" — the field it read had never
    /// been written by anybody.
    ///
    /// # Errors
    /// Refuses a payload that is not an approval item. The record
    /// stands; this view skips it and the observer reports it.
    pub(super) fn fold_question(&mut self, record: &EventRecord) -> Result<(), AxError> {
        let value = serde_json::Value::Object(record.data().as_map().clone());
        let item: kernel::ApprovalItem = serde_json::from_value(value).map_err(|err| {
            AxError::failure(
                AxCode::WireMismatch,
                "fold an approval into the queue",
                format!("seq {}: {err}", record.seq().value()),
            )
            .with_recovery("the record stands; this view skips it and the observer reports it")
        })?;
        self.approvals.insert(item.id.as_str().to_owned(), item);
        Ok(())
    }

    /// Closes one question with the ruling a person gave it.
    ///
    /// The cluster travels with the answer because the person answered
    /// the group they were shown; a row whose cluster will not read
    /// back is still an answer that happened, so it lands with an empty
    /// detail rather than being dropped.
    ///
    /// # Errors
    /// Refuses a ruling this build cannot read. It used to read as an
    /// allowance, which showed the person a permission they had never
    /// given.
    pub(super) fn fold_ruling(&mut self, record: &EventRecord) -> Result<(), AxError> {
        let data = record.data().as_map();
        let Some(id) = data.get("id").and_then(serde_json::Value::as_str) else {
            return Ok(());
        };
        self.approvals.remove(id);
        let cluster = data
            .get("cluster")
            .cloned()
            .and_then(|value| serde_json::from_value(value).ok())
            .unwrap_or(kernel::ClusterKey {
                class: kernel::ApprovalClass::Question,
                detail: String::new(),
            });
        let spelled = data.get("verdict").and_then(serde_json::Value::as_str);
        let verdict = match spelled {
            Some("allow") => kernel::Ruling::Allow,
            Some("deny") => kernel::Ruling::Deny,
            Some(_) | None => {
                return Err(AxError::failure(
                    AxCode::WireMismatch,
                    "fold an answer into what was decided",
                    format!(
                        "seq {}: {} is not a ruling",
                        record.seq().value(),
                        spelled.unwrap_or("no verdict at all")
                    ),
                )
                .with_recovery(
                    "the record stands; this view skips it and the observer reports it",
                ));
            }
        };
        self.decided.push(channels::Decision {
            item: id.to_owned(),
            verdict,
            cluster,
            at: record.t(),
        });
        Ok(())
    }
}
