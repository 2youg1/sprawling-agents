// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! What the server answered, read as rows.

use crate::lang::{Msg, fill, say};
use channels::{ChosenSummary, DialectKind, EndpointSummary, EndpointsAnswer, ModelTag};

use crate::socket::Enrolment;

pub struct EndpointRow {
    pub name: String,
    pub base_url: String,
    pub dialect: DialectKind,
    /// What this endpoint serves, and what a tag may be pointed at.
    pub models: Vec<String>,
    /// Where this endpoint is, and whether a credential is enrolled for
    /// it. Two messages rather than one sentence, because the words are
    /// joined in the reader's own language at the point of drawing.
    pub reach: Msg,
    pub credential: Msg,
}

/// The rows for what is attached.
#[must_use]
pub fn endpoint_rows(answer: &EndpointsAnswer) -> Vec<EndpointRow> {
    answer.endpoints.iter().map(row_of).collect()
}

fn row_of(endpoint: &EndpointSummary) -> EndpointRow {
    let reach = if endpoint.local {
        Msg::SettingsOnThisMachine
    } else {
        Msg::SettingsOffThisMachine
    };
    let credential = if endpoint.has_credential {
        Msg::SettingsWithCredential
    } else {
        Msg::SettingsNoCredential
    };
    EndpointRow {
        name: endpoint.name.clone(),
        base_url: endpoint.base_url.clone(),
        dialect: endpoint.dialect,
        models: endpoint.models.clone(),
        reach,
        credential,
    }
}

/// One tag and what answers for it, including the tags nothing answers
/// for yet.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TagRow {
    pub tag: ModelTag,
    /// `None` means no model is chosen; the page shows what that costs
    /// rather than leaving the row blank.
    pub chosen: Option<ChosenSummary>,
    pub consequence: Msg,
}

/// Every tag this build knows, in the order the page offers them.
///
/// Tags with nothing chosen are listed too: a settings page that only
/// showed what is configured would hide the thing the person came to
/// fix.
#[must_use]
pub fn tag_rows(answer: &EndpointsAnswer) -> Vec<TagRow> {
    ModelTag::ALL
        .into_iter()
        .map(|tag| TagRow {
            tag,
            chosen: answer
                .chosen
                .iter()
                .find(|choice| choice.tag == tag)
                .cloned(),
            consequence: consequence_of(tag),
        })
        .collect()
}

/// What not choosing a model for this tag means. Stated as the effect on
/// the person's work, because "main is unset" tells them nothing.
fn consequence_of(tag: ModelTag) -> Msg {
    match tag {
        ModelTag::Main => Msg::SettingsMainConsequence,
        ModelTag::Digest => Msg::SettingsDigestConsequence,
        // A tag added upstream without a sentence here still appears,
        // saying only that its effect is unrecorded.
        _ => Msg::SettingsUnknownConsequence,
    }
}

/// What an enrolment answer means to the person watching, and what it
/// leaves behind. Pure, so the sentence and the reference are decided in
/// one place rather than inside a browser callback.
#[must_use]
pub fn enrolment_note(lang: crate::lang::Lang, answer: &Enrolment) -> (Option<String>, String) {
    match answer {
        Enrolment::Stored { reference } => (
            Some(reference.clone()),
            fill(
                say(lang, Msg::SettingsStoredAs),
                &[("reference", reference)],
            ),
        ),
        Enrolment::Refused { reason } => (None, reason.clone()),
    }
}

/// Whether the city can be dispatched to at all: the one question this
/// page exists to answer, and the one a person checks before closing it.
#[must_use]
pub fn can_dispatch(answer: &EndpointsAnswer) -> bool {
    answer
        .chosen
        .iter()
        .any(|choice| choice.tag == ModelTag::Main)
}
