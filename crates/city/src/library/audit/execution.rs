// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Advisory audit execution (`crates/city/spec/Library/Audit.lean`, D19).
//! Calls are injected; no thread, network client or ledger belongs here.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::Duration;

use kernel::event::record::{AuditSource, AuditVerdict, ShelvedFrom, SkillAudited};
use kernel::{AxCode, AxError, B3Hash};
use serde::Deserialize;

/// Deadline for one skills.sh request, including its response body.
pub const SKILLS_SH_TIMEOUT: Duration = Duration::from_secs(5);
/// Deadline for the optional local scan.
pub const SKILLSPECTOR_TIMEOUT: Duration = Duration::from_secs(60);

/// The landed version, never the source directory that supplied it.
pub struct AuditRequest {
    pub skill: String,
    pub digest: B3Hash,
    pub path: PathBuf,
    pub source: ShelvedFrom,
}

pub struct HttpAudit {
    pub status: u16,
    pub body: Vec<u8>,
}

pub struct ScannerAudit {
    pub version: String,
    pub code: Option<i32>,
    pub body: Vec<u8>,
}

/// Transport failures remain distinct until recorded as Unreachable.
#[derive(Debug, Clone)]
pub enum AuditFetchError {
    Timeout,
    Unreachable(String),
}

/// Applicability is known before checking or executing the landed content.
pub enum LocalScanner<'a> {
    Absent,
    Available(&'a mut dyn FnMut(&Path) -> Result<ScannerAudit, AuditFetchError>),
}

/// Fetch failures are advisory records plus actionable diagnostics.
#[derive(Default)]
#[must_use]
pub struct AuditReport {
    pub records: Vec<SkillAudited>,
    pub failures: Vec<AxError>,
}

/// Tries both applicable auditors, preserving every failure as Unreachable.
/// Content is rechecked on both sides of the calls; changed content never
/// receives a successful verdict for the requested digest. A missing local
/// scanner is ordinary absence and produces no record.
pub fn audit_skill(
    request: &AuditRequest,
    fetch: &mut dyn FnMut(&str) -> Result<HttpAudit, AuditFetchError>,
    scan: LocalScanner<'_>,
) -> AuditReport {
    let mut report = AuditReport::default();
    let remote = remote_name(request);
    if remote.is_none() && matches!(&scan, LocalScanner::Absent) {
        return report;
    }
    let before = unchanged(request);
    if let Some(name) = remote {
        let link = format!("https://skills.sh/{name}");
        let result = if let Err(err) = &before {
            Err(err.clone())
        } else {
            fetch(&format!("https://skills.sh/api/v1/skills/audit/{name}"))
                .map_err(|err| fetch_error("fetch a skill audit", &link, err))
                .and_then(|response| partners(request, response))
        };
        match result {
            Ok(records) => report.records.extend(records),
            Err(err) => report.failed(request, AuditSource::SkillsSh, Some(link), err),
        }
    }
    match scan {
        LocalScanner::Absent => {}
        LocalScanner::Available(scan) => {
            let result = before.and_then(|()| {
                scan(&request.path)
                    .map_err(|err| fetch_error("scan a shelved skill", &request.skill, err))
                    .and_then(|response| scanner(request, response))
            });
            match result {
                Ok(record) => report.records.push(record),
                Err(err) => report.failed(request, AuditSource::SkillSpector, None, err),
            }
        }
    }
    if report
        .records
        .iter()
        .any(|record| super::is_an_audit(record.verdict))
        && let Err(err) = unchanged(request)
    {
        for record in &mut report.records {
            record.verdict = AuditVerdict::Unreachable;
            record.risk = None;
            record.audited_at = None;
        }
        report.failures.push(err);
    }
    report
}

impl AuditReport {
    fn failed(
        &mut self,
        request: &AuditRequest,
        source: AuditSource,
        link: Option<String>,
        error: AxError,
    ) {
        self.records.push(SkillAudited {
            skill: request.skill.clone(),
            digest: request.digest,
            source,
            scanner: match source {
                AuditSource::SkillsSh => "skills.sh",
                AuditSource::SkillSpector => "skillspector",
            }
            .to_owned(),
            verdict: AuditVerdict::Unreachable,
            risk: None,
            audited_at: None,
            link,
        });
        self.failures.push(error);
    }
}

fn unchanged(request: &AuditRequest) -> Result<(), AxError> {
    if super::super::install::skill_digest(&request.path)? == request.digest {
        return Ok(());
    }
    Err(AxError::failure(
        AxCode::VersionConflict,
        "audit a shelved skill",
        &request.skill,
    )
    .with_recovery("audit the content now on the shelf; it changed during this audit"))
}

fn remote_name(request: &AuditRequest) -> Option<String> {
    let name = match &request.source {
        ShelvedFrom::SkillsSh { name } => name.clone(),
        ShelvedFrom::Git { url, .. } => {
            let repo = url
                .strip_prefix("https://github.com/")?
                .trim_end_matches('/');
            let repo = repo.strip_suffix(".git").unwrap_or(repo);
            if repo.split('/').count() != 2 {
                return None;
            }
            format!("{repo}/{}", request.skill)
        }
        ShelvedFrom::Path | ShelvedFrom::Shipped | ShelvedFrom::Page => return None,
    };
    (name.split('/').count() == 3
        && name.split('/').all(|part| {
            !part.is_empty()
                && part != "."
                && part != ".."
                && part
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b"-_.".contains(&b))
        }))
    .then_some(name)
}

#[derive(Deserialize)]
struct Partners {
    audits: BTreeMap<String, Partner>,
}

#[derive(Deserialize)]
struct Partner {
    status: Status,
    #[serde(rename = "riskLevel")]
    risk: Option<String>,
    #[serde(rename = "auditedAt")]
    at: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "lowercase")]
enum Status {
    #[serde(alias = "passed")]
    Pass,
    #[serde(alias = "warning")]
    Warn,
    #[serde(alias = "failed")]
    Fail,
}

fn partners(request: &AuditRequest, response: HttpAudit) -> Result<Vec<SkillAudited>, AxError> {
    if response.status != 200 {
        return Err(unavailable(
            "fetch a skill audit",
            format!("HTTP {} for {}", response.status, request.skill),
        ));
    }
    let parsed: Partners = serde_json::from_slice(&response.body)
        .map_err(|err| unavailable("parse a skill audit", err.to_string()))?;
    if parsed.audits.is_empty() || parsed.audits.keys().any(|name| name.trim().is_empty()) {
        return Err(unavailable(
            "parse a skill audit",
            "no named partner audits",
        ));
    }
    Ok(parsed
        .audits
        .into_iter()
        .map(|(name, partner)| SkillAudited {
            skill: request.skill.clone(),
            digest: request.digest,
            source: AuditSource::SkillsSh,
            scanner: name,
            verdict: match partner.status {
                Status::Pass => AuditVerdict::Pass,
                Status::Warn => AuditVerdict::Warn,
                Status::Fail => AuditVerdict::Fail,
            },
            risk: partner.risk,
            audited_at: partner.at,
            link: remote_name(request).map(|name| format!("https://skills.sh/{name}")),
        })
        .collect())
}

fn scanner(request: &AuditRequest, response: ScannerAudit) -> Result<SkillAudited, AxError> {
    let verdict = match response.code {
        Some(0) => AuditVerdict::Pass,
        Some(1) => AuditVerdict::Fail,
        Some(_) | None => {
            return Err(unavailable(
                "scan a shelved skill",
                format!("{} exited {:?}", response.version, response.code),
            ));
        }
    };
    let body: serde_json::Map<String, serde_json::Value> =
        serde_json::from_slice(&response.body)
            .map_err(|err| unavailable("parse a local skill audit", err.to_string()))?;
    let risk = body
        .get("risk_level")
        .map(|value| {
            value.as_str().map(str::to_owned).ok_or_else(|| {
                unavailable("parse a local skill audit", "risk_level is not a string")
            })
        })
        .transpose()?;
    if response.version.trim().is_empty() {
        return Err(unavailable(
            "read the scanner version",
            "skillspector returned no version",
        ));
    }
    Ok(SkillAudited {
        skill: request.skill.clone(),
        digest: request.digest,
        source: AuditSource::SkillSpector,
        scanner: response.version,
        verdict,
        risk,
        audited_at: None,
        link: None,
    })
}

fn fetch_error(action: &str, subject: &str, error: AuditFetchError) -> AxError {
    match error {
        AuditFetchError::Timeout => AxError::failure(AxCode::Timeout, action, subject)
            .with_recovery(
                "open the skill's safety page or check the local scanner; shelving continues",
            ),
        AuditFetchError::Unreachable(detail) => unavailable(action, format!("{subject}: {detail}")),
    }
}

fn unavailable(action: &str, subject: impl Into<String>) -> AxError {
    AxError::failure(AxCode::ToolUnavailable, action, subject).with_recovery(
        "open the skill's safety page or check the local scanner; shelving continues",
    )
}
