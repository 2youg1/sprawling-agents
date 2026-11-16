// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::net::SocketAddr;
use std::sync::Arc;

use axum::body::Bytes;
use axum::extract::{ConnectInfo, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use kernel::event::record::SecretCaptured;
use kernel::{AxError, EventKind, EventRecord, Sealed, SecretRef};
use tokio::sync::broadcast;

use crate::command::Command;
use crate::reception::{EnrollVerdict, decide_enroll};

use super::refusal_text;
use super::{ENROLMENT_PATIENCE, EnrollBody, ShellState};
use crate::reply::{Delivered, Reply};

pub(super) async fn accept_enrolment(
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

    let place = match SecretRef::new(&realm, &name) {
        Ok(place) => place,
        Err(err) => {
            return (StatusCode::UNPROCESSABLE_ENTITY, refusal_text(&err)).into_response();
        }
    };

    let command = Command::PutSecret {
        realm: place.realm().to_owned(),
        name: place.name().to_owned(),
        value: Sealed::new(Box::new(value)),
    };

    let asked = place.to_string();

    let mut records = state.events.subscribe();
    let (refused, mut refusals) = tokio::sync::mpsc::unbounded_channel::<AxError>();
    let reply = Reply::to(move |err: AxError| match refused.send(err) {
        Ok(()) => Delivered::ToThePeer,
        Err(_) => Delivered::PeerGone,
    });
    if let Err(err) = (state.secrets)(command, reply) {
        return (StatusCode::UNPROCESSABLE_ENTITY, refusal_text(&err)).into_response();
    }
    let waited = tokio::time::timeout(ENROLMENT_PATIENCE, async move {

        let mut refusal_possible = true;
        loop {
            tokio::select! {
                refusal = refusals.recv(), if refusal_possible => match refusal {
                    Some(err) => return Waited::Settled(Enrolled::Refused(err)),
                    None => refusal_possible = false,
                },
                record = records.recv() => match record {
                    Ok(record) => {
                        if let Some(stored) = stored_reference(record.record(), &place) {
                            return Waited::Settled(Enrolled::Stored { reference: stored });
                        }
                    }

                    Err(broadcast::error::RecvError::Lagged(_)) => return Waited::Overtaken,
                    Err(broadcast::error::RecvError::Closed) => return Waited::Ended,
                },
            }
        }
    })
    .await;
    match waited {
        Ok(Waited::Settled(Enrolled::Stored { reference })) => {
            (StatusCode::CREATED, reference).into_response()
        }
        Ok(Waited::Settled(Enrolled::Refused(err))) => {
            (StatusCode::UNPROCESSABLE_ENTITY, refusal_text(&err)).into_response()
        }

        Err(_) => (
            StatusCode::ACCEPTED,
            format!(
                "{asked} was handed to the city and it has not answered within {}s; the \
                 worker may be inside a dispatch. Check whether the reference resolves before \
                 sending the credential again",
                ENROLMENT_PATIENCE.as_secs()
            ),
        )
            .into_response(),

        Ok(Waited::Overtaken) => (
            StatusCode::ACCEPTED,
            format!(
                "{asked} was handed to the city, and this server's event stream moved past \
                 the request before the vault said what became of it. Check whether the \
                 reference resolves before sending the credential again"
            ),
        )
            .into_response(),

        Ok(Waited::Ended) => (
            StatusCode::ACCEPTED,
            format!(
                "{asked} was handed to the city, and the city's event stream ended before \
                 the vault said what became of it. Check whether the reference resolves before \
                 sending the credential again"
            ),
        )
            .into_response(),
    }
}

fn stored_reference(record: &EventRecord, place: &SecretRef) -> Option<String> {
    if record.kind() != EventKind::SecretCaptured {
        return None;
    }

    let captured: SecretCaptured = record.data().read().ok()?;
    (captured.reference == *place).then(|| captured.reference.to_string())
}

enum Waited {
    Settled(Enrolled),

    Overtaken,

    Ended,
}

enum Enrolled {

    Stored {
        reference: String,
    },
    Refused(AxError),
}
