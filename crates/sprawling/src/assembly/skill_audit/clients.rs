// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Production audit effects (`crates/sprawling/spec/Assembly.lean`, city D19).
//! HTTP construction stays with gateway; program discovery stays with doctor.

use std::path::Path;
use std::process::Command;
use std::time::Duration;

use crate::doctor::asking::{Ended, TICK, ask};
use city::{AuditFetchError, HttpAudit, ScannerAudit};

fn fetch(url: &str) -> Result<HttpAudit, AuditFetchError> {
    let proxy = accounting::person::read()
        .map_err(|err| AuditFetchError::Unreachable(err.to_string()))?
        .proxying;
    let client = gateway::client_for(proxy, url)
        .timeout(city::SKILLS_SH_TIMEOUT)
        .build()
        .map_err(http_error)?;
    let response = client
        .get(url)
        .header("accept", "application/json")
        .send()
        .map_err(http_error)?;
    let status = response.status().as_u16();
    if status != 200 {
        return Ok(HttpAudit {
            status,
            body: Vec::new(),
        });
    }
    Ok(HttpAudit {
        status,
        body: response.bytes().map_err(http_error)?.to_vec(),
    })
}

pub(super) fn audit(request: &city::AuditRequest) -> city::AuditReport {
    match crate::doctor::host::find_program("skillspector") {
        Some(program) => city::audit_skill(
            request,
            &mut fetch,
            city::LocalScanner::Available(&mut |path| scan(&program, path)),
        ),
        None => city::audit_skill(request, &mut fetch, city::LocalScanner::Absent),
    }
}

fn scan(program: &Path, path: &Path) -> Result<ScannerAudit, AuditFetchError> {
    let version = answered(
        Command::new(program).arg("--version"),
        crate::doctor::PATIENCE,
    )?
    .1;
    let (code, body) = answered(
        Command::new(program)
            .arg("scan")
            .arg(path)
            .args(["--no-llm", "--format", "json"]),
        city::SKILLSPECTOR_TIMEOUT,
    )?;
    Ok(ScannerAudit {
        version: version.trim().to_owned(),
        code,
        body: body.into_bytes(),
    })
}

fn answered(
    command: &mut Command,
    patience: Duration,
) -> Result<(Option<i32>, String), AuditFetchError> {
    let knocks = u32::try_from(
        patience
            .as_millis()
            .checked_div(TICK.as_millis())
            .ok_or_else(|| {
                AuditFetchError::Unreachable("invalid scanner polling interval".to_owned())
            })?,
    )
    .map_err(|err| AuditFetchError::Unreachable(err.to_string()))?;
    match ask(command, knocks, |_| true) {
        Ended::Exited { code, kept } => Ok((code, kept)),
        Ended::Unstarted => Err(AuditFetchError::Unreachable(
            "skillspector could not start".to_owned(),
        )),
        Ended::Unanswered { stopping: None } => Err(AuditFetchError::Timeout),
        Ended::Unanswered {
            stopping: Some(reason),
        } => Err(AuditFetchError::Unreachable(reason)),
    }
}

fn http_error(err: reqwest::Error) -> AuditFetchError {
    if err.is_timeout() {
        AuditFetchError::Timeout
    } else {
        AuditFetchError::Unreachable(err.to_string())
    }
}
