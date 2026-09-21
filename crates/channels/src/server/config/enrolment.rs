// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The one door that waits: a credential is taken from a caller on this
//! machine, and this request holds until the city says what became of it.

use std::net::SocketAddr;
use std::sync::Arc;

use axum::body::Bytes;
use axum::extract::{ConnectInfo, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use kernel::{AxError, EventKind, Sealed, SecretRef};
use tokio::sync::broadcast;

use crate::command::Command;
use crate::reception::{EnrollVerdict, decide_enroll};

use super::super::reply::{Delivered, Reply, refusal_text};
use super::{ENROLMENT_PATIENCE, EnrollBody, ShellState};

/// The one route that carries a credential. It exists as HTTP rather
/// than as a socket frame because the socket's Command type cannot spell
/// a secret; here the same rule is enforced against the peer address.
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
    // The grammar of a vault place is `kernel::SecretRef`'s and this
    // route reads it back through the same parser every other reader
    // uses. Spelling it here with `format!` accepted a realm or a name
    // that no later reader could resolve, and answered 201 with the
    // unresolvable text (roadmap M-22).
    let place = match SecretRef::parse(&format!("secret:{realm}/{name}")) {
        Ok(place) => place,
        Err(err) => {
            return (StatusCode::UNPROCESSABLE_ENTITY, refusal_text(&err)).into_response();
        }
    };
    let reference = place.to_string();
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
                    Some(err) => return Waited::Settled(Enrolled::Refused(err)),
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
                            return Waited::Settled(Enrolled::Stored);
                        }
                    }
                    // This request was overtaken: the record that says what
                    // became of the credential may be among the ones the
                    // subscription skipped, and a skipped record is not
                    // sent again. Waiting for the patience to run out would
                    // only delay an answer this wait can no longer give.
                    Err(broadcast::error::RecvError::Lagged(_)) => return Waited::Overtaken,
                    Err(broadcast::error::RecvError::Closed) => return Waited::Ended,
                },
            }
        }
    })
    .await;
    match waited {
        Ok(Waited::Settled(Enrolled::Stored)) => (StatusCode::CREATED, reference).into_response(),
        Ok(Waited::Settled(Enrolled::Refused(err))) => {
            (StatusCode::UNPROCESSABLE_ENTITY, refusal_text(&err)).into_response()
        }
        // Nothing arrived inside the patience. Two-oh-two says so in the one
        // word HTTP has for it, and the body says why rather than leaving the
        // caller to read a status code as an outcome.
        Err(_) => (
            StatusCode::ACCEPTED,
            format!(
                "{reference} was handed to the city and it has not answered within {}s; the \
                 worker may be inside a dispatch. Check whether the reference resolves before \
                 sending the credential again",
                ENROLMENT_PATIENCE.as_secs()
            ),
        )
            .into_response(),
        // The event stream went past this request, which is a different
        // fact and gets its own sentence: the city may have stored the
        // credential and this server cannot say whether it did.
        Ok(Waited::Overtaken) => (
            StatusCode::ACCEPTED,
            format!(
                "{reference} was handed to the city, and this server's event stream moved past \
                 the request before the vault said what became of it. Check whether the \
                 reference resolves before sending the credential again"
            ),
        )
            .into_response(),
        // The stream ended outright, which says nothing about the credential
        // either; the city is going down, or its writer is.
        Ok(Waited::Ended) => (
            StatusCode::ACCEPTED,
            format!(
                "{reference} was handed to the city, and the city's event stream ended before \
                 the vault said what became of it. Check whether the reference resolves before \
                 sending the credential again"
            ),
        )
            .into_response(),
    }
}

/// What the city said about one enrolment, or why the wait for it ended.
///
/// `Stored` and `Refused` are the two answers the worker gives; the other
/// two are the ways this route stops waiting, and they are separate arms
/// because they are separate facts: a stream that overtook the request may
/// have carried the answer past unread, and a stream that ended never
/// carried it at all.
enum Waited {
    Settled(Enrolled),
    /// The event stream skipped records while this request waited.
    Overtaken,
    /// The event stream ended outright.
    Ended,
}

/// What the city said about one enrolment.
enum Enrolled {
    Stored,
    Refused(AxError),
}
