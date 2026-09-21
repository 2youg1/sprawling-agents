// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The listening end, and the humble half of it (ARCHITECTURE section
//! 9). Every branch here is a send, a receive, or the end of a session;
//! the judgements it applies are `channels::reception`'s and the bytes
//! it serves are `channels::assets`'.
//!
//! Four jobs and no policy: serve the client bundle, upgrade a
//! WebSocket, take a credential from a caller on this machine, and let
//! an outside editor drive the city.
//!
//! A refusal made minutes later has no way home, which is why a command
//! carries the [`Reply`] address of whoever sent it.

//! Serving configuration: routes and bodies.

use std::net::SocketAddr;
use std::sync::Arc;

use axum::Router;
use axum::extract::{Request, State};
use axum::http::{StatusCode, header};
use axum::middleware::{Next, from_fn_with_state};
use axum::response::{IntoResponse, Response};
use axum::routing::{MethodRouter, get, post};
use kernel::{Address, AxError, B3Hash, EventRecord, Sealed};
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;

use crate::answer::Answer;
use crate::assets::ClientAssets;
use crate::command::{Command, WireCommand};
use crate::reception::{Admission, BindFace, Door, Pairing, decide_admission, offered_pairing};
use crate::wire::Query;

use super::reply::{Reply, refusal_text};

mod enrolment;

use super::socket::{accept_acp, accept_recording, serve_asset, serve_index, upgrade};
use enrolment::accept_enrolment;
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
/// Each sink is injected as a closure rather than as a trait:
/// `channels` declares no `pub trait` (it is not on the seam list,
/// ARCHITECTURE section 3), and one implementation is not a seam.
pub struct ServeConfig {
    pub addr: SocketAddr,
    /// Digest of the pairing token, never the token. `None` means no token
    /// is configured, which [`decide_bind`] turns into a refusal for any
    /// address reachable beyond this machine.
    pub token_digest: Option<B3Hash>,
    /// The client bundle, handed in by the assembly layer so this crate
    /// never learns where build artifacts live.
    pub client: Arc<ClientAssets>,
    /// Where a recording goes, and the line of text that comes back.
    ///
    /// A route of its own rather than a Command, for the reason
    /// enrolment has one: a Command is accepted and answered later
    /// through the event stream, and the text of what somebody just said
    /// has to come back to the tab that recorded it. A Query is the
    /// other shape that answers, and a query that spent seconds on a
    /// provider would be a command pretending to be a read.
    ///
    /// The media type travels beside the bytes because a container the
    /// city cannot name has no media type to send, and the judgement of
    /// which containers exist belongs to whoever speaks the audio wire.
    pub transcribe_sink: TranscribeSink,
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
    /// The process log, on its way to whoever has the log lens open.
    ///
    /// A third channel for the reason there is a second: what it
    /// carries is discardable, and a slow reader that lost a line has
    /// lost nothing. Sharing the event channel would let a city running
    /// at the `wire` floor push history out of that reader's window.
    pub logs: broadcast::Sender<crate::wire::LogLine>,
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
    pub(crate) commands: Arc<dyn Fn(WireCommand, Reply) -> Result<(), AxError> + Send + Sync>,
    pub(crate) events: broadcast::Sender<EventRecord>,
    pub(crate) deltas: broadcast::Sender<crate::wire::Delta>,
    pub(crate) logs: broadcast::Sender<crate::wire::LogLine>,
    pub(crate) queries: Arc<dyn Fn(Query) -> Result<Answer, AxError> + Send + Sync>,
    pub(crate) secrets: SecretSink,
    pub(crate) acp: AcpSink,
    pub(crate) transcribe_sink: TranscribeSink,
    /// Which face this listener presents, and the credential it demands.
    /// The value comes from [`decide_bind`] and is the only thing any
    /// door reads to judge a caller: a shell that held the configured
    /// digest beside a separate idea of the face could serve an exposed
    /// one with nothing to demand.
    ///
    /// [`decide_bind`]: crate::reception::decide_bind
    pub(crate) face: BindFace,
    pub(crate) city: Option<Address>,
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
/// **The body travels as the JSON it arrived as, and nothing here
/// reads a field out of it.** The grammar of an inbound request is
/// `protocol::Incoming`, whose `parse` is the only constructor and
/// therefore the only place the rules about it hold; a struct here
/// with the same four fields used to be deserialized first and copied
/// across field by field, which left `parse` with no caller outside
/// its own tests and carried the plaintext token into a value that
/// derives `Debug`. The token is read where it is judged, below, and
/// travels no further.
pub type AcpSink =
    Arc<dyn Fn(&serde_json::Value, Pairing) -> Result<AcpProgress, AxError> + Send + Sync>;

/// Where a recording goes: the bytes, the media type the browser
/// recorded into, and the line of text that comes back.
pub type TranscribeSink = Arc<dyn Fn(Vec<u8>, String) -> Result<String, AxError> + Send + Sync>;

/// The enrolment body: a realm, a name, and the value that will never be
/// seen again outside the vault.
///
/// The realm arrives as text because the caller chooses it: which word a
/// key is filed under is a building's or a form's decision, not a set
/// this server fixes, so nothing here enumerates the legal realms. What
/// the two segments must spell is `kernel::SecretRef`'s grammar, and the
/// route judges them with it before the credential moves.
#[derive(Debug, Deserialize)]
pub struct EnrollBody {
    pub realm: String,
    pub name: String,
    pub value: String,
}

/// Builds the route table. Split from `serve` so a test can exercise the
/// routes over an in-process transport without owning a port.
///
/// `face` is the binding verdict's, so the credential every door judges
/// against is decided once, before the socket exists, and cannot differ
/// from the face the listener presents.
pub fn router(config: &ServeConfig, face: BindFace) -> Router {
    let state = Arc::new(ShellState {
        client: Arc::clone(&config.client),
        commands: Arc::clone(&config.commands),
        events: config.events.clone(),
        deltas: config.deltas.clone(),
        logs: config.logs.clone(),
        queries: Arc::clone(&config.queries),
        secrets: Arc::clone(&config.secrets),
        acp: Arc::clone(&config.acp),
        transcribe_sink: Arc::clone(&config.transcribe_sink),
        face,
        city: config.city.clone(),
    });
    Router::new()
        // The client bundle is the page itself: a browser that has not
        // been given the pairing code yet still has to load the form it
        // types the code into, so these two doors stay open by design.
        .route("/", get(serve_index))
        .route("/ws", get(upgrade))
        .route(
            "/transcribe",
            paired(Door::Transcribe, &state, post(accept_recording)),
        )
        .route(
            "/enroll",
            paired(Door::Enroll, &state, post(accept_enrolment)),
        )
        // `/acp` judges the same token through the same function, from
        // inside the handler: an editor offers it as a body key rather
        // than as a header, and an unpaired editor is answered by
        // `protocol::admit` rather than at the door.
        .route("/acp", post(accept_acp))
        .route("/{*asset}", get(serve_asset))
        .with_state(state)
}

/// One door with the pairing judgement in front of it.
///
/// The door is baked into the layer rather than read back off the
/// request path, so the route table above is the only place a path is
/// spelled.
fn paired(
    door: Door,
    state: &Arc<ShellState>,
    handler: MethodRouter<Arc<ShellState>>,
) -> MethodRouter<Arc<ShellState>> {
    handler.route_layer(from_fn_with_state((door, Arc::clone(state)), admit_request))
}

/// The layer in front of every acting door: read the offered token,
/// hand it to the one judgement, and pass or refuse.
async fn admit_request(
    State((door, state)): State<(Door, Arc<ShellState>)>,
    request: Request,
    next: Next,
) -> Response {
    let offered = request
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok());
    match decide_admission(door, offered_pairing(offered), &state.face) {
        Admission::Admit(Pairing::Held | Pairing::Absent) => next.run(request).await,
        Admission::Refuse(err) => (StatusCode::FORBIDDEN, refusal_text(&err)).into_response(),
    }
}
