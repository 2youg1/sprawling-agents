// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! One endpoint's HTTP client, and the one place a client is built.
//!
//! Every blocking client starts its own `reqwest-internal-sync-runtime`
//! thread and keeps its own connection pool. A client built per call
//! therefore held one thread per concurrent run and handshook again on
//! every call, so the book keeps one [`Transport`] per endpoint and every
//! call to that endpoint clones its client (`crates/gateway/spec/Endpoint/Transport.lean` §8-3).

use std::sync::{Arc, OnceLock};
use std::time::Duration;
use std::time::Instant;

use kernel::{AxCode, AxError, Proxying};

use super::config::{Endpoint, EndpointConfig};
use super::permit::{Gate, Gated};
use super::redemption::Redemption;
use crate::concurrency::MaxInFlight;

/// One endpoint's client, built the first time a call needs it.
///
/// Clones share the slot, so a copy of the book hands out the same
/// client. Building is deferred because a book is folded on every
/// replay, and an endpoint the city never calls should hold no thread.
#[derive(Debug, Clone, Default)]
pub(crate) struct Transport {
    slot: Arc<OnceLock<reqwest::blocking::Client>>,
    /// The permits every model call to this endpoint waits on
    /// (`crates/gateway/Spec.lean` §8-6).
    gate: Arc<Gate>,
    /// The step a test takes after the client is configured and before
    /// it is built (`crates/gateway/spec/Reach/Resolve.lean` §8-32).
    #[cfg(test)]
    detour: Option<Detour>,
}

impl Transport {
    /// How many calls the gate lets be in flight now.
    #[cfg(test)]
    pub(crate) fn limit(&self) -> u32 {
        self.gate.limit()
    }

    /// A transport whose gate lets `ceiling` calls be in flight at once.
    pub(crate) fn admitting(ceiling: MaxInFlight) -> Transport {
        Transport {
            gate: Arc::new(Gate::admitting(ceiling)),
            ..Transport::default()
        }
    }

    /// The shared client, built from `shape` if this is the first call.
    ///
    /// Two callers racing on an empty slot each build one; the slot keeps
    /// the first and the other is dropped with its thread.
    fn client(&self, shape: &ClientShape) -> Result<reqwest::blocking::Client, AxError> {
        if let Some(client) = self.slot.get() {
            return Ok(client.clone());
        }
        let built = self.build(shape)?;
        Ok(self.slot.get_or_init(|| built).clone())
    }

    /// A transport whose client takes `step` after the endpoint's own
    /// configuration: what a test uses to reach a stand-in under a
    /// preset host's name (`reach::resolve`).
    #[cfg(test)]
    pub(crate) fn detoured(
        step: impl Fn(reqwest::blocking::ClientBuilder) -> reqwest::blocking::ClientBuilder
        + Send
        + Sync
        + 'static,
    ) -> Transport {
        Transport {
            slot: Arc::default(),
            gate: Arc::default(),
            detour: Some(Detour(Arc::new(step))),
        }
    }

    fn build(&self, shape: &ClientShape) -> Result<reqwest::blocking::Client, AxError> {
        let builder = crate::client_for(shape.proxying, &shape.url)
            .timeout(Duration::from_millis(shape.timeout_ms));
        #[cfg(test)]
        let builder = match &self.detour {
            Some(Detour(step)) => step(builder),
            None => builder,
        };
        builder.build().map_err(|err| {
            AxError::failure(AxCode::ConfigInvalid, "build http client", err.to_string())
                .with_recovery(
                    "check this endpoint's `base_url` and the proxy settings this \
                     machine exports (`HTTPS_PROXY`, `NO_PROXY`)",
                )
        })
    }
}

/// What an endpoint's shared client is built from: the fields of an
/// [`EndpointConfig`] the client reads, and nothing a request carries.
#[derive(Debug, Clone)]
pub(crate) struct ClientShape {
    /// The chat URL: which proxy rule applies is decided from its host.
    pub(crate) url: String,
    pub(crate) proxying: Proxying,
    pub(crate) timeout_ms: u64,
}

impl ClientShape {
    fn of(config: &EndpointConfig) -> ClientShape {
        ClientShape {
            url: config.base_url.clone(),
            proxying: config.proxying,
            timeout_ms: config.timeout_ms,
        }
    }
}

/// One endpoint's connection, opened before its first call so that call
/// does not pay to resolve, connect and shake hands
/// (`crates/gateway/spec/Endpoint/Transport.lean` D26).
///
/// It holds no credential, no redemption and no header a person
/// entered, so the one request it sends cannot carry a key.
#[derive(Debug)]
pub struct WarmUp {
    transport: Transport,
    shape: ClientShape,
    models_url: String,
}

impl WarmUp {
    pub(crate) fn new(transport: Transport, shape: ClientShape, models_url: String) -> WarmUp {
        WarmUp {
            transport,
            shape,
            models_url,
        }
    }

    /// Builds the endpoint's shared client and sends one `GET` to its
    /// model list, read to the end so the connection goes back to the
    /// pool. Blocks for that one round trip; start it on a thread of
    /// its own.
    ///
    /// Reports nothing: a client that cannot be built, a connection
    /// refused, any status and a body cut short each end this warm-up
    /// alone, and the endpoint's first call meets the same failure and
    /// reports it with its own action and recovery.
    pub fn open(self) {
        let Ok(client) = self.transport.client(&self.shape) else {
            return;
        };
        if let Ok(answer) = client.get(&self.models_url).send() {
            drop(answer.bytes());
        }
    }
}

/// A step after a client's configuration, held so a transport stays
/// `Clone` and `Debug`.
#[cfg(test)]
#[derive(Clone)]
struct Detour(
    Arc<dyn Fn(reqwest::blocking::ClientBuilder) -> reqwest::blocking::ClientBuilder + Send + Sync>,
);

#[cfg(test)]
impl std::fmt::Debug for Detour {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Detour")
    }
}

impl Endpoint {
    /// One endpoint, calling through `transport`'s client.
    ///
    /// # Errors
    /// `E_CONFIG_INVALID` when no client can be built for this
    /// `base_url` and the machine's proxy settings.
    pub(crate) fn over(
        transport: &Transport,
        config: EndpointConfig,
        redemption: Redemption,
    ) -> Result<Endpoint, AxError> {
        Ok(Endpoint {
            client: transport.client(&ClientShape::of(&config))?,
            config,
            redemption,
        })
    }

    /// This endpoint as a model whose every call first takes a permit
    /// at `transport`'s gate, read against `monotonic`.
    pub(crate) fn gated(self, transport: &Transport, monotonic: fn() -> Instant) -> Gated {
        Gated::new(self, Arc::clone(&transport.gate), monotonic)
    }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    clippy::disallowed_methods,
    clippy::arithmetic_side_effects,
    clippy::print_stderr,
    reason = "test code: an instrument samples its own clock and prints its reading"
)]
mod tests {
    use std::time::{Duration, Instant};

    use super::*;
    use crate::endpoint::fakes::{config, request};
    use crate::endpoint::redemption::redemption;
    use crate::reach::resolve::KeptOpen;

    const HOST: &str = "api.anthropic.com";
    const BASE: &str = "https://api.anthropic.com/v1/messages";
    const MODELS: &str = "https://api.anthropic.com/v1/models";

    /// `crates/gateway/spec/Endpoint/Transport.lean` §8-35: every call to one endpoint goes out
    /// over the connection the endpoint's first call opened, so resolving,
    /// connecting and the handshake are paid once per endpoint rather than
    /// once per call. A call through another transport opens a connection
    /// of its own, which is what tells the stand-in's count apart from a
    /// count that could not see a second connection.
    #[test]
    fn a_second_call_to_one_endpoint_goes_out_over_the_first_calls_connection() {
        let stand_in = KeptOpen::listening(HOST);
        let shared = Transport::detoured(stand_in.toward());
        let other = Transport::detoured(stand_in.toward());
        for transport in [&shared, &shared, &other] {
            let mut endpoint = Endpoint::over(transport, config(BASE), redemption()).unwrap();
            let refused = endpoint.call(&request()).unwrap_err();
            assert_eq!(*refused.code(), AxCode::Provider, "{refused}");
        }
        let served = stand_in.served_on();
        assert_eq!(
            (served.len(), served[1] == served[0], served[2] == served[0]),
            (3, true, false),
            "connections that carried the three calls: {served:?}"
        );
    }

    /// `crates/gateway/spec/Endpoint/Transport.lean` D26: a warm-up opens the connection the
    /// endpoint's first call goes out over. The stand-in has heard one
    /// request once the warm-up returns, and the call that follows comes
    /// on the same connection.
    #[test]
    fn a_warm_up_opens_the_connection_the_first_call_goes_out_over() {
        let stand_in = KeptOpen::listening(HOST);
        let transport = Transport::detoured(stand_in.toward());
        let shape = ClientShape::of(&config(BASE));
        WarmUp::new(transport.clone(), shape, MODELS.to_owned()).open();
        let warmed = stand_in.served_on();
        let mut endpoint = Endpoint::over(&transport, config(BASE), redemption()).unwrap();
        let refused = endpoint.call(&request()).unwrap_err();
        assert_eq!(*refused.code(), AxCode::Provider, "{refused}");
        assert_eq!(
            (warmed, stand_in.served_on()),
            (vec![0], vec![0, 0]),
            "connections that carried the warm-up, then the warm-up and the call"
        );
    }

    /// The reading behind `crates/gateway/spec/Endpoint/Transport.lean` §8-35: an endpoint's
    /// first and second request through one client, timed apart, over
    /// `ROUNDS` fresh clients. The difference is what the first request
    /// pays to resolve, connect and shake hands. By default it reads the
    /// loopback TLS stand-in; `SPRAWLING_CONNECT_PROBE` names base URLs
    /// to read instead, comma-separated, each asked twice with a `GET`
    /// that carries no credential.
    #[test]
    #[ignore = "an instrument: run it by name with --run-ignored only --no-capture"]
    fn instrument_first_call_connect() {
        const ROUNDS: usize = 20;
        let stand_in = KeptOpen::listening(HOST);
        let named = std::env::var("SPRAWLING_CONNECT_PROBE").unwrap_or_default();
        let urls: Vec<String> = if named.is_empty() {
            vec![BASE.to_owned()]
        } else {
            named.split(',').map(str::to_owned).collect()
        };
        for url in urls {
            let detoured = named.is_empty();
            let mut first = Vec::new();
            let mut second = Vec::new();
            for _ in 0..ROUNDS {
                let builder = crate::client_for(kernel::Proxying::default(), &url);
                let builder = if detoured {
                    stand_in.toward()(builder)
                } else {
                    builder
                };
                let client = builder.build().unwrap();
                let asked = |client: &reqwest::blocking::Client| {
                    let t0 = Instant::now();
                    let answer = client.get(&url).send().unwrap();
                    drop(answer.text().unwrap());
                    t0.elapsed()
                };
                first.push(asked(&client));
                second.push(asked(&client));
            }
            let gaps: Vec<Duration> = first
                .iter()
                .zip(&second)
                .map(|(one, two)| one.saturating_sub(*two))
                .collect();
            let median = |samples: &[Duration]| {
                let mut sorted = samples.to_vec();
                sorted.sort();
                sorted[sorted.len() / 2].as_micros()
            };
            eprintln!(
                "connect url={url} rounds={ROUNDS} process_first_us={} first_p50_us={} second_p50_us={} gap_p50_us={}",
                first[0].as_micros(),
                median(&first),
                median(&second),
                median(&gaps)
            );
        }
    }
}
