// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Signing in, and the vault: a subscription login in two steps, the
//! renewal that runs before a credential is used, the one way a
//! credential enters, and the closure that redeems a reference.

use std::sync::Arc;

use kernel::Payload;
use kernel::{AxCode, AxError, EventKind};

use crate::serving::random_token;

use super::super::{RunWorker, now_ms};
use super::{Credential, Entered, PROBE_TIMEOUT_MS, poisoned_vault, subscription};

impl RunWorker {
    /// Renews a subscription credential that is about to stop working.
    ///
    /// Called before the credential is used rather than after a call
    /// fails: a 401 costs a whole turn to discover, and the expiry the
    /// provider stated is a fact this city already wrote down. A
    /// provider with no recorded expiry is left alone - not knowing when
    /// something expires is not a reason to renew it every time.
    ///
    /// # Errors
    /// Propagates the token endpoint's refusal. A refused refresh means
    /// the login is over, and saying so beats retrying what will fail
    /// again.
    pub(in crate::assembly) fn renew_if_stale(&mut self, provider: &str) -> Result<(), AxError> {
        let Some(expires_at) = self.expiries.of(provider) else {
            return Ok(());
        };
        // A minute of margin: a call started now must still be holding a
        // working credential when it reaches the far end.
        if now_ms()?.value().saturating_add(60_000) < expires_at {
            return Ok(());
        }
        let Some(profile) = gateway::profile(provider) else {
            return Ok(());
        };
        let stored = subscription::oauth_refresh_ref(provider)?;
        let refresh = {
            let vault = self.vault.lock().map_err(|_| poisoned_vault())?;
            vault.resolve(&stored)?
        };
        let tokens = gateway::oauth_refresh(profile, &refresh, PROBE_TIMEOUT_MS)?;
        let access = subscription::oauth_ref(provider)?;
        {
            let mut vault = self.vault.lock().map_err(|_| poisoned_vault())?;
            vault.set(&access, tokens.access)?;
            if let Some(next) = tokens.refresh {
                vault.set(&stored, next)?;
            }
        }
        let mut map = serde_json::Map::new();
        map.insert(
            "ref".to_owned(),
            serde_json::Value::String(access.to_string()),
        );
        map.insert(
            "origin".to_owned(),
            serde_json::Value::String(format!("{provider}-renewal")),
        );
        if let Some(seconds) = tokens.expires_in_s {
            let at = now_ms()?
                .value()
                .saturating_add(seconds.saturating_mul(1_000));
            map.insert(
                "expires_at".to_owned(),
                serde_json::Value::Number(at.into()),
            );
        }
        self.record(EventKind::SecretCaptured, Payload::new(map)?)
    }

    /// One step of a subscription login.
    ///
    /// Two steps rather than one because a person stands between them:
    /// the provider shows them a code after they approve, and they bring
    /// it back. Nothing listens on a port for it — the profile's own
    /// redirect is the provider's page, so a listener would be a second
    /// way in that nobody uses.
    ///
    /// The two grants share those two steps. A vendor that signs in by
    /// device code shows the person a short code instead of a
    /// redirect, and the second step carries that code back: the
    /// person approving on another device is the only thing this city
    /// can wait for, and waiting for it inside the worker would park
    /// the whole city for as long as they take.
    pub(in crate::assembly) fn login(
        &mut self,
        provider: &str,
        step: channels::LoginStep,
    ) -> Result<(), AxError> {
        let profile = *gateway::profile(provider).ok_or_else(|| {
            // The list is read out of the table rather than written
            // here: a fifth row would otherwise reach a person as a
            // sentence naming four.
            let known: Vec<&str> = gateway::OAUTH_PROFILES
                .iter()
                .map(|row| row.provider)
                .collect();
            AxError::failure(
                AxCode::ConfigInvalid,
                "begin a subscription login",
                provider.to_owned(),
            )
            .with_recovery(format!(
                "this build knows the subscription flow of: {}; \
                 other providers attach with an API key",
                known.join(", ")
            ))
        })?;
        self.login_with(&profile, provider, step)
    }

    /// The same login against a profile the caller supplies. The lookup
    /// is the only thing this does not do, which is what lets a test
    /// point the flow at a server it controls without the production
    /// path growing an override nobody in production would set.
    pub(in crate::assembly) fn login_with(
        &mut self,
        profile: &gateway::OauthProfile,
        provider: &str,
        step: channels::LoginStep,
    ) -> Result<(), AxError> {
        match step {
            channels::LoginStep::Begin => {
                // Which grant a vendor answers is the profile's
                // statement, and the branch is here because the two
                // flows need different things this crate owns: the
                // redirect draws entropy, and the device code reads
                // the clock. Gateway samples neither.
                let pending = match profile.grant {
                    // Two independent draws. The verifier proves the
                    // client that redeems is the client that asked;
                    // the state proves the redirect answers this
                    // request. One value doing both jobs proves
                    // neither, and `oauth_begin` refuses it.
                    gateway::Grant::AuthorizationCode { .. } => gateway::OauthPending::Redirect(
                        gateway::oauth_begin(profile, random_token(48)?, random_token(24)?)?,
                    ),
                    gateway::Grant::DeviceCode { .. } => {
                        gateway::device_login_begin(profile, now_ms()?.value(), PROBE_TIMEOUT_MS)?
                    }
                };
                let mut map = serde_json::Map::new();
                map.insert(
                    "provider".to_owned(),
                    serde_json::Value::String(provider.to_owned()),
                );
                // The URL carries a PKCE challenge and a state, both of
                // which are public by design; no credential exists yet.
                // A device login's page is public for the same reason:
                // the code that pairs with it stays in this process.
                map.insert(
                    "auth_url".to_owned(),
                    serde_json::Value::String(pending.open_url().to_owned()),
                );
                // The short code the person types on that page, for
                // the grant that has one. Absent rather than empty: a
                // redirect login has no such code, and a blank one on
                // the page would read as a code that failed to arrive.
                if let Some(user_code) = pending.user_code() {
                    map.insert(
                        "user_code".to_owned(),
                        serde_json::Value::String(user_code.to_owned()),
                    );
                }
                self.logins.insert(provider.to_owned(), pending);
                self.record(EventKind::LoginStarted, Payload::new(map)?)
            }
            channels::LoginStep::Code { code } => {
                let asked_at = now_ms()?.value();
                let pending = self.logins.get_mut(provider).ok_or_else(|| {
                    AxError::failure(
                        AxCode::CredentialMissing,
                        "redeem an authorization code",
                        provider.to_owned(),
                    )
                    .with_recovery(
                        "start the login first; the code answers a request this process made",
                    )
                })?;
                let tokens = match pending {
                    gateway::OauthPending::Redirect(redirect) => {
                        gateway::oauth_redeem(profile, redirect, &code, PROBE_TIMEOUT_MS)?
                    }
                    gateway::OauthPending::Device(device) => {
                        match device.ask(profile, &code, asked_at)? {
                            gateway::DeviceStep::Signed(tokens) => tokens,
                            gateway::DeviceStep::NotYet { seconds } => {
                                return Err(not_approved_yet(provider, seconds));
                            }
                        }
                    }
                };
                // Spent, and only now: a person who mistyped keeps the
                // login they began rather than starting a new one.
                self.logins.remove(provider);
                let access = subscription::oauth_ref(provider)?;
                {
                    let mut vault = self.vault.lock().map_err(|_| poisoned_vault())?;
                    vault.set(&access, tokens.access)?;
                    if let Some(refresh) = tokens.refresh {
                        let reference = subscription::oauth_refresh_ref(provider)?;
                        vault.set(&reference, refresh)?;
                    }
                }
                let mut map = serde_json::Map::new();
                map.insert(
                    "ref".to_owned(),
                    serde_json::Value::String(access.to_string()),
                );
                map.insert(
                    "origin".to_owned(),
                    serde_json::Value::String(format!("{provider}-subscription")),
                );
                // When it stops working, in the city's own clock. Not a
                // secret, and the one fact that decides whether the next
                // call must renew first.
                if let Some(seconds) = tokens.expires_in_s {
                    let at = now_ms()?
                        .value()
                        .saturating_add(seconds.saturating_mul(1_000));
                    map.insert(
                        "expires_at".to_owned(),
                        serde_json::Value::Number(at.into()),
                    );
                }
                self.record(EventKind::SecretCaptured, Payload::new(map)?)?;
                if profile.api_base.is_empty() {
                    return Err(AxError::failure(
                        AxCode::ConfigInvalid,
                        "attach the endpoint this login is for",
                        provider.to_owned(),
                    )
                    .with_recovery(
                        "the token is in the vault; attach the endpoint by hand until this \
                         provider's api base is known",
                    ));
                }
                // The person logged in to use it, so the endpoint they
                // logged into is attached here rather than left as a
                // second thing to remember.
                self.attach_endpoint(
                    Entered {
                        name: provider.to_owned(),
                        base_url: profile.api_base.to_owned(),
                        // Which face a subscription answers on is the
                        // family's statement, read through the
                        // connection this registration is: a second
                        // mapping from a provider word to a dialect
                        // sent Codex's subscription to the chat face.
                        dialect: gateway::ConnectionKind::Harness(profile.family).wire(),
                        credential: Credential::Subscription {
                            reference: access.to_string(),
                        },
                        tuning: gateway::EndpointTuning::default(),
                    },
                    &[],
                )
            }
        }
    }

    /// Puts one credential in the vault. Nothing about it reaches the
    /// ledger but the fact that it happened.
    ///
    /// The vault key and the `ref` the record states are one value built
    /// once here: a route that announced a reference it spelled itself
    /// would answer with a place the vault may never have been told.
    pub(in crate::assembly) fn put_secret(
        &mut self,
        realm: String,
        name: String,
        value: kernel::Sealed<String>,
    ) -> Result<(), AxError> {
        let reference = kernel::SecretRef::new(&realm, &name)?;
        {
            let mut vault = self.vault.lock().map_err(|_| poisoned_vault())?;
            vault.set(&reference, value.into_vault_value())?;
        }
        let mut map = serde_json::Map::new();
        map.insert(
            "ref".to_owned(),
            serde_json::Value::String(reference.to_string()),
        );
        map.insert(
            "origin".to_owned(),
            serde_json::Value::String("enrolment".to_owned()),
        );
        self.record(EventKind::SecretCaptured, Payload::new(map)?)
    }

    /// The redemption closure the adapters take: one resolve per call,
    /// nothing cached, the lock held only while the vault is read.
    pub(in crate::assembly) fn resolver(&self) -> gateway::SecretResolver {
        resolving(Arc::clone(&self.vault))
    }

    /// The vault this city resolves credentials through.
    ///
    /// Shared rather than lent, because one route resolves a credential
    /// off this thread: the transcription door runs on a socket task and
    /// must reach the same vault, so that "a credential is resolved at
    /// the last moment and exposed only while a header is written" stays
    /// one path rather than two.
    pub(crate) fn vault_handle(&self) -> Arc<std::sync::Mutex<gateway::Custodian>> {
        Arc::clone(&self.vault)
    }
}

/// The person has not finished approving a device-code login, and the
/// vendor states how long to leave it before asking again.
///
/// A refusal rather than a quiet success: nothing was signed in, and
/// the login is still in flight, so the one thing the person can do is
/// finish on the vendor's page and say so again.
fn not_approved_yet(provider: &str, seconds: u64) -> AxError {
    AxError::failure(
        AxCode::CredentialMissing,
        "finish a device-code login",
        provider.to_owned(),
    )
    .with_recovery(format!(
        "approve the login on the page this city opened, then enter the code again; \
         the vendor asks for {seconds} seconds between tries"
    ))
}

/// One resolver over one vault. A fresh one per operation, because
/// `SecretResolver` is spent by the endpoint it is handed to.
pub(crate) fn resolving(
    vault: Arc<std::sync::Mutex<gateway::Custodian>>,
) -> gateway::SecretResolver {
    Box::new(move |reference: &kernel::SecretRef| {
        let held = vault.lock().map_err(|_| poisoned_vault())?;
        held.resolve(reference)
    })
}
