// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::sync::Arc;

use axum::Json;
use axum::body::Bytes;
use axum::extract::{
    Query, State,
    rejection::{BytesRejection, JsonRejection, QueryRejection},
};
use axum::http::{StatusCode, header};
use axum::response::{IntoResponse, Response};
use kernel::{AxCode, AxError, B3Hash};
use serde::{Deserialize, Serialize};

use super::config::ShellState;

pub(super) const SAVE_BYTES_MAX: usize = 8 * 1024 * 1024;

pub type WorkspaceSink =
    Arc<dyn Fn(WorkspaceOperation) -> Result<WorkspaceReply, AxError> + Send + Sync>;

#[derive(Debug, Deserialize)]
pub struct WorkspaceSave {
    pub workspace: B3Hash,
    pub id: B3Hash,
    pub baseline: B3Hash,
}

pub enum WorkspaceOperation {
    Request(WorkspaceRequest),
    Save(WorkspaceSave, Bytes),
}

#[derive(Debug, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum WorkspaceRequest {
    Workspaces,
    Register {
        root: String,
    },
    List {
        workspace: B3Hash,
        path: String,
        cursor: Option<String>,
    },
    Watch {
        workspace: B3Hash,
        path: String,
        revision: String,
    },
    Open {
        workspace: B3Hash,
        target: WorkspaceTarget,
    },
    Bytes {
        workspace: B3Hash,
        id: B3Hash,
        version: B3Hash,
    },
    Resource {
        workspace: B3Hash,
        path: String,
    },
    Remember {
        workspace: B3Hash,
        id: B3Hash,
        changes: Vec<WorkspaceChange>,
    },
    Relink {
        workspace: B3Hash,
        id: B3Hash,
        path: String,
    },
}

#[derive(Debug, Deserialize)]
#[serde(untagged, deny_unknown_fields)]
pub enum WorkspaceTarget {
    Path { path: String },
    Record { id: B3Hash },
}

#[derive(Debug, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum WorkspaceChange {
    Progress { progress: ReadingPosition },
    Category { category: Option<String> },
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ReadingPosition {
    pub page: u32,
    pub offset: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceRecord {
    pub id: B3Hash,
    pub path: String,
    pub version: B3Hash,
    pub progress: Option<ReadingPosition>,
    pub category: Option<String>,
    pub last_opened: u64,
}

#[derive(Debug, Serialize)]
pub struct WorkspaceInfo {
    pub id: B3Hash,
    pub name: String,
    pub root: String,
    pub recent: Vec<WorkspaceRecord>,
}

#[derive(Debug, Clone, Serialize)]
pub struct WorkspaceEntry {
    pub name: String,
    pub path: String,
    pub kind: WorkspaceEntryKind,
}

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkspaceEntryKind {
    Directory,
    File,
}

#[derive(Debug, Serialize)]
pub struct WorkspacePage {
    pub entries: Vec<WorkspaceEntry>,
    pub next: Option<String>,
    pub revision: Option<String>,
}

pub struct WorkspaceWatch {
    pub revision: String,
    pub observed: Option<tokio::sync::watch::Receiver<Option<String>>>,
}

impl WorkspaceWatch {
    async fn response(self) -> Response {
        #[derive(Serialize)]
        struct Revision {
            revision: Option<String>,
        }
        let Self { revision, observed } = self;
        let changed = match observed {
            Some(mut observed) => {
                match tokio::time::timeout(
                    std::time::Duration::from_secs(30),
                    observed.wait_for(|current| current.as_ref() != Some(&revision)),
                )
                .await
                {
                    Ok(Ok(current)) => current.clone(),
                    Ok(Err(_)) => None,
                    Err(_) => Some(revision),
                }
            }
            None => None,
        };
        Json(Revision { revision: changed }).into_response()
    }
}

pub enum WorkspaceReply {
    Watch(WorkspaceWatch),
    Workspaces(Vec<WorkspaceInfo>),
    Registered(WorkspaceInfo),
    Page(WorkspacePage),
    Record(WorkspaceRecord),
    Bytes(Vec<u8>),
}

impl WorkspaceReply {
    async fn respond(self) -> Response {
        match self {
            Self::Watch(watch) => watch.response().await,
            Self::Workspaces(list) => Json(list).into_response(),
            Self::Registered(workspace) => Json(workspace).into_response(),
            Self::Page(page) => Json(page).into_response(),
            Self::Record(record) => Json(record).into_response(),
            Self::Bytes(bytes) => {
                ([(header::CONTENT_TYPE, "application/octet-stream")], bytes).into_response()
            }
        }
    }
}

pub(super) async fn request(
    State(state): State<Arc<ShellState>>,
    body: Result<Json<WorkspaceRequest>, JsonRejection>,
) -> Response {
    match body {
        Ok(Json(request)) => serve(&state, WorkspaceOperation::Request(request)).await,
        Err(err) => invalid(err.body_text()),
    }
}

pub(super) async fn save(
    State(state): State<Arc<ShellState>>,
    query: Result<Query<WorkspaceSave>, QueryRejection>,
    bytes: Result<Bytes, BytesRejection>,
) -> Response {
    match (query, bytes) {
        (Ok(Query(save)), Ok(bytes)) => serve(&state, WorkspaceOperation::Save(save, bytes)).await,
        (Err(err), _) => invalid(err.body_text()),
        (Ok(_), Err(err)) => invalid(err.body_text()),
    }
}

async fn serve(state: &ShellState, operation: WorkspaceOperation) -> Response {
    let Some(sink) = state.workspace.clone() else {
        return refusal(
            AxError::failure(
                AxCode::ToolUnavailable,
                "open local workspace files",
                "this listener has no local workspace registry",
            )
            .with_recovery("open RefRain directly from the city host"),
        );
    };
    match tokio::task::spawn_blocking(move || sink(operation)).await {
        Ok(Ok(reply)) => reply.respond().await,
        Ok(Err(error)) => refusal(error),
        Err(error) => refusal(
            AxError::failure(
                AxCode::StorageFatal,
                "complete the workspace request",
                error.to_string(),
            )
            .effect_unknown()
            .with_recovery(
                "reopen the file and check its current version before repeating the request",
            ),
        ),
    }
}

fn invalid(detail: String) -> Response {
    refusal(
        AxError::failure(AxCode::InvalidArgs, "read the workspace request", detail)
            .with_recovery("check the workspace request fields and try again"),
    )
}

pub(super) fn refusal(error: AxError) -> Response {
    let status = if error.code() == &AxCode::GateDenied {
        StatusCode::FORBIDDEN
    } else if error.code() == &AxCode::PathNotFound {
        StatusCode::NOT_FOUND
    } else if error.code() == &AxCode::VersionConflict {
        StatusCode::CONFLICT
    } else if error.code() == &AxCode::InvalidArgs {
        StatusCode::UNPROCESSABLE_ENTITY
    } else if error.code() == &AxCode::ToolUnavailable {
        StatusCode::NOT_IMPLEMENTED
    } else {
        StatusCode::INTERNAL_SERVER_ERROR
    };
    (status, Json(error)).into_response()
}
