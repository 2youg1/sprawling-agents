// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::time::Duration;

use kernel::{AxCode, AxError};

use super::{credential, unreachable_city};

pub(crate) fn split_reference(raw: &str) -> Option<(&str, &str)> {
    let (realm, name) = raw.split_once('/')?;
    if realm.is_empty() || name.is_empty() || name.contains('/') {
        return None;
    }
    Some((realm, name))
}

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

    let mut request = client.post(format!("http://{at}/enroll")).json(&body);
    if let Some(key) = credential(at, None)? {
        request = request.bearer_auth(key);
    }
    let answer = request
        .send()
        .map_err(|err| unreachable_city(at, &err.to_string()))?;
    let status = answer.status();

    let said = answer.text().unwrap_or_default();
    match status.as_u16() {
        201 => Ok(said),

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
