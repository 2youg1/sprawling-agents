// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! What failed about one provider call, and the one place that decides
//! whether it may be asked again and what the person does next.

use kernel::{AxCode, AxError};

/// A transport failure as its whole chain states it.
///
/// `reqwest`'s own `Display` says only that sending failed; whether it
/// was a refused connection, an unresolvable name or a passed deadline
/// lives one link further down - and that link is the entire difference
/// between "the provider is down" and "the provider is slow" for the
/// person reading the refusal.
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

/// What failed about one provider call, said by the call site that saw
/// it rather than guessed later from the text of a message.
///
/// **This enum is the one home of "may this be tried again".** An
/// exchange that never completed carries no answer, so the same request
/// may go out again; an answer the city refuses would be refused
/// identically next time. Before this type every call site built its
/// own error and none opted in, which made the default
/// `Retries::UntilHalted` worth zero retries in practice — one
/// transient disconnect ended a sub-run for good.
#[derive(Debug)]
pub(crate) enum ProviderFailure<'e> {
    /// The exchange never completed: the request did not reach the
    /// provider, or the answer's body stopped before its end, or the
    /// transport's deadline passed.
    Exchange(&'e reqwest::Error),
    /// A streamed body could not be read further off the socket.
    Cut(&'e std::io::Error),
    /// A stream stayed open and silent past the bound set for silence.
    Silence { quiet_ms: u64 },
    /// The provider answered and the status refuses the request. The
    /// provider's own body is never quoted back.
    Refused {
        url: &'e str,
        status: reqwest::StatusCode,
    },
    /// The provider answered 2xx and the body is not a shape this city
    /// can read.
    Unreadable(String),
    /// The request could not even be built — a URL, a header or a body
    /// this side wrote wrong. Deterministic: the identical build fails
    /// identically, which is why the one failure here that never
    /// reached a provider is classed with the refusals.
    Unbuilt(&'e reqwest::Error),
}

impl ProviderFailure<'_> {
    fn subject(&self) -> String {
        match self {
            ProviderFailure::Exchange(err) | ProviderFailure::Unbuilt(err) => transport_detail(err),
            ProviderFailure::Cut(err) => err.to_string(),
            ProviderFailure::Silence { quiet_ms } => {
                format!("no byte arrived for {quiet_ms} ms")
            }
            ProviderFailure::Refused { url, status } => {
                format!("{url} answered {}", status.as_u16())
            }
            ProviderFailure::Unreadable(detail) => detail.clone(),
        }
    }

    fn retriable(&self) -> bool {
        match self {
            ProviderFailure::Exchange(_)
            | ProviderFailure::Cut(_)
            | ProviderFailure::Silence { .. } => true,
            ProviderFailure::Refused { .. }
            | ProviderFailure::Unreadable(_)
            | ProviderFailure::Unbuilt(_) => false,
        }
    }

    fn recovery(&self) -> &'static str {
        match self {
            ProviderFailure::Exchange(_)
            | ProviderFailure::Cut(_)
            | ProviderFailure::Silence { .. } => {
                "the watchdog decides retry or failover; admission widens the interval"
            }
            ProviderFailure::Refused { .. } | ProviderFailure::Unreadable(_) => {
                "the provider answered, and it would answer the same way again: check this \
                 endpoint's model name, credential and dialect, then dispatch again"
            }
            ProviderFailure::Unbuilt(_) => {
                "check this endpoint's `base_url` and its extra headers: the request was \
                 refused by this side before it was sent"
            }
        }
    }
}

/// One provider failure as an `E_PROVIDER` error, its retriability and
/// its way out taken from [`ProviderFailure`], never decided here.
pub(crate) fn provider_err(action: &str, failure: &ProviderFailure<'_>) -> AxError {
    let draft = AxError::failure(AxCode::Provider, action, failure.subject());
    let draft = if failure.retriable() {
        draft.retriable()
    } else {
        draft
    };
    draft.with_recovery(failure.recovery())
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    reason = "test code"
)]
mod tests {
    use super::*;

    #[test]
    fn what_never_completed_is_asked_again_and_what_was_refused_is_not() {
        let cut = std::io::Error::new(std::io::ErrorKind::UnexpectedEof, "the body stopped");
        for exchange in [
            ProviderFailure::Cut(&cut),
            ProviderFailure::Silence { quiet_ms: 5 },
        ] {
            assert!(provider_err("read provider response", &exchange).is_retriable());
        }
        let refused = provider_err(
            "call provider",
            &ProviderFailure::Refused {
                url: "http://house/v1",
                status: reqwest::StatusCode::INTERNAL_SERVER_ERROR,
            },
        );
        assert!(!refused.is_retriable(), "it would answer the same way");
        assert!(refused.subject().contains("answered 500"));
        let shape = ProviderFailure::Unreadable("no data array".to_owned());
        assert!(!provider_err("read the model list", &shape).is_retriable());
    }

}
