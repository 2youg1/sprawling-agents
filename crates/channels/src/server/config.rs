// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The listening end, and the humble half of it (ARCHITECTURE section
//! 9). Every branch here is a send, a receive, or the end of a session;
//! the judgements it applies are `channels::reception`'s and the bytes
//! it serves are `channels::assets`'.
//!
//! Five jobs and no policy: serve the client bundle, upgrade a
//! WebSocket, accept an upload, take a credential from a caller on this
//! machine, and let an outside editor drive the city.
//!
//! A refusal made minutes later has no way home, which is why a command
//! carries the [`Reply`] address of whoever sent it.

//! Serving configuration: routes and bodies.

use std::net::SocketAddr;
use std::sync::Arc;

use axum::Router;
use axum::body::Bytes;
use axum::extract::{ConnectInfo, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use kernel::{Address, AxError, B3Hash, EventKind, EventRecord, Sealed};
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;

use crate::answer::Answer;
use crate::assets::ClientAssets;
use crate::carried_name::UploadId;
use crate::command::{Command, WireCommand};
use crate::reception::{EnrollVerdict, decide_enroll};
use crate::wire::Query;

use super::reply::{Delivered, Reply, refusal_text};
use super::socket::{accept_acp, accept_upload, serve_asset, serve_index, upgrade};
pub type SecretSink =
    Arc<dyn Fn(Command<Sealed<String>>, Reply) -> Result<(), AxError> + Send + Sync>;

/// How long the enrolment route waits for the worker to say what
/// happened.
///
/// Bounded because the worker may be inside a dispatch that runs for
/// minutes, and an HTTP request that waited for it would look like a
/// hang. Waiting longer would not help in that case and hurts in every
/// other: what is being waited for is a queue hop and a vault write,
/// both of which are milliseconds.
const ENROLMENT_PATIENCE: std::time::Duration = std::time::Duration::from_secs(2);

/// Everything the shell needs that it must not decide for itself.
///
/// `upload_sink` is injected as a closure rather than a trait: `channels`
/// declares no `pub trait` (it is not on the seam list, ARCHITECTURE
/// section 3), and one implementation is not a seam.
pub struct ServeConfig {
    pub addr: SocketAddr,
    /// Digest of the pairing token, never the token. `None` means no token
    /// is configured, which [`decide_bind`] turns into a refusal for any
    /// address reachable beyond this machine.
    pub token_digest: Option<B3Hash>,
    /// The client bundle, handed in by the assembly layer so this crate
    /// never learns where build artifacts live.
    pub client: Arc<ClientAssets>,
    /// Where `Attach` bytes go. Returns the handle a later Command names.
    ///
    /// Takes `Vec<u8>` rather than the transport's own buffer type: the
    /// assembly layer must not have to name axum to hand us a sink, and a
    /// public signature that leaks the HTTP library would make replacing it
    /// a breaking change for every caller.
    pub upload_sink: Arc<dyn Fn(Vec<u8>) -> Result<UploadId, AxError> + Send + Sync>,
    /// Where a request from an outside editor goes. Separate from
    /// `commands` because it arrives over its own route, carries its own
    /// authentication, and gets an answer rather than an event stream.
    pub acp: AcpSink,
    /// Where an accepted Command goes. **Accepting is not running it**: a
    /// dispatch may take hours, and awaiting it inside the socket task
    /// would tie the work to the lifetime of one browser tab. The sink
    /// takes the command, returns, and the progress comes back as events.
    ///
    /// The [`Reply`] travels with the command because the answer arrives
    /// long after this call returned, and a refusal belongs to the peer
    /// that caused it.
    pub commands: Arc<dyn Fn(WireCommand, Reply) -> Result<(), AxError> + Send + Sync>,
    /// The event fan-out. This crate only subscribes; the writer is
    /// whoever owns the Ledger, because the ledger line is the event.
    pub events: broadcast::Sender<EventRecord>,
    /// What a model is saying, while it is still saying it.
    ///
    /// A second channel rather than a second variant on the first,
    /// because the two carry different kinds of thing and lag
    /// differently: an increment a slow client missed is nothing, and an
    /// event it missed is history it must recover. Sharing one channel
    /// would let a burst of increments push records out of a reader's
    /// window.
    pub deltas: broadcast::Sender<crate::wire::Delta>,
    /// Answers a query from the city's derived views. Synchronous: a
    /// query reads a projection, and a projection that needed to block
    /// would be a query pretending to be a command.
    pub queries: Arc<dyn Fn(Query) -> Result<Answer, AxError> + Send + Sync>,
    /// Where an enrolled credential goes. Takes the full [`Command`] and
    /// not [`WireCommand`]: this is the one sink whose input has no byte
    /// form, which is what keeps enrolment a local act.
    pub secrets: SecretSink,
    /// Which city this server is. Told at the handshake, because a client
    /// that only hears what happens next cannot know the name of a city
    /// that was initialised last month.
    pub city: Option<Address>,
}

pub(crate) struct ShellState {
    pub(crate) client: Arc<ClientAssets>,
    pub(crate) upload_sink: Arc<dyn Fn(Vec<u8>) -> Result<UploadId, AxError> + Send + Sync>,
    pub(crate) commands: Arc<dyn Fn(WireCommand, Reply) -> Result<(), AxError> + Send + Sync>,
    pub(crate) events: broadcast::Sender<EventRecord>,
    pub(crate) deltas: broadcast::Sender<crate::wire::Delta>,
    pub(crate) queries: Arc<dyn Fn(Query) -> Result<Answer, AxError> + Send + Sync>,
    pub(crate) secrets: SecretSink,
    pub(crate) acp: AcpSink,
    pub(crate) token_digest: Option<B3Hash>,
    pub(crate) city: Option<Address>,
}

/// One request from an outside editor driving this city as an agent.
///
/// The shape is the protocol's, not this crate's; what this crate adds
/// is that the token is compared here, where the pairing token already
/// lives, and only the verdict travels inward.
#[derive(Debug, Deserialize)]
pub struct AcpBody {
    pub token: String,
    pub addr: String,
    pub task: String,
    pub goal: String,
}

/// What an accepted request gets back: the run it became, and nothing
/// else. Progress is what an editor may see; the city's history is not
/// published through this door.
#[derive(Debug, Serialize)]
pub struct AcpProgress {
    pub run: String,
    pub turns: u32,
    pub finished: bool,
}

/// Where an outside request goes once the token has been judged.
///
/// The boolean is carried rather than acted on here: the refusal for an
/// unauthenticated request is `protocol::admit`'s to word, and it words
/// it so that a stranger learns exactly one bit.
pub type AcpSink = Arc<dyn Fn(AcpBody, bool) -> Result<AcpProgress, AxError> + Send + Sync>;

/// The enrolment body: a realm, a name, and the value that will never be
/// seen again outside the vault.
#[derive(Debug, Deserialize)]
pub struct EnrollBody {
    pub realm: String,
    pub name: String,
    pub value: String,
}

/// Builds the route table. Split from `serve` so a test can exercise the
/// routes over an in-process transport without owning a port.
pub fn router(config: &ServeConfig) -> Router {
    let state = Arc::new(ShellState {
        client: Arc::clone(&config.client),
        upload_sink: Arc::clone(&config.upload_sink),
        commands: Arc::clone(&config.commands),
        events: config.events.clone(),
        deltas: config.deltas.clone(),
        queries: Arc::clone(&config.queries),
        secrets: Arc::clone(&config.secrets),
        acp: Arc::clone(&config.acp),
        token_digest: config.token_digest,
        city: config.city.clone(),
    });
    Router::new()
        .route("/", get(serve_index))
        .route("/ws", get(upgrade))
        .route("/upload", post(accept_upload))
        .route("/enroll", post(accept_enrolment))
        .route("/acp", post(accept_acp))
        .route("/{*asset}", get(serve_asset))
        .with_state(state)
}

/// The one route that carries a credential. It exists as HTTP rather
/// than as a socket frame because the socket's Command type cannot spell
/// a secret; here the same rule is enforced against the peer address.
async fn accept_enrolment(
    State(state): State<Arc<ShellState>>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    body: Bytes,
) -> Response {
    if let EnrollVerdict::Refuse(err) = decide_enroll(&peer) {
        return (StatusCode::FORBIDDEN, refusal_text(&err)).into_response();
    }
    let Ok(enrolment) = serde_json::from_slice::<EnrollBody>(&body) else {
        return (
            StatusCode::UNPROCESSABLE_ENTITY,
            "send {\"realm\":..,\"name\":..,\"value\":..}",
        )
            .into_response();
    };
    let EnrollBody { realm, name, value } = enrolment;
    let reference = format!("secret:{realm}/{name}");
    let command = Command::PutSecret {
        realm,
        name,
        value: Sealed::new(Box::new(value)),
    };
    // Subscribed before the command is posted: a worker that finished
    // while this task was still setting up would otherwise write the one
    // record this request is waiting for into a stream nobody is reading.
    let mut records = state.events.subscribe();
    let (refused, mut refusals) = tokio::sync::mpsc::unbounded_channel::<AxError>();
    let reply = Reply::to(move |err: AxError| match refused.send(err) {
        Ok(()) => Delivered::ToThePeer,
        Err(_) => Delivered::PeerGone,
    });
    if let Err(err) = (state.secrets)(command, reply) {
        return (StatusCode::UNPROCESSABLE_ENTITY, refusal_text(&err)).into_response();
    }
    let wanted = reference.clone();
    let waited = tokio::time::timeout(ENROLMENT_PATIENCE, async move {
        // The reply address is dropped when the worker finishes without
        // refusing, so a closed refusal channel says only that no
        // refusal is coming - never that the credential was stored. The
        // guard stops the closed branch from spinning; the event is
        // still what a 201 waits for.
        let mut refusal_possible = true;
        loop {
            tokio::select! {
                refusal = refusals.recv(), if refusal_possible => match refusal {
                    Some(err) => return Some(Enrolled::Refused(err)),
                    None => refusal_possible = false,
                },
                record = records.recv() => match record {
                    Ok(record) => {
                        if record.kind() == EventKind::SecretCaptured
                            && record
                                .data()
                                .as_map()
                                .get("ref")
                                .and_then(serde_json::Value::as_str)
                                == Some(wanted.as_str())
                        {
                            return Some(Enrolled::Stored);
                        }
                    }
                    // Lagged means this task missed records, not that the
                    // enrolment failed; the loop keeps waiting and the
                    // timeout below is what ends it.
                    Err(broadcast::error::RecvError::Lagged(_)) => {}
                    Err(broadcast::error::RecvError::Closed) => return None,
                },
            }
        }
    })
    .await;
    match waited {
        Ok(Some(Enrolled::Stored)) => (StatusCode::CREATED, reference).into_response(),
        Ok(Some(Enrolled::Refused(err))) => {
            (StatusCode::UNPROCESSABLE_ENTITY, refusal_text(&err)).into_response()
        }
        // Neither arrived. Two-oh-two says so in the one word HTTP has
        // for it, and the body says why rather than leaving the caller
        // to read a status code as an outcome.
        Ok(None) | Err(_) => (
            StatusCode::ACCEPTED,
            format!(
                "{reference} was handed to the city and it has not answered within {}s; the                  worker may be inside a dispatch. Check whether the reference resolves before                  sending the credential again",
                ENROLMENT_PATIENCE.as_secs()
            ),
        )
            .into_response(),
    }
}

/// What the city said about one enrolment. Two arms and a timeout, which
/// is three answers, and the route gives each of them its own status.
enum Enrolled {
    Stored,
    Refused(AxError),
}
