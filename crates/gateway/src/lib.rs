// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! gateway — duty routing, dialects, credentials and cost for model calls.
//! Adapters implement the `kernel::model` seam; the dialect face is pure
//! translation (deliberately not a trait).

mod admission;
mod anthropic;
mod cost;
mod credential;
mod dialect;
mod endpoint;
mod fallback;
mod market;
mod mismatch;
mod native;
mod oauth_profiles;
mod openai;
mod reach;
mod router;
mod transcribe;

pub use admission::{AdmissionState, AdmissionVerdict, ProviderOutcome};
pub use cost::{CallCost, CostSource, settle};
pub use credential::oauth_refresh;
pub use credential::{Captured, Custodian, Described, EnvReader, Persistence};
pub use credential::{OauthPending, OauthTokens, TokenRequest, oauth_begin};
pub use credential::{oauth_random, oauth_redeem, oauth_redeem_request};
pub use dialect::{ImageBytes, increment_of, request_wire, response_from_wire};
pub use dialect::{response_wire, settled_from_stream};
pub use endpoint::adapter_for;
pub use endpoint::{AuthSpec, CALL_TIMEOUT_MS, Endpoint, EndpointConfig, SecretResolver};
pub use endpoint::{ImageResolver, ModelFacts, Redemption};
pub use fallback::{Fallback, Retreat, retreat_payload};
pub use market::{InputKinds, MarketSnapshot, ModelEntry};
pub use native::{Native, NativeConfig};
pub use oauth_profiles::{OAUTH_PROFILES, OauthProfile, profile};
pub use reach::{client_for, is_local, reach, through};
pub use router::{AttachedEndpoint, Chosen, EndpointBook, EndpointTuning};
pub use router::{attached_payload, selected_payload};
pub use transcribe::{AudioType, Recording, Transcriber, TranscriberConfig, transcriber_for};
