// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
//! OAuth flow: begin, redeem, refresh.
//! OAuth on the wire: PKCE, redeem, refresh.

use super::super::vault::Persistence;
use super::codec::{base64url_nopad, percent_encode};
use super::types::{OauthPending, OauthTokens, TokenRequest};

use kernel::{AxCode, AxError, Payload, Sealed};
use serde_json::{Map, Value};
use zeroize::Zeroizing;

pub fn oauth_begin(
    profile: &crate::oauth_profiles::OauthProfile,
    code_verifier: String,
    state: String,
) -> Result<OauthPending, AxError> {
    if profile.auth_endpoint.is_empty()
        || profile.token_endpoint.is_empty()
        || profile.client_id.is_empty()
    {
        return Err(AxError::failure(
            AxCode::ConfigInvalid,
            "begin oauth",
            format!("{}: profile intelligence incomplete", profile.provider),
        )
        .with_recovery("fill the oauth_profiles row for this provider"));
    }
    if !(43..=128).contains(&code_verifier.len()) {
        return Err(AxError::failure(
            AxCode::InvalidArgs,
            "begin oauth",
            "code verifier length outside 43..=128",
        ));
    }
    // The two values answer different questions: the verifier proves the
    // client that redeems is the client that asked, and the state proves
    // the redirect came back from the request this process started.
    // Reusing one as the other collapses both proofs into one, and at
    // least one provider now rejects it outright. Refused here rather
    // than trusted to whoever wires the call, because this is exactly
    // the shape an implementation copied from elsewhere arrives in.
    if state == code_verifier {
        return Err(AxError::failure(
            AxCode::InvalidArgs,
            "begin oauth",
            "state and code verifier are the same value",
        )
        .with_recovery(
            "draw the state from randomness of its own; it proves the redirect, not the client",
        ));
    }
    let digest = <sha2::Sha256 as sha2::Digest>::digest(code_verifier.as_bytes());
    let challenge = base64url_nopad(&digest);
    let scope = profile.scopes.join(" ");
    let auth_url = format!(
        "{}?code=true&client_id={}&response_type=code&redirect_uri={}&scope={}&code_challenge={}&code_challenge_method=S256&state={}",
        profile.auth_endpoint,
        percent_encode(profile.client_id),
        percent_encode(profile.redirect_uri),
        percent_encode(&scope),
        challenge,
        percent_encode(&state),
    );
    Ok(OauthPending {
        auth_url,
        state,
        code_verifier: Zeroizing::new(code_verifier),
    })
}

/// What a token endpoint hands back. The two secrets stay wrapped from
/// the moment they are parsed: nothing here logs, formats or returns
pub fn oauth_refresh(
    profile: &crate::oauth_profiles::OauthProfile,
    refresh: &Sealed<String>,
    timeout_ms: u64,
) -> Result<OauthTokens, AxError> {
    // The one place the stored token is in plain text, and it is the
    // slot before the wire - the same shape as redemption, which is why
    // this file is on the expose whitelist rather than the caller.
    let body = serde_json::json!({
        "grant_type": "refresh_token",
        "refresh_token": refresh.expose(),
        "client_id": profile.client_id,
    });
    send_token_request(profile.token_endpoint, body.to_string(), timeout_ms)
}

/// Sends the redeem POST and reads the tokens out of the answer.
///
/// The send lives here rather than in `endpoint` because this module is
/// the whole OAuth flow: splitting "build the request" from "send it"
/// across two modules would put one exchange under two authorities.
///
/// # Errors
/// Transport failure, a non-success status, a body that is not JSON, and
/// a body without an access token. None of them quote the body: a token
/// endpoint's error page can contain the code that was just sent.
pub fn oauth_redeem(
    profile: &crate::oauth_profiles::OauthProfile,
    pending: &OauthPending,
    code: &str,
    timeout_ms: u64,
) -> Result<OauthTokens, AxError> {
    let request = oauth_redeem_request(profile, pending, code)?;
    send_token_request(&request.url, request.body, timeout_ms)
}

/// The one exchange with a token endpoint: send, read, refuse without
/// quoting. Both grants use it, so neither can drift.
fn send_token_request(url: &str, body: String, timeout_ms: u64) -> Result<OauthTokens, AxError> {
    // A token endpoint belongs to the provider a person is subscribing
    // to, so there is no endpoint of theirs to carry a setting; the
    // city's own default is what applies, and a loopback URL here is a
    // stand-in a test stood up.
    let client = crate::client_for(kernel::Proxying::ExceptLocal, url)
        .timeout(std::time::Duration::from_millis(timeout_ms))
        .build()
        .map_err(|err| {
            AxError::failure(AxCode::ConfigInvalid, "build http client", err.to_string())
        })?;
    let response = client
        .post(url)
        .header("content-type", "application/json")
        .body(body)
        .send()
        .map_err(|err| {
            AxError::failure(
                AxCode::Provider,
                "redeem an authorization code",
                err.to_string(),
            )
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
    if !status.is_success() {
        // The provider's own words are not quoted: this body is the one
        // place a just-used code can appear in plain text.
        return Err(AxError::failure(
            AxCode::Provider,
            "redeem an authorization code",
            format!("the provider answered {}", status.as_u16()),
        )
        .with_recovery("start the login again; a code can be redeemed once and expires quickly"));
    }
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

/// The redeem POST for the authorization code; the response's token goes
/// straight into the vault via `Custodian::set`.
pub fn oauth_redeem_request(
    profile: &crate::oauth_profiles::OauthProfile,
    pending: &OauthPending,
    code: &str,
) -> Result<TokenRequest, AxError> {
    let body = serde_json::json!({
        "grant_type": "authorization_code",
        "code": code,
        "redirect_uri": profile.redirect_uri,
        "client_id": profile.client_id,
        "code_verifier": pending.code_verifier.as_str(),
        "state": pending.state,
    });
    Ok(TokenRequest {
        url: profile.token_endpoint.to_owned(),
        body: body.to_string(),
    })
}

pub(crate) fn degraded_payload(reason: &str) -> Option<Payload> {
    let mut map = Map::new();
    map.insert("component".to_owned(), Value::String("vault".to_owned()));
    map.insert(
        "fallback".to_owned(),
        Value::String("session-memory".to_owned()),
    );
    map.insert(
        "persistence".to_owned(),
        Value::String(Persistence::ThisProcess.as_str().to_owned()),
    );
    map.insert("reason".to_owned(), Value::String(reason.to_owned()));
    Payload::new(map).ok()
}

#[cfg(test)]
mod tests;
