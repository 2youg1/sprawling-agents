// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

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
                let json = serde_json::to_value(provider_err("call provider", &failure)).unwrap();
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
                let account = (status == 401 || exhausted).then(|| serde_json::json!("advance"));
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
        let json = serde_json::to_value(provider_err("read provider response", &failure)).unwrap();
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
