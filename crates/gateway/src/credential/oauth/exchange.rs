// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! One POST to a token endpoint, and the tokens its answer carries
//! (shape 2 adapter).
//!
//! **Both grants end at the same endpoint.** The browser-redirect flow
//! sends a JSON body and the device-code flow sends a form body, but
//! from the status line onwards the two are one exchange: the same
//! `access_token`, `refresh_token` and `expires_in` are read out of the
//! same answer, and a refusal from either must not quote the body,
//! which holds a live code. Written once here so that a change to what
//! a token answer looks like cannot reach one grant and miss the other.
//!
//! The status is handed back rather than judged: the device-code flow
//! reads its refusals out of the body of a non-success answer (RFC 8628
//! §3.5), where the redirect flow has nothing to do with one but stop.

use kernel::{AxCode, AxError};
use serde_json::Value;
use zeroize::Zeroizing;

use super::types::OauthTokens;

/// What a token endpoint answered: whether it succeeded, the status it
/// answered with, and the JSON it carried.
pub(super) struct Answer {
    pub(super) ok: bool,
    pub(super) status: u16,
    pub(super) body: Value,
}

/// The content type a form-encoded token request travels under (RFC
/// 6749 §4.1.3, carried into RFC 8628 §3.4).
pub(super) const FORM_CONTENT_TYPE: &str = "application/x-www-form-urlencoded";

/// The content type the JSON-bodied redeem and refresh travel under.
const JSON_CONTENT_TYPE: &str = "application/json";

/// The redeem or refresh POST, with a JSON body.
pub(super) fn post_json(url: &str, body: String, timeout_ms: u64) -> Result<Answer, AxError> {
    post(url, JSON_CONTENT_TYPE, body, timeout_ms)
}

/// The device-code POST, with a form body.
pub(super) fn post_form(url: &str, body: String, timeout_ms: u64) -> Result<Answer, AxError> {
    post(url, FORM_CONTENT_TYPE, body, timeout_ms)
}

/// # Errors
/// `E_CONFIG_INVALID` when no HTTP client can be built for this host,
/// `E_PROVIDER` when the send fails or the answer is not JSON. Neither
/// repeats the body.
fn post(url: &str, content_type: &str, body: String, timeout_ms: u64) -> Result<Answer, AxError> {
    // A token endpoint belongs to the provider a person is subscribing
    // to, so there is no endpoint of theirs to carry a setting; the
    // city's own default is what applies, and a loopback URL here is a
    // stand-in a test stood up.
    let client = crate::client_for(kernel::Proxying::ExceptLocal, url)
        .timeout(std::time::Duration::from_millis(timeout_ms))
        .build()
        .map_err(|err| {
            AxError::failure(AxCode::ConfigInvalid, "build http client", err.to_string())
                .with_recovery(
                    "check the proxy settings this machine exports (`HTTPS_PROXY`, \
                     `NO_PROXY`) and the TLS roots this build was given",
                )
        })?;
    let response = client
        .post(url)
        .header("content-type", content_type)
        .body(body)
        .send()
        .map_err(|err| {
            AxError::failure(AxCode::Provider, "ask a token endpoint", err.to_string())
                .with_recovery("check the network and start the login again")
        })?;
    let status = response.status();
    let body: Value = response.json().map_err(|err| {
        AxError::failure(
            AxCode::Provider,
            "read a token answer",
            format!("{}: {err}", status.as_u16()),
        )
        .with_recovery("the provider answered something this version cannot read")
    })?;
    Ok(Answer {
        ok: status.is_success(),
        status: status.as_u16(),
        body,
    })
}

/// The tokens one successful answer carries.
///
/// # Errors
/// `E_PROVIDER` when the answer states no access token, which is the
/// one field that makes the rest of the login possible.
pub(super) fn tokens(body: &Value) -> Result<OauthTokens, AxError> {
    let access = body
        .get("access_token")
        .and_then(Value::as_str)
        .ok_or_else(|| {
            AxError::failure(
                AxCode::Provider,
                "read a token answer",
                "no access_token in the answer",
            )
            .with_recovery("the provider answered something this version cannot read")
        })?;
    Ok(OauthTokens {
        access: Zeroizing::new(access.to_owned()),
        refresh: body
            .get("refresh_token")
            .and_then(Value::as_str)
            .map(|token| Zeroizing::new(token.to_owned())),
        expires_in_s: body.get("expires_in").and_then(Value::as_u64),
    })
}
