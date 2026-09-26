// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! What failed about one provider call, and the one place that says
//! whether the same request may go out again and what the caller does
//! next.

use kernel::{AxCode, AxError, Retry};

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
/// may go out again, and so may one the provider answered with "busy"
/// or "broken"; an answer that says the request itself is wrong would
/// be given identically next time. Before this type every call site built its
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
    /// The provider answered with a status that is not 2xx. The
    /// provider's own body is never quoted back; its headers are read
    /// for the one hint they may carry, how long to wait.
    Refused {
        url: &'e str,
        status: reqwest::StatusCode,
        headers: &'e reqwest::header::HeaderMap,
    },
    /// The provider answered 2xx and the body is not a shape this city
    /// can read.
    Unreadable(String),
    /// A stream that already answered 2xx carried an error frame of
    /// this type or code. Its message is not quoted, like a refusal's body.
    Reported { kind: &'e str },
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
            ProviderFailure::Refused { url, status, .. } => {
                format!("{url} answered {}", status.as_u16())
            }
            ProviderFailure::Unreadable(detail) => detail.clone(),
            ProviderFailure::Reported { kind } => format!("the stream reported {kind}"),
        }
    }

    /// Whether the identical request may succeed if sent again later,
    /// and whether its effect landed. A connection that never opened
    /// carried no request, and a provider answering 408, 429 or 5xx (529
    /// is Anthropic's overload) says it is busy or broken now, not that
    /// the request is wrong. Any other exchange that stopped did so after
    /// the request left, so the provider may have run it and billed it.
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
            | ProviderFailure::Reported { .. }
            | ProviderFailure::Unreadable(_)
            | ProviderFailure::Unbuilt(_) => Retry::No,
        }
    }

    /// The provider's own word on how long to wait: `retry-after-ms`
    /// first, then `retry-after` in whole seconds. The HTTP-date form is
    /// not read, because turning a date into a wait needs a clock this
    /// crate does not sample; the watchdog's schedule covers it.
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

/// One provider failure as an `E_PROVIDER` error, its retriability and
/// its way out taken from [`ProviderFailure`], never decided here.
pub(crate) fn provider_err(action: &str, failure: &ProviderFailure<'_>) -> AxError {
    let draft = AxError::failure(AxCode::Provider, action, failure.subject());
    let draft = match (failure.retry(), failure.retry_after_ms()) {
        (Retry::Yes, Some(wait_ms)) => draft.retriable_after(wait_ms),
        (Retry::Yes, None) => draft.retriable(),
        (Retry::Unknown, _) => draft.effect_unknown(),
        (Retry::No, _) => draft,
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
            assert_eq!(
                provider_err("read provider response", &exchange).retry(),
                Retry::Unknown
            );
        }
        let refused = provider_err(
            "call provider",
            &ProviderFailure::Refused {
                url: "http://house/v1",
                status: reqwest::StatusCode::BAD_REQUEST,
                headers: &reqwest::header::HeaderMap::new(),
            },
        );
        assert_eq!(refused.retry(), Retry::No, "it would answer the same way");
        assert!(refused.subject().contains("answered 400"));
        let shape = ProviderFailure::Unreadable("no data array".to_owned());
        assert_eq!(
            provider_err("read the model list", &shape).retry(),
            Retry::No
        );
    }
    #[test]
    fn a_provider_that_says_busy_or_broken_is_asked_again_and_one_that_refuses_is_not() {
        let asked_again = |code: u16| {
            let status = reqwest::StatusCode::from_u16(code).unwrap();
            provider_err(
                "call provider",
                &ProviderFailure::Refused {
                    url: "http://house/v1",
                    status,
                    headers: &reqwest::header::HeaderMap::new(),
                },
            )
            .retry()
                == Retry::Yes
        };
        let statuses = [400, 401, 403, 404, 408, 422, 429, 500, 502, 503, 504, 529];
        let retried: Vec<u16> = statuses.into_iter().filter(|&c| asked_again(c)).collect();
        assert_eq!(retried, [408, 429, 500, 502, 503, 504, 529]);
    }
    #[test]
    fn every_way_out_reads_as_one_sentence() {
        let cut = std::io::Error::new(std::io::ErrorKind::UnexpectedEof, "the body stopped");
        let failures = [
            ProviderFailure::Cut(&cut),
            ProviderFailure::Refused {
                url: "http://house/v1",
                status: reqwest::StatusCode::BAD_REQUEST,
                headers: &reqwest::header::HeaderMap::new(),
            },
        ];
        for failure in failures {
            let recovery = failure.recovery();
            assert!(!recovery.contains("  "), "{recovery:?}");
        }
        assert_eq!(
            ProviderFailure::Cut(&cut).recovery(),
            "the request went out and its answer was lost, so the provider may have run and billed it; the watchdog backs off and sends it again, until the run's retry limit or a Halt"
        );
    }

    #[test]
    fn a_request_that_went_out_and_lost_its_answer_says_its_effect_is_unknown() {
        let cut = std::io::Error::new(std::io::ErrorKind::UnexpectedEof, "the body stopped");
        let headers = reqwest::header::HeaderMap::new();
        let refused = |code: u16| ProviderFailure::Refused {
            url: "http://house/v1",
            status: reqwest::StatusCode::from_u16(code).unwrap(),
            headers: &headers,
        };
        let retry = |failure: &ProviderFailure<'_>| {
            serde_json::to_value(provider_err("call provider", failure)).unwrap()["retry"].clone()
        };
        assert_eq!(
            [
                retry(&ProviderFailure::Cut(&cut)),
                retry(&ProviderFailure::Silence { quiet_ms: 5 }),
                retry(&refused(503)),
                retry(&refused(400)),
            ],
            ["unknown", "unknown", "yes", "no"]
        );
    }

    #[test]
    fn a_wait_the_provider_names_travels_with_the_failure() {
        let told = |code: u16, header: [&'static str; 2]| {
            let mut headers = reqwest::header::HeaderMap::new();
            headers.insert(header[0], header[1].parse().unwrap());
            provider_err(
                "call provider",
                &ProviderFailure::Refused {
                    url: "http://house/v1",
                    status: reqwest::StatusCode::from_u16(code).unwrap(),
                    headers: &headers,
                },
            )
            .retry_after_ms()
        };
        assert_eq!(
            [
                told(429, ["retry-after", "7"]),
                told(503, ["retry-after-ms", "250"]),
                told(429, ["retry-after", "Wed, 21 Oct 2015 07:28:00 GMT"]),
                told(400, ["retry-after", "7"]),
            ],
            [Some(7_000), Some(250), None, None]
        );
    }
}
