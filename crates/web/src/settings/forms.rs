// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The two forms and the one statement of what a complete choice is.

use crate::lang::Msg;
use channels::ModelTag;
use channels::{DialectKind, EndpointsAnswer};

/// What the person has filled in. Strings because that is what a form
/// yields; the judgement about whether they are usable is [`ready`].
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AttachForm {
    pub name: String,
    pub base_url: String,
    pub dialect: Option<DialectKind>,
    /// The reference returned by the enrolment route, not the key. A
    /// local server that asks for nothing leaves it empty.
    pub secret: Option<String>,
    /// The models the person ticked after asking what this base URL
    /// serves. Empty admits everything, which is what somebody who
    /// never asked meant.
    pub admit: Vec<String>,
}

/// Why a form cannot be submitted yet, or that it can. Exhaustive, so
/// the page always has a sentence and never a disabled button with no
/// explanation beside it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AttachReadiness {
    Ready,
    NeedsName,
    NeedsUrl,
    /// A URL that is neither https nor loopback http. Refused here for
    /// the same reason the adapter refuses it: a credential must not
    /// cross a plaintext link to somewhere else.
    UrlNotSafe,
    NeedsDialect,
}

impl AttachReadiness {
    /// The sentence shown beside the form.
    #[must_use]
    pub fn sentence(&self) -> Msg {
        match self {
            AttachReadiness::Ready => Msg::SettingsAttachIt,
            AttachReadiness::NeedsName => Msg::SettingsNeedsName,
            AttachReadiness::NeedsUrl => Msg::SettingsNeedsUrl,
            AttachReadiness::UrlNotSafe => Msg::SettingsUrlNotSafe,
            AttachReadiness::NeedsDialect => Msg::SettingsNeedsDialect,
        }
    }
}

/// Whether a URL may carry a credential: https anywhere, or plain http
/// only to this machine.
#[must_use]
pub fn url_is_safe(url: &str) -> bool {
    if let Some(rest) = url.strip_prefix("https://") {
        return !rest.is_empty();
    }
    let Some(rest) = url.strip_prefix("http://") else {
        return false;
    };
    let host = rest
        .split('/')
        .next()
        .unwrap_or_default()
        .rsplit_once(':')
        .map_or(rest.split('/').next().unwrap_or_default(), |(head, _)| head);
    matches!(host, "localhost" | "127.0.0.1" | "[::1]" | "::1")
}

/// Whether the attach form can be submitted.
#[must_use]
pub fn ready(form: &AttachForm) -> AttachReadiness {
    if form.name.trim().is_empty() {
        return AttachReadiness::NeedsName;
    }
    if form.base_url.trim().is_empty() {
        return AttachReadiness::NeedsUrl;
    }
    if !url_is_safe(form.base_url.trim()) {
        return AttachReadiness::UrlNotSafe;
    }
    if form.dialect.is_none() {
        return AttachReadiness::NeedsDialect;
    }
    AttachReadiness::Ready
}

/// What a person picked in the model form.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SelectForm {
    pub endpoint: String,
    pub model: String,
    pub tag: Option<ModelTag>,
}

/// Why a model choice is not yet a command. Same shape as the attach
/// form's readiness, and for the same reason: a disabled button with no
/// sentence beside it is a puzzle.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SelectReadiness {
    Ready,
    NeedsEndpoint,
    NeedsModel,
    NeedsTag,
    ModelNotServed,
}

impl SelectReadiness {
    #[must_use]
    pub fn sentence(&self) -> Msg {
        match self {
            SelectReadiness::Ready => Msg::SettingsUseThisModel,
            SelectReadiness::NeedsEndpoint => Msg::SettingsPickProvider,
            SelectReadiness::NeedsModel => Msg::SettingsPickModel,
            SelectReadiness::NeedsTag => Msg::SettingsPickJob,
            SelectReadiness::ModelNotServed => Msg::SettingsModelNotServed,
        }
    }
}

/// The one statement of what a complete model choice is.
///
/// The last case is the load-bearing one: a model the endpoint never
/// listed cannot be chosen here either. The server refuses it too, and
/// this is not a second authority — it is the same refusal shown before
/// the person presses anything.
#[must_use]
pub fn select_ready(form: &SelectForm, answer: &EndpointsAnswer) -> SelectReadiness {
    if form.endpoint.trim().is_empty() {
        return SelectReadiness::NeedsEndpoint;
    }
    if form.model.trim().is_empty() {
        return SelectReadiness::NeedsModel;
    }
    if form.tag.is_none() {
        return SelectReadiness::NeedsTag;
    }
    let served = answer
        .endpoints
        .iter()
        .any(|endpoint| endpoint.name == form.endpoint && endpoint.models.contains(&form.model));
    if !served {
        return SelectReadiness::ModelNotServed;
    }
    SelectReadiness::Ready
}

/// What one endpoint serves, and nothing another endpoint serves.
///
/// The model list used to be every model of every attached provider,
/// so a person could pick a name their chosen provider had never heard
/// of and read `that endpoint does not list this model` back. This is
/// not a second authority over what is servable — the server refuses
/// the same pair — it is the same refusal moved to before the click.
pub(crate) fn models_of(answer: &EndpointsAnswer, endpoint: &str) -> Vec<String> {
    answer
        .endpoints
        .iter()
        .filter(|attached| attached.name == endpoint)
        .flat_map(|attached| attached.models.clone())
        .collect()
}
