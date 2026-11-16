// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use kernel::{AccountDisposition, AxError, ProviderFailureKind, Retry};

fn transport_detail(err: &reqwest::Error) -> String {
    let mut out = err.to_string();
    let mut cause = std::error::Error::source(err);
    while let Some(link) = cause {
        out.push_str(": ");
        out.push_str(&link.to_string());
        cause = link.source();
    }
    out
}

#[derive(Debug)]
pub(crate) enum ProviderFailure<'e> {
    Exchange(&'e reqwest::Error),

    Cut(&'e std::io::Error),

    Silence {
        quiet_ms: u64,
    },

    Refused {
        url: &'e str,
        status: reqwest::StatusCode,
        headers: &'e reqwest::header::HeaderMap,
    },

    AccountUnavailable {
        url: &'e str,
        status: reqwest::StatusCode,
    },

    Overflow {
        url: &'e str,
        status: reqwest::StatusCode,
    },

    Unreadable(String),

    Reported {
        kind: &'e str,
    },

    Unbuilt(&'e reqwest::Error),
}

const WINDOW_MARKERS: [&str; 4] = [
    "context_length_exceeded",
    "prompt is too long",
    "maximum context length",
    "context window",
];

const QUOTA_EXHAUSTED: &str = "insufficient_quota";

fn names_an_exhausted_quota(said: &str) -> bool {
    let Ok(body) = serde_json::from_str::<serde_json::Value>(said) else {
        return false;
    };
    let error = body.get("error");
    let field = |name: &str| {
        error
            .and_then(|error| error.get(name))
            .filter(|value| !value.is_null())
    };
    field("code")
        .or_else(|| field("type"))
        .and_then(serde_json::Value::as_str)
        == Some(QUOTA_EXHAUSTED)
}

impl<'e> ProviderFailure<'e> {
    pub(crate) fn refusal(
        url: &'e str,
        status: reqwest::StatusCode,
        headers: &'e reqwest::header::HeaderMap,
        body: &reqwest::Result<String>,
    ) -> ProviderFailure<'e> {
        let outgrew = |said: &String| {
            let lowered = said.to_ascii_lowercase();
            WINDOW_MARKERS.iter().any(|marker| lowered.contains(marker))
        };
        match (status.as_u16(), body) {
            (400 | 413, Ok(said)) if outgrew(said) => ProviderFailure::Overflow { url, status },
            (429, Ok(said)) if names_an_exhausted_quota(said) => {
                ProviderFailure::AccountUnavailable { url, status }
            }
            _ => ProviderFailure::Refused {
                url,
                status,
                headers,
            },
        }
    }
}

impl ProviderFailure<'_> {
    fn subject(&self) -> String {
        match self {
            ProviderFailure::Exchange(err) | ProviderFailure::Unbuilt(err) => transport_detail(err),
            ProviderFailure::Cut(err) => err.to_string(),
            ProviderFailure::Silence { quiet_ms } => {
                format!("no byte arrived for {quiet_ms} ms")
            }
            ProviderFailure::Refused { url, status, .. }
            | ProviderFailure::AccountUnavailable { url, status }
            | ProviderFailure::Overflow { url, status } => {
                format!("{url} answered {}", status.as_u16())
            }
            ProviderFailure::Unreadable(detail) => detail.clone(),
            ProviderFailure::Reported { kind } => format!("the stream reported {kind}"),
        }
    }

    fn kind(&self) -> ProviderFailureKind {
        let status = |code: &reqwest::StatusCode| u32::from(code.as_u16());
        match self {
            ProviderFailure::Exchange(_) => ProviderFailureKind::Exchange,
            ProviderFailure::Cut(_) => ProviderFailureKind::Cut,
            ProviderFailure::Silence { .. } => ProviderFailureKind::Silence,
            ProviderFailure::Refused { status: code, .. } => ProviderFailureKind::Refused {
                status: status(code),
            },
            ProviderFailure::AccountUnavailable { status: code, .. } => {
                ProviderFailureKind::Quota {
                    status: status(code),
                }
            }
            ProviderFailure::Overflow { status: code, .. } => ProviderFailureKind::Overflow {
                status: status(code),
            },
            ProviderFailure::Unreadable(_) => ProviderFailureKind::Unreadable,
            ProviderFailure::Reported { .. } => ProviderFailureKind::Reported,
            ProviderFailure::Unbuilt(_) => ProviderFailureKind::Unbuilt,
        }
    }

    fn retry(&self) -> Retry {
        match self {
            ProviderFailure::Exchange(err) if err.is_connect() => Retry::Yes,
            ProviderFailure::Exchange(_)
            | ProviderFailure::Cut(_)
            | ProviderFailure::Silence { .. } => Retry::Unknown,
            ProviderFailure::Refused { status, .. }
                if *status == reqwest::StatusCode::REQUEST_TIMEOUT
                    || *status == reqwest::StatusCode::TOO_MANY_REQUESTS
                    || status.is_server_error() =>
            {
                Retry::Yes
            }
            ProviderFailure::Reported { kind }
                if matches!(
                    *kind,
                    "server_error"
                        | "rate_limit_exceeded"
                        | "overloaded_error"
                        | "api_error"
                        | "rate_limit_error"
                        | "timeout_error"
                ) =>
            {
                Retry::Yes
            }
            ProviderFailure::Refused { .. }
            | ProviderFailure::AccountUnavailable { .. }
            | ProviderFailure::Overflow { .. }
            | ProviderFailure::Reported { .. }
            | ProviderFailure::Unreadable(_)
            | ProviderFailure::Unbuilt(_) => Retry::No,
        }
    }

    fn account_disposition(&self) -> AccountDisposition {
        match self {
            ProviderFailure::Refused { status, .. }
                if *status == reqwest::StatusCode::UNAUTHORIZED =>
            {
                AccountDisposition::Advance
            }
            ProviderFailure::AccountUnavailable { .. }
            | ProviderFailure::Reported {
                kind: QUOTA_EXHAUSTED,
            } => AccountDisposition::Advance,
            ProviderFailure::Exchange(_)
            | ProviderFailure::Cut(_)
            | ProviderFailure::Silence { .. }
            | ProviderFailure::Refused { .. }
            | ProviderFailure::Overflow { .. }
            | ProviderFailure::Unreadable(_)
            | ProviderFailure::Reported { .. }
            | ProviderFailure::Unbuilt(_) => AccountDisposition::Keep,
        }
    }

    fn retry_after_ms(&self) -> Option<u64> {
        let ProviderFailure::Refused { headers, .. } = self else {
            return None;
        };
        let number = |name: &str| headers.get(name)?.to_str().ok()?.trim().parse::<u64>().ok();
        number("retry-after-ms")
            .or_else(|| number("retry-after").and_then(|secs| secs.checked_mul(1_000)))
    }

    fn recovery(&self) -> &'static str {
        match (self, self.retry()) {
            (ProviderFailure::Unbuilt(_), _) => {
                "check this endpoint's `base_url` and its extra headers: the request was \
                 refused by this side before it was sent"
            }
            (
                ProviderFailure::AccountUnavailable { .. }
                | ProviderFailure::Reported {
                    kind: QUOTA_EXHAUSTED,
                },
                _,
            ) => {
                "this account's quota is used up, and asking again will not restore it: add \
                 credit to the account, or add another account to this provider on the \
                 Provider page, then dispatch again"
            }
            (ProviderFailure::Overflow { .. }, _) => {
                "the conversation no longer fits this model's context window, and it would not \
                 fit on a second try: start a new session that keeps the summary (`/new --carry`), \
                 or give this duty a model with a larger window"
            }
            (_, Retry::Yes) => {
                "the watchdog backs off and sends the same request again, until the run's \
                 retry limit or a Halt"
            }
            (_, Retry::Unknown) => {
                "the request went out and its answer was lost, so the provider may have \
                 run and billed it; the watchdog backs off and sends it again, until the \
                 run's retry limit or a Halt"
            }
            (_, Retry::No) => {
                "the provider answered, and it would answer the same way again: check this \
                 endpoint's model name, credential and dialect, then dispatch again"
            }
        }
    }
}

pub(crate) fn provider_err(action: &str, failure: &ProviderFailure<'_>) -> AxError {
    let draft = AxError::provider(failure.kind(), action, failure.subject());
    let draft = match (
        failure.account_disposition(),
        failure.retry(),
        failure.retry_after_ms(),
    ) {
        (AccountDisposition::Advance, _, _) => draft.account_unusable(),
        (AccountDisposition::Keep, Retry::Yes, Some(wait_ms)) => draft.retriable_after(wait_ms),
        (AccountDisposition::Keep, Retry::Yes, None) => draft.retriable(),
        (AccountDisposition::Keep, Retry::Unknown, _) => draft.effect_unknown(),
        (AccountDisposition::Keep, Retry::No, _) => draft,
    };
    draft.with_recovery(failure.recovery())
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "test code"
)]
mod tests;
