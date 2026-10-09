// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Handing a credential to this city's enrolment route, and the name
//! that comes back in its place.
//!
//! The plaintext travels in an HTTP body over loopback and never
//! through `argv`, which is readable in the process table, in shell
//! history, and by whatever started this process.

use std::time::Duration;

use kernel::{AxCode, AxError};

use super::{credential, unreachable_city};

/// Splits `realm/name` into its two halves.
///
/// Fail-closed on anything else: a reference with no realm, an empty
/// half, or a second slash is not a credential name this city can hold,
/// and guessing which half was meant would put a key under a name its
/// owner did not choose.
pub(crate) fn split_reference(raw: &str) -> Option<(&str, &str)> {
    let (realm, name) = raw.split_once('/')?;
    if realm.is_empty() || name.is_empty() || name.contains('/') {
        return None;
    }
    Some((realm, name))
}
/// Hands a credential to the local enrolment route and returns the
/// reference that replaces it.
///
/// The value never travels through `argv`, which is readable in the
/// process table, in shell history, and in whatever started this
/// process. That is what makes this better custody than the browser
/// path, where the page holds the plaintext first.
///
/// # Errors
/// Fails when the route refuses - which it does for any peer that is not
/// on this machine - or when the city cannot be reached.
pub(crate) fn enrol(at: &str, realm: &str, name: &str, value: &str) -> Result<String, AxError> {
    let body = serde_json::json!({ "realm": realm, "name": name, "value": value });
    let client = gateway::client_for(kernel::Proxying::ExceptLocal, &format!("http://{at}"))
        .timeout(Duration::from_secs(20))
        .build()
        .map_err(|err| {
            AxError::failure(AxCode::Provider, "build an http client", err.to_string())
                .with_recovery(
                    "check the TLS roots this build was given; the city is on loopback \
                     and no proxy is consulted for it",
                )
        })?;
    // `/enroll` asks for a credential like every door: the city's key
    // from its key file on this machine.
    let mut request = client.post(format!("http://{at}/enroll")).json(&body);
    if let Some(key) = credential(at, None) {
        request = request.bearer_auth(key);
    }
    let answer = request
        .send()
        .map_err(|err| unreachable_city(at, &err.to_string()))?;
    let status = answer.status();
    // The reply body is this city's own text, so quoting it is quoting
    // ourselves rather than a stranger's error page.
    let said = answer.text().unwrap_or_default();
    match status.as_u16() {
        201 => Ok(said),
        // The city took it and has not said what became of it. Reported
        // as a failure because the caller must not go on as though the
        // reference resolves - but with the city's own words, which say
        // what to check rather than what went wrong.
        202 => Err(
            AxError::failure(AxCode::CredentialMissing, "enrol a credential", said).with_recovery(
                "the city is busy; ask it again once the run it is inside has finished",
            ),
        ),
        code => Err(AxError::failure(
            AxCode::CredentialMissing,
            "enrol a credential",
            format!("the city answered {code}: {said}"),
        )
        .with_recovery("enrolment is refused for any peer that is not on this machine")),
    }
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
    use super::split_reference;

    #[test]
    fn a_reference_is_a_realm_and_a_name() {
        assert_eq!(
            split_reference("modelscope/api"),
            Some(("modelscope", "api"))
        );
    }

    #[test]
    fn anything_that_is_not_two_halves_is_refused_rather_than_guessed() {
        for raw in ["api", "/api", "modelscope/", "", "a/b/c"] {
            assert_eq!(split_reference(raw), None, "{raw} was accepted");
        }
    }
}
