// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! gateway — duty routing, dialects, credentials and cost for model calls.
//! Adapters implement the `kernel::model` seam; the dialect face is pure
//! translation (deliberately not a trait).

mod adviser;
mod anthropic;
mod cost;
mod credential;
mod dialect;
mod endpoint;
mod market;
mod mismatch;
mod openai;
mod provider;
mod reach;
mod router;
mod transcribe;

pub use adviser::{AdviserClient, Question};
pub use cost::settle;
pub use credential::{Custodian, Custody, Persistence, Store};
pub use dialect::{ImageBytes, response_from_wire};
pub use endpoint::adapter_for;
pub use endpoint::{AuthSpec, Endpoint, EndpointConfig, HeaderValue, SecretResolver};
pub use endpoint::{ModelFacts, Redemption};
pub use market::{InputKinds, MarketSnapshot, ModelEntry};
pub use provider::ceiling::{CeilingSource, OutputCeiling, Stated, Target};
pub use provider::modality::call::{Ranks, Vectors};
pub use provider::modality::embedding::{EmbeddingRequest, Embeddings};
pub use provider::modality::rerank::{Rank, Ranking, RerankRequest};
pub use provider::preset::{HostPreset, KnownHost, ModelPreset, known_hosts, window_for};
pub use provider::registry::{ConnectionKind, resolve as resolve_connection};
pub use reach::{client_for, is_local, reach, through};
pub use router::{AttachedEndpoint, Chosen, EndpointBook, EndpointTuning};
pub use router::{DialectHint, normalise_entered};
pub use router::{TuningDefaults, attached_payload, selected_payload};
pub use transcribe::{AudioType, Recording, Transcriber, transcriber_for};
