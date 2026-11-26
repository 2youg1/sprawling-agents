// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::error::Error;
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use axum::Router;
use axum::body::Body;
use axum::http::{Method, Request, Response, StatusCode, header};
use bytes::Bytes;
use city::RemoteRoute;
use futures_util::StreamExt;
use h3::ext::Protocol;
use h3_webtransport::server::{AcceptedBi, WebTransportSession};
use kernel::{AxCode, AxError};
use quinn::crypto::rustls::QuicServerConfig;
use reqwest::Url;
use rustls::pki_types::{CertificateDer, PrivateKeyDer, pem::PemObject};
use tower::ServiceExt;

use super::console::Lasting;
use super::keeper::{Doorway, Phase};
use super::relay::{self, HANDSHAKE_PATIENCE, Run};
use super::stream::Device;

type Failure = Box<dyn Error + Send + Sync>;
type Answer = h3::server::RequestStream<h3_quinn::BidiStream<Bytes>, Bytes>;

const MAX_CONNECTIONS: usize = 64;
const PAGE_CHUNK_BYTES: usize = 16 * 1024;
const TICK: Duration = Duration::from_secs(1);

#[derive(Clone)]
pub(crate) struct Reaching {
    pub(crate) runtime: tokio::runtime::Handle,
    pub(crate) city_root: PathBuf,
    pub(crate) city: SocketAddr,
    pub(crate) token: String,
    pub(crate) page: Arc<wire::ClientAssets>,
}

struct Bound {
    endpoint: quinn::Endpoint,
    address: SocketAddr,
    serving: Serving,
}

struct Listening {
    _registered: crate::browser_tool::Served,
    endpoint: quinn::Endpoint,
    task: tokio::task::AbortHandle,
}

impl Drop for Listening {
    fn drop(&mut self) {
        self.endpoint.close(0_u32.into(), b"remote door closed");
        self.task.abort();
    }
}

pub(super) fn open(
    doorway: &Doorway,
    reaching: &Reaching,
    lasting: Lasting,
) -> Result<Url, AxError> {
    doorway.open(
        lasting,
        || prepare(doorway, reaching),
        |Bound {
             endpoint,
             address,
             serving,
         }| {
            let registered = crate::browser_tool::serve_public(address, &serving.origin);
            let task = reaching.runtime.spawn(serve(endpoint.clone(), serving));
            Box::new(Listening {
                _registered: registered,
                endpoint,
                task: task.abort_handle(),
            })
        },
    )
}

fn prepare(doorway: &Doorway, reaching: &Reaching) -> Result<(Url, Bound), AxError> {
    let RemoteRoute::H3 {
        url,
        listen,
        certificate,
        private_key,
    } = city::remote_route(&reaching.city_root)?;
    let file = kernel::layout::CityLayout::new(&reaching.city_root).city_config();
    let origin = Url::parse(&url).map_err(|source| invalid(&file, "url", &url, &source))?;
    if origin.scheme() != "https"
        || origin.host_str().is_none()
        || !origin.username().is_empty()
        || origin.password().is_some()
        || origin.path() != "/"
        || origin.query().is_some()
        || origin.fragment().is_some()
    {
        return Err(invalid(
            &file,
            "url",
            &url,
            &"use an https:// origin without a path, login, query or fragment",
        ));
    }
    let tls = tls(&file, &certificate, &private_key)?;
    let page = wire::bundle_routes(
        Arc::clone(&reaching.page),
        &wire::PageHeaders::every_listener(),
    )?;
    let endpoint = {
        let _inside = reaching.runtime.enter();
        quinn::Endpoint::server(tls, listen)
            .map_err(|source| invalid(&file, "listen", &listen, &source))?
    };
    let address = endpoint
        .local_addr()
        .map_err(|source| invalid(&file, "listen", &listen, &source))?;
    Ok((
        origin.clone(),
        Bound {
            endpoint,
            address,
            serving: Serving {
                doorway: doorway.clone(),
                reaching: reaching.clone(),
                serialized_origin: origin.origin().ascii_serialization(),
                origin,
                page,
            },
        },
    ))
}

fn tls(file: &Path, certificate: &str, private_key: &str) -> Result<quinn::ServerConfig, AxError> {
    for (key, path) in [("certificate", certificate), ("private_key", private_key)] {
        if !Path::new(path).is_absolute() {
            return Err(invalid(file, key, &path, &"use an absolute PEM file path"));
        }
    }
    let chain = CertificateDer::pem_file_iter(certificate)
        .and_then(Iterator::collect)
        .map_err(|source| invalid(file, "certificate", &certificate, &source))?;
    let key = PrivateKeyDer::from_pem_file(private_key)
        .map_err(|source| invalid(file, "private_key", &private_key, &source))?;
    let mut tls = rustls::ServerConfig::builder_with_provider(Arc::new(
        rustls::crypto::aws_lc_rs::default_provider(),
    ))
    .with_protocol_versions(&[&rustls::version::TLS13])
    .map_err(|source| invalid(file, "certificate", &certificate, &source))?
    .with_no_client_auth()
    .with_single_cert(chain, key)
    .map_err(|source| invalid(file, "certificate and private_key", &certificate, &source))?;
    tls.alpn_protocols = vec![b"h3".to_vec()];
    let crypto = QuicServerConfig::try_from(tls)
        .map_err(|source| invalid(file, "certificate", &certificate, &source))?;
    Ok(quinn::ServerConfig::with_crypto(Arc::new(crypto)))
}

fn invalid(
    file: &Path,
    key: &str,
    value: &dyn std::fmt::Display,
    cause: &dyn std::fmt::Display,
) -> AxError {
    AxError::failure(
        AxCode::ConfigInvalid,
        "open the remote door",
        format!("{}: [remote] {key} = {value}: {cause}", file.display()),
    )
    .with_recovery("correct the indicated setting in that file and retry /remote open; see docs/operate/operating.md#choosing-the-route")
}

#[derive(Clone)]
struct Serving {
    doorway: Doorway,
    reaching: Reaching,
    origin: Url,
    serialized_origin: String,
    page: Router,
}

async fn serve(endpoint: quinn::Endpoint, serving: Serving) {
    let mut connections = tokio::task::JoinSet::new();
    let expiry = keep_time(serving.doorway.clone());
    tokio::pin!(expiry);
    loop {
        tokio::select! {
            () = &mut expiry => return,
            incoming = endpoint.accept() => {
                let Some(incoming) = incoming else { return };
                if connections.len() >= MAX_CONNECTIONS {
                    incoming.refuse();
                } else {
                    let serving = serving.clone();
                    connections.spawn(async move {
                        if let Err(_disconnected) = answer(incoming, serving).await {}
                    });
                }
            }
            joined = connections.join_next(), if !connections.is_empty() => {
                if let Some(Err(failure)) = joined {
                    eprintln!("  a remote connection task ended: {failure}");
                }
            }
        }
    }
}

async fn answer(incoming: quinn::Incoming, serving: Serving) -> Result<(), Failure> {
    let connection = tokio::time::timeout(HANDSHAKE_PATIENCE, incoming).await??;
    let mut builder = h3::server::builder();
    builder
        .enable_webtransport(true)
        .enable_extended_connect(true)
        .enable_datagram(true)
        .max_webtransport_sessions(1);
    let mut h3 = tokio::time::timeout(
        HANDSHAKE_PATIENCE,
        builder.build(h3_quinn::Connection::new(connection.clone())),
    )
    .await??;
    while let Some(resolver) = tokio::time::timeout(HANDSHAKE_PATIENCE, h3.accept()).await?? {
        let (request, response) =
            tokio::time::timeout(HANDSHAKE_PATIENCE, resolver.resolve_request()).await??;
        let status = if !serving.addressed(&request) {
            StatusCode::MISDIRECTED_REQUEST
        } else if let Some(run) = run(&request) {
            if serving.same_origin(&request) {
                let session = tokio::time::timeout(
                    HANDSHAKE_PATIENCE,
                    WebTransportSession::accept(request, response, h3),
                )
                .await??;
                carry(&session, &serving, run).await?;
                if let Err(_expired) =
                    tokio::time::timeout(HANDSHAKE_PATIENCE, connection.closed()).await
                {
                }
                return Ok(());
            }
            StatusCode::FORBIDDEN
        } else if matches!(*request.method(), Method::GET | Method::HEAD) {
            let answer = serving
                .page
                .clone()
                .oneshot(request.map(|()| Body::empty()))
                .await?;
            page(response, answer).await?;
            continue;
        } else {
            StatusCode::METHOD_NOT_ALLOWED
        };
        let refused = Response::builder()
            .status(status)
            .header(header::CACHE_CONTROL, "no-store")
            .body(Body::empty())?;
        page(response, refused).await?;
    }
    Ok(())
}

impl Serving {
    fn addressed(&self, request: &Request<()>) -> bool {
        let uri = request.uri();
        let Some(expected) = self.serialized_origin.strip_prefix("https://") else {
            return false;
        };
        uri.scheme_str() == Some("https")
            && uri.authority().is_some_and(|value| {
                let authority = value.as_str();
                authority.eq_ignore_ascii_case(expected)
                    || (self.origin.port().is_none()
                        && authority
                            .strip_suffix(":443")
                            .is_some_and(|host| host.eq_ignore_ascii_case(expected)))
            })
    }

    fn same_origin(&self, request: &Request<()>) -> bool {
        let mut origins = request.headers().get_all(header::ORIGIN).iter();
        origins
            .next()
            .is_some_and(|value| *value == self.serialized_origin)
            && origins.next().is_none()
    }
}

fn run(request: &Request<()>) -> Option<Run> {
    if request.method() != Method::CONNECT
        || request.extensions().get::<Protocol>() != Some(&Protocol::WEB_TRANSPORT)
    {
        return None;
    }
    match request.uri().path_and_query()?.as_str() {
        "/remote/pair" => Some(Run::Pair),
        "/remote/session" => Some(Run::Session),
        _ => None,
    }
}

async fn carry(
    session: &WebTransportSession<h3_quinn::Connection, Bytes>,
    serving: &Serving,
    run: Run,
) -> Result<(), Failure> {
    if let Some(AcceptedBi::BidiStream(id, stream)) =
        tokio::time::timeout(HANDSHAKE_PATIENCE, session.accept_bi()).await??
        && id == session.session_id()
    {
        relay::carry(
            Device::new(stream),
            &serving.doorway,
            &serving.reaching,
            run,
        )
        .await;
    }
    Ok(())
}

async fn page(mut response: Answer, answer: Response<Body>) -> Result<(), Failure> {
    let (head, body) = answer.into_parts();
    tokio::time::timeout(
        HANDSHAKE_PATIENCE,
        response.send_response(Response::from_parts(head, ())),
    )
    .await??;
    let mut chunks = body.into_data_stream();
    while let Some(chunk) = chunks.next().await {
        let mut chunk = chunk?;
        while !chunk.is_empty() {
            let piece = chunk.split_to(chunk.len().min(PAGE_CHUNK_BYTES));
            tokio::time::timeout(HANDSHAKE_PATIENCE, response.send_data(piece)).await??;
        }
    }
    tokio::time::timeout(HANDSHAKE_PATIENCE, response.finish()).await??;
    Ok(())
}

async fn keep_time(doorway: Doorway) {
    let now = tokio::time::Instant::now();
    let mut ticks = tokio::time::interval_at(now.checked_add(TICK).unwrap_or(now), TICK);
    loop {
        ticks.tick().await;
        let door = doorway.clone();
        match tokio::task::spawn_blocking(move || door.tick()).await {
            Ok(Ok(Phase::Open)) => {}
            Ok(Ok(Phase::Closed)) | Err(_) => return,
            Ok(Err(refused)) => {
                eprintln!("  the remote door could not close at its time: {refused}");
                eprintln!("  {}", refused.recovery());
                return;
            }
        }
    }
}
