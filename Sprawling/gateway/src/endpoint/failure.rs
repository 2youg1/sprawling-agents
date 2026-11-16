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

    Silence { quiet_ms: u64 },

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

    Reported { kind: &'e str },

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
        assert_eq!(
            ProviderFailure::Overflow {
                url: "http://house/v1",
                status: reqwest::StatusCode::BAD_REQUEST,
            }
            .recovery(),
            "the conversation no longer fits this model's context window, and it would not fit on a second try: start a new session that keeps the summary (`/new --carry`), or give this duty a model with a larger window"
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

    #[test]
    fn a_refusal_advances_exactly_for_a_rejected_key_or_an_exhausted_quota() {
        let headers = reqwest::header::HeaderMap::new();
        let statuses = [400, 401, 402, 403, 404, 408, 413, 422, 429, 500, 503, 529];
        for status in statuses {
            for outgrew in [false, true] {
                for quota in [false, true] {
                    let body = serde_json::json!({ "error": {
                        "message": if outgrew { "maximum context length is 8192 tokens" } else { "no" },
                        "type": if quota { "insufficient_quota" } else { "invalid_request_error" },
                        "code": serde_json::Value::Null,
                    }})
                    .to_string();
                    let failure = ProviderFailure::refusal(
                        "http://house/v1",
                        reqwest::StatusCode::from_u16(status).unwrap(),
                        &headers,
                        &Ok(body),
                    );
                    let json =
                        serde_json::to_value(provider_err("call provider", &failure)).unwrap();
                    let overflow = matches!(status, 400 | 413) && outgrew;
                    let exhausted = status == 429 && quota;
                    let busy = matches!(status, 408 | 429) || status >= 500;
                    let kind = if overflow {
                        "overflow"
                    } else if exhausted {
                        "quota"
                    } else {
                        "refused"
                    };
                    let retry = if busy && !overflow && !exhausted {
                        "yes"
                    } else {
                        "no"
                    };
                    let account =
                        (status == 401 || exhausted).then(|| serde_json::json!("advance"));
                    assert_eq!(
                        (&json["provider"], &json["retry"], json.get("account")),
                        (
                            &serde_json::json!({ "kind": kind, "status": status }),
                            &serde_json::json!(retry),
                            account.as_ref()
                        ),
                        "status {status}, outgrew {outgrew}, quota {quota}"
                    );
                }
            }
        }
    }

    #[test]
    fn only_the_structured_code_or_type_says_the_quota_is_used_up() {
        let headers = reqwest::header::HeaderMap::new();
        let kind_of = |body: reqwest::Result<String>| {
            let failure = ProviderFailure::refusal(
                "http://house/v1",
                reqwest::StatusCode::TOO_MANY_REQUESTS,
                &headers,
                &body,
            );
            serde_json::to_value(provider_err("call provider", &failure)).unwrap()["provider"]["kind"]
                .clone()
        };
        let said = |error: serde_json::Value| Ok(serde_json::json!({ "error": error }).to_string());
        assert_eq!(
            [
                kind_of(said(
                    serde_json::json!({ "type": "insufficient_quota", "code": "insufficient_quota" })
                )),
                kind_of(said(
                    serde_json::json!({ "type": "requests", "code": "insufficient_quota" })
                )),
                kind_of(said(serde_json::json!({ "type": "insufficient_quota" }))),
                kind_of(said(
                    serde_json::json!({ "type": "insufficient_quota", "code": "rate_limit_exceeded" })
                )),
                kind_of(said(
                    serde_json::json!({ "type": "requests", "message": "insufficient_quota" })
                )),
                kind_of(Ok("insufficient_quota".to_owned())),
            ],
            ["quota", "quota", "quota", "refused", "refused", "refused"]
        );
    }

    #[test]
    fn a_reported_error_advances_exactly_for_an_exhausted_quota() {
        let said = |kind: &str| {
            let json = serde_json::to_value(provider_err(
                "read provider stream",
                &ProviderFailure::Reported { kind },
            ))
            .unwrap();
            (json["retry"].clone(), json.get("account").cloned())
        };
        assert_eq!(
            [
                said("insufficient_quota"),
                said("rate_limit_exceeded"),
                said("invalid_prompt")
            ],
            [
                (serde_json::json!("no"), Some(serde_json::json!("advance"))),
                (serde_json::json!("yes"), None),
                (serde_json::json!("no"), None),
            ]
        );
    }

    #[test]
    fn an_unknown_effect_keeps_the_account() {
        let cut = std::io::Error::new(std::io::ErrorKind::UnexpectedEof, "the body stopped");
        for failure in [
            ProviderFailure::Cut(&cut),
            ProviderFailure::Silence { quiet_ms: 5 },
        ] {
            let json =
                serde_json::to_value(provider_err("read provider response", &failure)).unwrap();
            assert_eq!(
                (&json["retry"], json.get("account")),
                (&serde_json::json!("unknown"), None)
            );
        }
    }

    #[test]
    fn a_request_that_outgrew_the_window_is_told_how_to_fit_again() {
        use super::super::config::Endpoint;
        use super::super::fakes::{config, fake_provider, request};
        use super::super::redemption::redemption;
        use kernel::AxCode;
        let said = serde_json::json!({
            "type": "error",
            "error": { "type": "invalid_request_error",
                       "message": "prompt is too long: 210000 tokens > 200000 maximum" },
        })
        .to_string();
        let (url, handle) = fake_provider(vec![(400, said)], false);
        let mut endpoint = Endpoint::new(config(&url), redemption()).unwrap();
        let err = endpoint.call(&request()).unwrap_err();
        assert_eq!(
            (
                err.code(),
                err.retry(),
                err.recovery().contains("/new --carry")
            ),
            (&AxCode::Provider, Retry::No, true),
            "{}",
            err.recovery()
        );
        assert!(
            !err.subject().contains("210000"),
            "the body is never quoted back"
        );
        drop(handle.join());
    }
}
