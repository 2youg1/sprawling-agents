// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The one mint for a compressed summary's producer fingerprint.
//!
//! A summary outlives the model that wrote it: it is carried into later
//! prefixes and read long after the room has moved to another model. The
//! fingerprint answers "who wrote this, and how many summaries deep is
//! it" from the record alone, and this module is the only place that
//! stamps one.
//!
//! **Missing provenance is recorded as unknown, never guessed.** The two
//! guesses this refuses are the ones that look like answers: naming the
//! model a replay happens to run on as the producer of an old summary,
//! and giving a chain nobody counted a generation anyway. A generation
//! with no anchor is a claim, and the record does not make it.

use kernel::event::record::SummaryProducer;
use std::num::NonZeroU32;

/// What a summary produced now, under `model`, is stamped with, given
/// what the summaries before it recorded.
///
/// The chain starts at generation 1; a recorded generation before it is
/// one higher. A chain whose previous stamp is [`SummaryProducer::Unknown`],
/// or a model name that is empty, answers [`SummaryProducer::Unknown`]:
/// half a fingerprint is the other half's guess, and an empty name is the
/// absence of a name rather than one.
///
/// Generation arithmetic is checked. A chain deeper than the count can
/// hold is recorded as unknown rather than wrapped to zero, because zero
/// and unknown are two different answers.
#[must_use]
pub fn mint(model: &str, previous: Option<&SummaryProducer>) -> SummaryProducer {
    if model.trim().is_empty() {
        return SummaryProducer::Unknown;
    }
    let generation = match previous {
        None => NonZeroU32::MIN,
        Some(SummaryProducer::Unknown) => return SummaryProducer::Unknown,
        Some(SummaryProducer::Written { generation, .. }) => {
            let Some(next) = generation.get().checked_add(1).and_then(NonZeroU32::new) else {
                return SummaryProducer::Unknown;
            };
            next
        }
    };
    SummaryProducer::Written {
        model: model.to_owned(),
        generation,
    }
}
