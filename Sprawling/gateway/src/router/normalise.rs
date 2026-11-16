// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use kernel::{AxCode, AxError, DialectKind};

const UNLISTED_HOST_PATH: &str = "/v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DialectHint {

    Unset,

    Chat,

    Responses,

    Messages,
}

impl DialectHint {

    #[must_use]
    pub const fn of(dialect: DialectKind) -> DialectHint {
        match dialect {
            DialectKind::Anthropic => DialectHint::Messages,
            DialectKind::OpenAi => DialectHint::Chat,
            DialectKind::OpenAiResponses => DialectHint::Responses,
        }
    }

    #[must_use]
    pub const fn dialect(self) -> Option<DialectKind> {
        match self {
            DialectHint::Unset => None,
            DialectHint::Chat => Some(DialectKind::OpenAi),
            DialectHint::Responses => Some(DialectKind::OpenAiResponses),
            DialectHint::Messages => Some(DialectKind::Anthropic),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Normalised {

    pub base_url: String,

    pub dialect: DialectHint,
}

impl Normalised {

    #[must_use]
    pub const fn face(&self) -> DialectKind {
        match self.dialect.dialect() {
            Some(face) => face,
            None => DialectKind::OpenAi,
        }
    }
}

pub(crate) trait HostDefaults {

    fn default_path(&self, host: &str, face: DialectHint) -> Option<&str>;

    fn default_dialect(&self, host: &str) -> Option<DialectHint>;
}

pub fn normalise_entered(entered: &str, hint: DialectHint) -> Result<Normalised, AxError> {
    normalise(entered, hint, &crate::provider::preset::Presets)
}

pub fn normalise(
    entered: &str,
    hint: DialectHint,
    presets: &dyn HostDefaults,
) -> Result<Normalised, AxError> {
    let trimmed = entered.trim();
    let (given_scheme, locator) = match trimmed.split_once("://") {
        Some((scheme, rest)) => (Some(scheme.to_ascii_lowercase()), rest),
        None => (None, trimmed),
    };
    refuse_credentials(locator)?;
    if let Some(scheme) = given_scheme.as_deref()
        && scheme != "http"
        && scheme != "https"
    {
        return Err(refusal(
            entered,
            format!("{scheme} is not a scheme the city can call"),
            "enter an http:// or https:// URL, the one the provider's documentation prints",
        ));
    }

    let probe = match given_scheme.as_deref() {
        Some(scheme) => format!("{scheme}://{locator}"),
        None => format!("http://{locator}"),
    };
    let Some((host, _, _)) = crate::reach::split(&probe) else {
        return Err(refusal(
            entered,
            "no host to call",
            "enter the base URL of the provider, for example https://api.openai.com/v1",
        ));
    };
    let host = host.to_ascii_lowercase();
    let scheme = match given_scheme {
        Some(scheme) => scheme,
        None if crate::is_local(&probe) => "http".to_owned(),
        None => "https".to_owned(),
    };
    let (address, suffix) = split_suffix(locator);
    let (authority, entered_path) = match address.split_once('/') {
        Some((authority, path)) => (authority, path),
        None => (address, ""),
    };
    let segments: Vec<&str> = entered_path.split('/').filter(|s| !s.is_empty()).collect();
    let (kept, from_url) = strip_face(&segments);
    let dialect = match resolve_dialect(from_url, hint, &host, presets) {

        DialectHint::Unset if crate::is_local(&probe) => DialectHint::Chat,
        settled @ (DialectHint::Unset
        | DialectHint::Chat
        | DialectHint::Responses
        | DialectHint::Messages) => settled,
    };
    let path = resolve_path(&kept, &suffix, presets.default_path(&host, dialect));
    Ok(Normalised {
        base_url: format!(
            "{scheme}://{}{path}{suffix}",
            authority.to_ascii_lowercase()
        ),
        dialect,
    })
}

fn refuse_credentials(locator: &str) -> Result<(), AxError> {
    let (address, suffix) = split_suffix(locator);
    let authority = address
        .split_once('/')
        .map_or(address, |(authority, _)| authority);
    let found = if authority.contains('@') {
        "a user name or password written before the host"
    } else if query_carries_credential(&suffix) {
        "a credential written into the query"
    } else {
        return Ok(());
    };
    Err(AxError::failure(
        AxCode::ConfigInvalid,
        "read the endpoint URL",
        format!("the URL carries {found}"),
    )
    .with_recovery(
        "remove the key from the URL and enter it as the endpoint's key, which the city keeps in the vault"
            .to_owned(),
    ))
}

fn query_carries_credential(suffix: &str) -> bool {
    suffix
        .trim_start_matches(['?', '#'])
        .split(['&', '#'])
        .any(|pair| {
            let (name, value) = pair.split_once('=').unwrap_or((pair, ""));
            kernel::secret::names_a_credential(name)
                || !kernel::secret::scan(value.as_bytes()).is_empty()
        })
}

fn refusal(entered: &str, subject: impl Into<String>, recovery: &str) -> AxError {
    AxError::failure(
        AxCode::ConfigInvalid,
        format!("read the endpoint URL {entered}"),
        subject,
    )
    .with_recovery(recovery.to_owned())
}

fn split_suffix(locator: &str) -> (&str, String) {
    if let Some((address, query)) = locator.split_once('?') {
        return (address, format!("?{query}"));
    }
    if let Some((address, fragment)) = locator.split_once('#') {
        return (address, format!("#{fragment}"));
    }
    (locator, String::new())
}

fn strip_face<'p>(segments: &[&'p str]) -> (Vec<&'p str>, DialectHint) {
    let mut kept: &[&'p str] = segments;
    let mut face = DialectHint::Unset;
    loop {
        let (rest, found) = strip_one_face(kept);
        match found {
            DialectHint::Unset => return (kept.to_vec(), face),
            DialectHint::Chat | DialectHint::Responses | DialectHint::Messages => {
                if face == DialectHint::Unset {
                    face = found;
                }
                kept = rest;
            }
        }
    }
}

fn strip_one_face<'s, 'p>(segments: &'s [&'p str]) -> (&'s [&'p str], DialectHint) {
    match segments.split_last() {
        Some((&"completions", head)) => match head.split_last() {
            Some((&"chat", rest)) => (rest, DialectHint::Chat),
            _ => (segments, DialectHint::Unset),
        },
        Some((&"responses", head)) => (head, DialectHint::Responses),
        Some((&"messages", head)) => (head, DialectHint::Messages),
        _ => (segments, DialectHint::Unset),
    }
}

fn resolve_path(kept: &[&str], suffix: &str, preset: Option<&str>) -> String {
    let entered = kept.join("/");
    if !entered.is_empty() {
        return format!("/{entered}");
    }
    if !suffix.is_empty() {
        return String::new();
    }

    match preset.map(|preset| preset.trim_matches('/')) {
        Some("") => String::new(),
        Some(preset) => format!("/{preset}"),
        None => UNLISTED_HOST_PATH.to_owned(),
    }
}

fn resolve_dialect(
    from_url: DialectHint,
    hint: DialectHint,
    host: &str,
    presets: &dyn HostDefaults,
) -> DialectHint {
    match (from_url, hint) {
        (DialectHint::Unset, DialectHint::Unset) => match presets.default_dialect(host) {
            Some(preset) => preset,
            None => DialectHint::Unset,
        },
        (DialectHint::Unset, chosen) => chosen,
        (from_url, _) => from_url,
    }
}

#[cfg(test)]
mod tests;
