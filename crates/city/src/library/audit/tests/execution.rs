// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Offline conformance checks for city D19's audit execution.

use super::{AuditState, AuditVerdict, B3Hash, Fail, Pass, Seq, Warn, audit_state};
use crate::library::audit;

fn request(dir: &tempfile::TempDir) -> audit::AuditRequest {
    let path = dir.path().join("kiln.md");
    std::fs::write(&path, "# kiln\n").unwrap();
    audit::AuditRequest {
        skill: "kiln".to_owned(),
        digest: crate::skill_digest(&path).unwrap(),
        path,
        source: kernel::event::record::ShelvedFrom::SkillsSh {
            name: "owner/repo/kiln".to_owned(),
        },
    }
}

#[test]
fn audit_fetch_records_every_partner_and_preserves_its_metadata() {
    use kernel::event::record::{AuditSource, SkillAudited};
    let dir = tempfile::tempdir().unwrap();
    let request = request(&dir);
    let mut urls = Vec::new();
    let report = audit::audit_skill(
        &request,
        &mut |url| {
            urls.push(url.to_owned());
            Ok(audit::HttpAudit { status: 200, body: br#"{"audits":{"a":{"status":"pass","riskLevel":"low","auditedAt":"2026-01-01"},"b":{"status":"warn"},"c":{"status":"fail"}}}"#.to_vec() })
        },
        &mut |_| None,
    );
    let records = [
        (
            "a",
            Pass,
            Some("low".to_owned()),
            Some("2026-01-01".to_owned()),
        ),
        ("b", Warn, None, None),
        ("c", Fail, None, None),
    ]
    .into_iter()
    .map(|(name, verdict, risk, at)| SkillAudited {
        skill: request.skill.clone(),
        digest: request.digest,
        source: AuditSource::SkillsSh,
        scanner: name.to_owned(),
        verdict,
        risk,
        audited_at: at,
        link: Some("https://skills.sh/owner/repo/kiln".to_owned()),
    })
    .collect::<Vec<_>>();
    assert_eq!(report.records, records);
    assert!(report.failures.is_empty());
    assert_eq!(
        urls,
        vec!["https://skills.sh/api/v1/skills/audit/owner/repo/kiln"]
    );
}

#[test]
fn audit_failures_are_unreachable_without_changing_shelved_bytes() {
    use audit::{AuditFetchError, HttpAudit};
    use kernel::AxCode;
    for (answer, code) in [
        (Err(AuditFetchError::Timeout), AxCode::Timeout),
        (
            Err(AuditFetchError::Unreachable("offline".to_owned())),
            AxCode::ToolUnavailable,
        ),
        (
            Ok(HttpAudit {
                status: 403,
                body: Vec::new(),
            }),
            AxCode::ToolUnavailable,
        ),
        (
            Ok(HttpAudit {
                status: 201,
                body: Vec::new(),
            }),
            AxCode::ToolUnavailable,
        ),
        (
            Ok(HttpAudit {
                status: 200,
                body: b"not json".to_vec(),
            }),
            AxCode::ToolUnavailable,
        ),
        (
            Ok(HttpAudit {
                status: 200,
                body: br#"{"audits":{"socket":{"status":"unknown"}}}"#.to_vec(),
            }),
            AxCode::ToolUnavailable,
        ),
        (
            Ok(HttpAudit {
                status: 200,
                body: br#"{"audits":{}}"#.to_vec(),
            }),
            AxCode::ToolUnavailable,
        ),
    ] {
        let dir = tempfile::tempdir().unwrap();
        let request = request(&dir);
        let mut answer = Some(answer);
        let report = audit::audit_skill(&request, &mut |_| answer.take().unwrap(), &mut |_| None);
        assert_eq!(report.failures.len(), 1);
        assert_eq!(report.failures.first().unwrap().code(), &code);
        assert_eq!(report.records.len(), 1);
        assert_eq!(
            report.records.first().unwrap().verdict,
            AuditVerdict::Unreachable
        );
        assert_eq!(report.records.first().unwrap().digest, request.digest);
        assert_eq!(std::fs::read(&request.path).unwrap(), b"# kiln\n");
        assert_eq!(
            audit_state(
                &request.digest,
                &[(
                    request.digest,
                    report.records.first().unwrap().verdict,
                    Seq::new(1)
                )]
            ),
            AuditState::Unaudited
        );
    }
}

#[test]
fn audit_of_content_changed_during_fetch_is_not_a_successful_audit() {
    let dir = tempfile::tempdir().unwrap();
    let request = request(&dir);
    let report = audit::audit_skill(
        &request,
        &mut |_| {
            std::fs::write(&request.path, "# changed\n").unwrap();
            Ok(audit::HttpAudit {
                status: 200,
                body: br#"{"audits":{"socket":{"status":"pass"}}}"#.to_vec(),
            })
        },
        &mut |_| None,
    );
    assert_eq!(
        report.records.first().unwrap().verdict,
        AuditVerdict::Unreachable
    );
    assert_eq!(
        report.failures.first().unwrap().code(),
        &kernel::AxCode::VersionConflict
    );
    let current = crate::skill_digest(&request.path).unwrap();
    assert_ne!(current, request.digest);
    assert_eq!(
        audit_state(&current, &[(request.digest, Pass, Seq::new(1))]),
        AuditState::Stale {
            audited: request.digest
        }
    );
}

#[test]
fn audit_local_scanner_runs_after_remote_failure_and_absence_writes_nothing() {
    let dir = tempfile::tempdir().unwrap();
    let mut request = request(&dir);
    for (exit, expected) in [
        (Some(0), Pass),
        (Some(1), Fail),
        (Some(2), AuditVerdict::Unreachable),
        (None, AuditVerdict::Unreachable),
    ] {
        let report = audit::audit_skill(
            &request,
            &mut |_| Err(audit::AuditFetchError::Timeout),
            &mut |path| {
                assert_eq!(path, request.path);
                Some(Ok(audit::ScannerAudit {
                    version: "skillspector 1".to_owned(),
                    code: exit,
                    body: br#"{"risk_level":"high"}"#.to_vec(),
                }))
            },
        );
        assert_eq!(
            report.records.iter().map(|r| r.verdict).collect::<Vec<_>>(),
            vec![AuditVerdict::Unreachable, expected]
        );
    }
    request.source = kernel::event::record::ShelvedFrom::Path;
    let report = audit::audit_skill(
        &request,
        &mut |_| panic!("local content has no remote auditor"),
        &mut |_| None,
    );
    assert!(report.records.is_empty());
    assert!(report.failures.is_empty());
}

#[test]
fn audit_uses_the_installed_package_digest_when_only_a_script_changes() {
    let dir = tempfile::tempdir().unwrap();
    let package = dir.path().join("kiln");
    std::fs::create_dir(&package).unwrap();
    std::fs::write(
        package.join("SKILL.md"),
        "# kiln
",
    )
    .unwrap();
    std::fs::write(
        package.join("run.py"),
        "print(1)
",
    )
    .unwrap();
    let slot = crate::Slot::library("tools").unwrap();
    let installed = crate::install_skill(dir.path(), &slot, &package, &mut |bytes| {
        Ok(B3Hash::digest(bytes))
    })
    .unwrap();
    let path = dir
        .path()
        .join(installed.holding.package.as_ref().unwrap().as_str());
    assert_eq!(crate::skill_digest(&path).unwrap(), installed.hash);
    assert_ne!(installed.hash, installed.holding.hash);
    let request = audit::AuditRequest {
        skill: "kiln".to_owned(),
        digest: installed.hash,
        path,
        source: kernel::event::record::ShelvedFrom::Path,
    };
    let report = audit::audit_skill(
        &request,
        &mut |_| panic!("no remote auditor"),
        &mut |path| {
            std::fs::write(
                path.join("run.py"),
                "print(2)
",
            )
            .unwrap();
            Some(Ok(audit::ScannerAudit {
                version: "skillspector 1".to_owned(),
                code: Some(0),
                body: b"{}".to_vec(),
            }))
        },
    );
    assert_eq!(
        report.records.first().unwrap().verdict,
        AuditVerdict::Unreachable
    );
    assert_eq!(
        report.failures.first().unwrap().code(),
        &kernel::AxCode::VersionConflict
    );
    let current = crate::skill_digest(&request.path).unwrap();
    assert_ne!(current, installed.hash);
    assert_eq!(
        B3Hash::digest(&std::fs::read(request.path.join("SKILL.md")).unwrap()),
        installed.holding.hash
    );
}

#[test]
fn audit_stale_before_a_local_scan_records_unreachable_without_running_it() {
    let dir = tempfile::tempdir().unwrap();
    let mut request = request(&dir);
    request.source = kernel::event::record::ShelvedFrom::Path;
    std::fs::write(&request.path, "# changed\n").unwrap();
    let report = audit::audit_skill(&request, &mut |_| panic!("no remote auditor"), &mut |_| {
        panic!("stale content must not be scanned")
    });
    assert_eq!(report.records.len(), 1);
    assert_eq!(
        report.records.first().unwrap().verdict,
        AuditVerdict::Unreachable
    );
    assert_eq!(
        report.failures.first().unwrap().code(),
        &kernel::AxCode::VersionConflict
    );
}
