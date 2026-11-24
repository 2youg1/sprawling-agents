// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use axum::Router;
use axum::extract::{DefaultBodyLimit, Request, State};
use axum::http::{StatusCode, header};
use axum::middleware::{Next, from_fn_with_state};
use axum::response::{IntoResponse, Response};
use axum::routing::{MethodRouter, get, post};
use kernel::{Address, AxError, B3Hash, Sealed, Seq};
use serde::Deserialize;
use tokio::sync::broadcast;

use crate::answer::Answer;
use crate::assets::ClientAssets;
use crate::auth::Pairing;
use crate::command::{Command, WireCommand};
use crate::frames::Query;
use crate::reception::{
    Admission, Arrival, BindFace, Door, ListenerOrigins, PageHeaders, decide_admission,
    offered_pairing,
};
use crate::reply::{AcpProgress, Reply};

use super::committed::Committed;

mod enrolment;

use super::bundle::bundle_routes;
use super::door::LocalDoor;
use super::guard::{Stamp, Stored, enter, preflight, stamp};
use super::pairing::{accept_pairing, offer_challenge, open_session};
use super::socket::upgrade;
use super::uploads::{accept_acp, accept_drop, accept_recording};
use super::workspace::{self, SAVE_BYTES_MAX, WorkspaceSink};
use enrolment::accept_enrolment;

pub type Answering = Arc<dyn Fn(Query) -> (Seq, Result<Answer, AxError>) + Send + Sync>;

pub type SecretSink =
    Arc<dyn Fn(Command<Sealed<String>>, Reply) -> Result<(), AxError> + Send + Sync>;

const ENROLMENT_PATIENCE: std::time::Duration = std::time::Duration::from_secs(2);

pub struct ServeConfig {
    pub client: Arc<ClientAssets>,

    pub transcribe_sink: TranscribeSink,

    pub drop_sink: DropSink,

    pub workspace: Option<WorkspaceSink>,

    pub acp: AcpSink,

    pub commands: Arc<dyn Fn(WireCommand, Reply) -> Result<(), AxError> + Send + Sync>,

    pub events: broadcast::Sender<Committed>,

    pub deltas: broadcast::Sender<crate::frames::Delta>,

    pub logs: broadcast::Sender<crate::frames::LogLine>,

    pub outputs: broadcast::Sender<crate::frames::LiveOutput>,

    pub outputs_so_far: Arc<dyn Fn() -> Vec<crate::frames::LiveOutput> + Send + Sync>,

    pub monitor: MonitorFeed,

    pub queries: Answering,

    pub secrets: SecretSink,

    pub city: Option<Address>,

    pub head: Arc<LedgerHead>,

    pub epoch: Option<B3Hash>,

    pub door: LocalDoor,
}

#[derive(Debug, Default)]
pub struct LedgerHead(AtomicU64);

impl LedgerHead {
    #[must_use]
    pub fn at(head_seq: Option<Seq>) -> Self {
        let head = Self::default();
        if let Some(seq) = head_seq {
            head.advance(seq);
        }
        head
    }

    pub fn advance(&self, seq: Seq) {
        if let Some(stored) = seq.value().checked_add(1) {
            self.0.store(stored, Ordering::Release);
        }
    }

    #[must_use]
    pub fn read(&self) -> Option<Seq> {
        self.0.load(Ordering::Acquire).checked_sub(1).map(Seq::new)
    }
}

pub(crate) struct ShellState {
    pub(crate) commands: Arc<dyn Fn(WireCommand, Reply) -> Result<(), AxError> + Send + Sync>,
    pub(crate) events: broadcast::Sender<Committed>,
    pub(crate) deltas: broadcast::Sender<crate::frames::Delta>,
    pub(crate) logs: broadcast::Sender<crate::frames::LogLine>,
    pub(crate) outputs: broadcast::Sender<crate::frames::LiveOutput>,
    pub(crate) outputs_so_far: Arc<dyn Fn() -> Vec<crate::frames::LiveOutput> + Send + Sync>,
    pub(crate) monitor: MonitorFeed,
    pub(crate) queries: Answering,
    pub(crate) secrets: SecretSink,
    pub(crate) acp: AcpSink,
    pub(crate) transcribe_sink: TranscribeSink,
    pub(crate) drop_sink: DropSink,
    pub(crate) workspace: Option<WorkspaceSink>,

    pub(crate) face: BindFace,
    pub(crate) door: LocalDoor,
    pub(crate) city: Option<Address>,
    pub(crate) head: Arc<LedgerHead>,
    pub(crate) epoch: Option<B3Hash>,
}

#[derive(Clone)]
pub struct MonitorFeed {
    pub watch: Arc<dyn Fn(crate::frames::Watched) -> Box<dyn Send> + Send + Sync>,

    pub samples: broadcast::Sender<crate::frames::Sample>,

    pub beat: Arc<dyn Fn(crate::frames::BeatMs) + Send + Sync>,
}

pub type AcpSink =
    Arc<dyn Fn(&serde_json::Value, Pairing) -> Result<AcpProgress, AxError> + Send + Sync>;

pub type TranscribeSink = Arc<dyn Fn(Vec<u8>, String) -> Result<String, AxError> + Send + Sync>;

pub type DropSink = Arc<dyn Fn(&str, &[u8]) -> Result<String, AxError> + Send + Sync>;

pub const DROP_BYTES_MAX: usize = 64 * 1024 * 1024;

#[derive(Debug, Deserialize)]
pub struct EnrollBody {
    pub realm: String,
    pub name: String,
    pub value: String,
}

pub fn router(
    config: &ServeConfig,
    face: BindFace,
    origins: &ListenerOrigins,
) -> Result<Router, AxError> {
    let state = Arc::new(ShellState {
        commands: Arc::clone(&config.commands),
        events: config.events.clone(),
        deltas: config.deltas.clone(),
        logs: config.logs.clone(),
        outputs: config.outputs.clone(),
        outputs_so_far: Arc::clone(&config.outputs_so_far),
        monitor: config.monitor.clone(),
        queries: Arc::clone(&config.queries),
        secrets: Arc::clone(&config.secrets),
        acp: Arc::clone(&config.acp),
        transcribe_sink: Arc::clone(&config.transcribe_sink),
        drop_sink: Arc::clone(&config.drop_sink),
        workspace: config.workspace.clone(),
        face,
        door: config.door.clone(),
        city: config.city.clone(),
        head: Arc::clone(&config.head),
        epoch: config.epoch,
    });
    let names = Arc::new(origins.clone());
    let headers = PageHeaders::every_listener();
    let entered = |arrival: Arrival, handler: MethodRouter<Arc<ShellState>>| {
        handler.route_layer(from_fn_with_state((arrival, Arc::clone(&names)), enter))
    };
    let door = |door: Door, handler| entered(Arrival::Door(door), paired(door, &state, handler));
    let answers = Router::new()
        .route("/ws", entered(Arrival::Socket, get(upgrade)))
        .route(
            "/transcribe",
            door(Door::Transcribe, post(accept_recording)),
        )
        .route("/enroll", door(Door::Enroll, post(accept_enrolment)))
        .route(
            "/drop",
            door(
                Door::Drop,
                post(accept_drop).layer(DefaultBodyLimit::max(DROP_BYTES_MAX)),
            ),
        )
        .route(
            "/workspace",
            door(Door::Workspace, post(workspace::request)),
        )
        .route(
            "/workspace/save",
            door(
                Door::Workspace,
                post(workspace::save).layer(DefaultBodyLimit::max(SAVE_BYTES_MAX)),
            ),
        )
        .route("/acp", entered(Arrival::Door(Door::Acp), post(accept_acp)))
        .route("/pair", entered(Arrival::Pairing, post(accept_pairing)))
        .route(
            "/session/challenge",
            entered(Arrival::Pairing, post(offer_challenge)),
        )
        .route("/session", entered(Arrival::Pairing, post(open_session)))
        .layer(from_fn_with_state(
            Arc::new(Stamp::of(&headers, Stored::Never)?),
            stamp,
        ))
        .with_state(state);

    let page = bundle_routes(Arc::clone(&config.client), &headers)?.route_layer(
        from_fn_with_state((Arrival::Page, Arc::clone(&names)), enter),
    );
    Ok(answers
        .merge(page)
        .layer(from_fn_with_state(names, preflight)))
}

pub(crate) fn refusal_text(err: &AxError) -> String {
    format!("{}: {}", err.action(), err.recovery())
}

fn paired(
    door: Door,
    state: &Arc<ShellState>,
    handler: MethodRouter<Arc<ShellState>>,
) -> MethodRouter<Arc<ShellState>> {
    handler.route_layer(from_fn_with_state((door, Arc::clone(state)), admit_request))
}

async fn admit_request(
    State((door, state)): State<(Door, Arc<ShellState>)>,
    request: Request,
    next: Next,
) -> Response {
    if door == Door::Workspace && !matches!(state.face, BindFace::Loopback { .. }) {
        return workspace::refusal(
            AxError::failure(
                kernel::AxCode::GateDenied,
                "open local workspace files",
                "the listener is not bound to loopback",
            )
            .with_recovery("open RefRain on a loopback listener on the city host"),
        );
    }
    let offered = request
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok());
    let pairing = match state.door.pairing(&state.face, offered_pairing(offered)) {
        Ok(pairing) => pairing,
        Err(err) => return (StatusCode::INTERNAL_SERVER_ERROR, refusal_text(&err)).into_response(),
    };
    match decide_admission(door, pairing) {
        Admission::Admit(Pairing::Held | Pairing::Absent) => next.run(request).await,
        Admission::Refuse(err) if door == Door::Workspace => workspace::refusal(err),
        Admission::Refuse(err) => (StatusCode::FORBIDDEN, refusal_text(&err)).into_response(),
    }
}
