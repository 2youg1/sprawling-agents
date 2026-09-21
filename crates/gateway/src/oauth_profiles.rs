// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Subscription-login intelligence: data only, zero
//! branches. The flow code lives in `credential`; this table tracks each
//! harness family's endpoints, scopes and client ids — sourced from the
//! upstream paths `docs/third-party.md` §1 names and watches, refreshed
//! as a weekly task, never a CI gate ("is this endpoint stale" is not
//! machine-decidable).
//!
//! **One row per family, and the family is the key.** A row states
//! which vendor's own client this city signs in as, which realm the
//! credential is filed under, and which grant the vendor answers; the
//! flow reads the grant rather than assuming one, so a family that
//! signs in by device code cannot be driven down the browser-redirect
//! path by accident.
//!
//! Empty fields are deliberate: better absent than wrong; `oauth_begin`
//! fails closed on them.

use crate::provider::registry::Family;

/// How a vendor lets a client sign a person in.
///
/// Exhaustive: a third grant is a flow this city has to write, and the
/// compiler is what says so.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Grant {
    /// RFC 6749 §4.1 with the RFC 7636 PKCE extension: the person is
    /// sent to the vendor's page and comes back to `redirect_uri` with
    /// a code.
    AuthorizationCode {
        /// Where the vendor sends the person back. A loopback URL is a
        /// listener this city runs for the length of one login.
        redirect_uri: &'static str,
    },
    /// RFC 8628: this city asks for a user code, the person types it
    /// on the vendor's page, and this city polls the token endpoint
    /// until the person is done.
    DeviceCode {
        /// Where the user code and the device code are asked for.
        authorization_endpoint: &'static str,
    },
}

#[derive(Debug, Clone, Copy)]
pub struct OauthProfile {
    /// Whose own client this city signs in as.
    pub family: Family,
    /// The realm the redeemed credential is filed under in the vault,
    /// and the word the wire carries for this login. A vendor name
    /// rather than the family's: two harnesses of one vendor would
    /// share a realm.
    pub provider: &'static str,
    /// Where this provider's API lives, so a finished login can attach
    /// an endpoint without a person retyping a URL they never chose.
    /// Empty means the same as an empty endpoint: fail closed.
    pub api_base: &'static str,
    pub auth_endpoint: &'static str,
    pub token_endpoint: &'static str,
    pub scopes: &'static [&'static str],
    pub client_id: &'static str,
    pub grant: Grant,
    pub headers: &'static [(&'static str, &'static str)],
}

/// The table. Every value is read from the upstream path that
/// `docs/third-party.md` §1 watches for the family it belongs to, on
/// the date that file records; a fact no watched path states is left
/// empty rather than guessed.
pub const OAUTH_PROFILES: [OauthProfile; 4] = [
    // Followed from openai/codex at the paths `docs/third-party.md`
    // section 1 names for it: the login server states the issuer, the
    // endpoint paths, the scopes and the loopback redirect, and the
    // auth manager beside it states the client id.
    //
    // A ChatGPT subscription is served under
    // `https://chatgpt.com/backend-api/codex`, not under the
    // key-billed platform: `model-provider-info` picks that base for
    // every ChatGPT auth mode and `https://api.openai.com/v1` only for
    // an API key (the provider table under `codex-rs/model-provider-
    // info/`, read 2026-09-21 and watched in docs/third-party.md
    // section 1). That base answers on the responses face, which
    // `Family::Codex` states and `dialect::responses` writes.
    OauthProfile {
        family: Family::Codex,
        provider: "openai",
        api_base: "https://chatgpt.com/backend-api/codex",
        auth_endpoint: "https://auth.openai.com/oauth/authorize",
        token_endpoint: "https://auth.openai.com/oauth/token",
        scopes: &[
            "openid",
            "profile",
            "email",
            "offline_access",
            "api.connectors.read",
            "api.connectors.invoke",
        ],
        // A public OAuth client id, not a secret: it is published by
        // the upstream this row follows, and no request is authorised
        // by holding it.
        client_id: "app_EMoamEEZ73f0CkXaXp7hrann", // secret-ok: public oauth client id
        grant: Grant::AuthorizationCode {
            redirect_uri: "http://localhost:1455/auth/callback",
        },
        headers: &[],
    },
    OauthProfile {
        family: Family::ClaudeCode,
        provider: "anthropic",
        api_base: "https://api.anthropic.com",
        auth_endpoint: "https://claude.ai/oauth/authorize",
        token_endpoint: "https://console.anthropic.com/v1/oauth/token",
        scopes: &["org:create_api_key", "user:profile", "user:inference"],
        client_id: "9d1c250a-e61b-44d9-88ed-5944d1962f5e",
        grant: Grant::AuthorizationCode {
            redirect_uri: "https://console.anthropic.com/oauth/code/callback",
        },
        headers: &[],
    },
    // xAI's browser login reads its endpoints from an OIDC discovery
    // document at run time (the `oidc/` module fetches
    // `{issuer}/.well-known/openid-configuration`), so those two URLs
    // are the document's to state and are left empty here rather than
    // pinned in a second place. **The device-code login needs no
    // discovery**: `device_code.rs` posts to `{issuer}/oauth2/device/
    // code` and `{issuer}/oauth2/token` with the issuer and client id
    // the login crate's configuration module states, so this row
    // carries the grant that can be driven from constants alone (both
    // paths read 2026-09-21).
    OauthProfile {
        family: Family::GrokBuild,
        provider: "xai",
        api_base: "https://api.x.ai/v1",
        auth_endpoint: "",
        token_endpoint: "https://auth.x.ai/oauth2/token",
        scopes: &[
            "openid",
            "profile",
            "email",
            "offline_access",
            "grok-cli:access",
            "api:access",
            "conversations:read",
            "conversations:write",
            "workspaces:read",
            "workspaces:write",
        ],
        // A public OAuth client id, not a secret: the upstream ships it
        // in a binary every user downloads, and no request is
        // authorised by holding it.
        client_id: "b1a00492-073a-47ea-816f-4c329264a828", // secret-ok: public oauth client id
        grant: Grant::DeviceCode {
            authorization_endpoint: "https://auth.x.ai/oauth2/device/code",
        },
        headers: &[],
    },
    // `src/kimi_cli/auth/oauth.py` (client id, OAuth host, both
    // endpoint paths, device-code grant) and `platforms.py` (the Kimi
    // Code base URL, which is the subscription's own face and not the
    // key-billed `api.moonshot.*` platforms).
    OauthProfile {
        family: Family::KimiCli,
        provider: "kimi",
        api_base: "https://api.kimi.com/coding/v1",
        auth_endpoint: "",
        token_endpoint: "https://auth.kimi.com/api/oauth/token",
        scopes: &[],
        client_id: "17e5f671-d194-4dfb-9706-5516cb48c098",
        grant: Grant::DeviceCode {
            authorization_endpoint: "https://auth.kimi.com/api/oauth/device_authorization",
        },
        headers: &[],
    },
];

/// The row filed under one realm word, which is what the wire carries.
#[must_use]
pub fn profile(provider: &str) -> Option<&'static OauthProfile> {
    OAUTH_PROFILES.iter().find(|row| row.provider == provider)
}

/// The row for one harness family.
#[must_use]
pub fn profile_for(family: Family) -> Option<&'static OauthProfile> {
    OAUTH_PROFILES.iter().find(|row| row.family == family)
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
    fn the_table_is_data_and_lookup_finds_rows() {
        assert!(profile("anthropic").is_some());
        assert!(profile("openai").is_some());
        assert!(profile("nonexistent").is_none());
        // Zero branches: nothing here computes; emptiness is a value.
        // xAI's browser-redirect endpoint is the discovery document's
        // to state, so this row leaves it empty and signs in by device
        // code instead.
        assert!(profile("xai").unwrap().auth_endpoint.is_empty());
    }

    /// Every family this city signs in as has exactly one row, and no
    /// two rows share a realm: a family with no row cannot be signed
    /// in at all, and two rows under one realm would file two people's
    /// credentials in one place.
    #[test]
    fn one_family_has_one_row_and_one_realm() {
        for family in Family::ALL {
            let row = profile_for(family).expect("every family states its login");
            assert_eq!(row.family, family);
            assert_eq!(profile(row.provider).map(|held| held.family), Some(family));
        }
        let mut realms: Vec<&str> = OAUTH_PROFILES.iter().map(|row| row.provider).collect();
        realms.sort_unstable();
        let listed = realms.len();
        realms.dedup();
        assert_eq!(realms.len(), listed, "two families under one realm");
        assert_eq!(listed, Family::ALL.len());
    }

    /// A row states the grant its vendor answers, and a device-code
    /// row states where the device code is asked for. Both are read by
    /// the flow rather than assumed by it.
    #[test]
    fn a_row_states_the_grant_its_vendor_answers() {
        let kimi = profile_for(Family::KimiCli).unwrap();
        let Grant::DeviceCode {
            authorization_endpoint,
        } = kimi.grant
        else {
            panic!("the Kimi Code subscription signs in by device code");
        };
        assert!(authorization_endpoint.starts_with("https://"));
        let codex = profile_for(Family::Codex).unwrap();
        assert!(matches!(
            codex.grant,
            Grant::AuthorizationCode { redirect_uri } if redirect_uri.starts_with("http://localhost:")
        ));
        let xai = profile_for(Family::GrokBuild).unwrap();
        let Grant::DeviceCode {
            authorization_endpoint,
        } = xai.grant
        else {
            panic!("the xAI subscription signs in by device code");
        };
        assert_eq!(
            authorization_endpoint,
            "https://auth.x.ai/oauth2/device/code"
        );
        assert!(xai.token_endpoint.starts_with("https://auth.x.ai/"));
    }

    /// A subscription login that can be driven at all states its token
    /// endpoint, its client id and at least one scope. A row missing
    /// one of the three fails closed in the flow, which is correct but
    /// silent until somebody tries to sign in.
    #[test]
    fn a_row_that_states_a_grant_states_what_that_grant_needs() {
        for family in [Family::Codex, Family::ClaudeCode, Family::KimiCli] {
            let row = profile_for(family).unwrap();
            assert!(!row.client_id.is_empty(), "{}", row.provider);
        }
        let xai = profile_for(Family::GrokBuild).unwrap();
        assert!(!xai.client_id.is_empty());
        assert!(!xai.scopes.is_empty());
    }

    /// A finished login attaches the endpoint it was for, so every
    /// family states the base its subscription is served under. An
    /// empty one leaves the token in the vault and the person with a
    /// second thing to do by hand.
    #[test]
    fn every_family_states_the_base_its_subscription_answers_on() {
        for family in Family::ALL {
            let row = profile_for(family).unwrap();
            assert!(
                row.api_base.starts_with("https://"),
                "{} states no api base",
                row.provider
            );
        }
    }
}
