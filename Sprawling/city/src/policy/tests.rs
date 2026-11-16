// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use super::*;

fn addr(raw: &str) -> Address {
    Address::parse(raw).unwrap()
}

fn ordinary(rest: &str) -> String {
    format!("confidential = false\nwrite = \"everything\"\n{rest}")
}

fn shut(rest: &str) -> String {
    format!("confidential = true\nwrite = \"everything\"\n{rest}")
}

#[test]
fn a_building_without_a_file_is_an_ordinary_building() {
    let dir = tempfile::tempdir().unwrap();
    let rules = load(dir.path(), &addr("lab")).unwrap();
    assert!(!rules.policy().confidential);
    assert_eq!(rules.model_pool(), ModelPool::Any);

    assert_eq!(rules.write_domain().unwrap().prefixes().count(), 1);
    std::fs::write(dir.path().join("NOTES.md"), "a file, so it has no rules").unwrap();
    let file = load(dir.path(), &addr("NOTES.md")).unwrap();
    assert_eq!(file.policy(), rules.policy());
}

#[test]
fn a_buildings_rules_sit_where_its_own_runs_cannot_write() {
    let dir = tempfile::tempdir().unwrap();
    let lab = addr("lab");
    let file = rules_path(dir.path(), &lab);
    let relative = file
        .strip_prefix(dir.path())
        .unwrap()
        .to_string_lossy()
        .replace('\\', "/");
    let target = Address::parse(&relative).unwrap();
    assert!(target.is_reserved(), "{relative}");

    let domain = load(dir.path(), &lab).unwrap().write_domain().unwrap();
    assert!(
        matches!(
            domain.admits(&target),
            kernel::DomainVerdict::Outside { .. }
        ),
        "a run in this building can rewrite the rules that govern it"
    );
}

#[test]
fn rules_in_a_document_this_version_does_not_read_are_refused_rather_than_ignored() {
    for stale in [
        std::path::PathBuf::from("lab").join(SUPERSEDED_FILE),
        std::path::PathBuf::from("lab")
            .join(kernel::RESERVED_PREFIX)
            .join(SUPERSEDED_FILE),
    ] {
        let dir = tempfile::tempdir().unwrap();
        let at = dir.path().join(&stale);
        std::fs::create_dir_all(at.parent().unwrap()).unwrap();
        std::fs::write(&at, "`confidential: true`\n").unwrap();
        let err = load(dir.path(), &addr("lab")).unwrap_err();
        assert_eq!(err.code(), &AxCode::ConfigInvalid, "{}", stale.display());
        assert!(
            err.recovery().contains(RULES_FILE),
            "the refusal does not say where the rules go: {}",
            err.recovery()
        );
    }
}

#[test]
fn a_file_that_does_not_say_is_refused_rather_than_assumed_open() {
    let err = evaluate(&addr("lab"), "write = \"everything\"\n").unwrap_err();
    assert_eq!(err.code(), &AxCode::ConfigInvalid);
    assert!(
        err.subject().contains("confidential"),
        "the deserialiser names the key it wanted: {}",
        err.subject()
    );
}

#[test]
fn a_typo_in_the_privacy_setting_does_not_resolve_to_the_permissive_side() {
    let err = evaluate(
        &addr("lab"),
        "confidential = \"yes\"\nwrite = \"everything\"\n",
    )
    .unwrap_err();
    assert_eq!(err.code(), &AxCode::ConfigInvalid);
    assert!(err.subject().contains("confidential"), "{}", err.subject());
}

#[test]
fn a_key_this_version_does_not_know_is_refused_rather_than_skipped() {
    let err = evaluate(&addr("lab"), &ordinary("desktopp = true\n")).unwrap_err();
    assert_eq!(err.code(), &AxCode::ConfigInvalid);
    assert!(err.subject().contains("desktopp"), "{}", err.subject());
}

fn placeholder(lines: &[&str]) -> Option<(usize, usize)> {
    let header = lines.iter().position(|line| {
        line.strip_prefix("# ")
            .is_some_and(|text| text.starts_with("[workspace_models]"))
    })?;
    let end = lines[header..]
        .iter()
        .position(|line| !line.starts_with('#'))
        .map_or(lines.len(), |stop| header.saturating_add(stop));
    Some((header, end))
}

fn uncovered(template: &str) -> Option<String> {
    let lines: Vec<&str> = template.lines().collect();
    let (start, end) = placeholder(&lines)?;
    let mut out = lines[..start].to_vec();
    out.extend(
        lines[start..end]
            .iter()
            .map(|line| line.strip_prefix("# ").unwrap_or(line)),
    );
    out.extend(lines[end..].iter().copied());
    Some(out.join("\n"))
}

#[test]
fn the_template_hands_a_building_a_working_routing_placeholder() {
    let dir = tempfile::tempdir().unwrap();
    let lab = addr("lab");
    crate::building::create(dir.path(), &lab, crate::BuildingTemplate::Minimal).unwrap();
    let template = std::fs::read_to_string(rules_path(dir.path(), &lab)).unwrap();
    let quiet = evaluate(&lab, &template).unwrap();
    assert!(
        quiet.workspace_models().is_none(),
        "a placeholder must not enable routing on its own"
    );

    let filled = uncovered(&template).expect("the template carries no routing placeholder");
    let placed = evaluate(&lab, &filled)
        .unwrap_or_else(|err| panic!("the placeholder is not a policy this reader accepts: {err}"));
    let policy = placed
        .workspace_models()
        .expect("an uncovered placeholder is a policy");
    assert!(!policy.allowed().is_empty());
    assert_eq!(policy.recommendations().len(), 3);
}

fn workspace_rules() -> String {
    ordinary(
        r#"[workspace_models]
allowed = [{ endpoint = "house", model = "ordinary" }, { endpoint = "house", model = "strong" }]
max_effort = "high"
[workspace_models.recommendations.easy]
endpoint = "house"
model = "ordinary"
effort = "low"
[workspace_models.recommendations.medium]
endpoint = "house"
model = "ordinary"
effort = "medium"
[workspace_models.recommendations.hard]
endpoint = "house"
model = "strong"
effort = "high"
"#,
    )
}

#[test]
fn a_workspace_model_policy_is_read_from_the_rules() {
    let rules = evaluate(&addr("shop"), &workspace_rules()).unwrap();
    let policy = rules.workspace_models().unwrap();
    assert_eq!(policy.allowed().len(), 2);
    assert_eq!(
        policy.recommendation(kernel::Difficulty::Hard).model(),
        &kernel::ModelRef {
            endpoint: "house".to_owned(),
            model: "strong".to_owned(),
        }
    );
    assert_eq!(policy.digest().unwrap(), policy.digest().unwrap());
    assert!(policy.choice().is_none());
}

#[test]
fn a_direct_choice_is_read_and_a_missing_section_gains_one() {
    let chosen = format!(
        "{}\n[workspace_models.choice]\nendpoint = \"house\"\nmodel = \"strong\"\neffort = \"high\"\n",
        workspace_rules()
    );
    let policy = evaluate(&addr("shop"), &chosen)
        .unwrap()
        .workspace_models()
        .unwrap()
        .clone();
    assert_eq!(
        policy.choice().map(crate::ModelRecommendation::model),
        Some(&kernel::ModelRef {
            endpoint: "house".to_owned(),
            model: "strong".to_owned(),
        })
    );
    let inscribed =
        crate::inscribe_choice(&workspace_rules(), "house", "ordinary", kernel::Effort::Low);
    let again = evaluate(&addr("shop"), &inscribed)
        .unwrap()
        .workspace_models()
        .unwrap()
        .clone();
    assert_eq!(again.choice().unwrap().effort(), kernel::Effort::Low);
    assert_eq!(
        again.recommendation(kernel::Difficulty::Hard).model().model,
        "strong",
        "inscribing a choice leaves the difficulty shortcuts"
    );
    let multiline = ordinary(
        r#"[workspace_models]
allowed = [
  { endpoint = "house", model = "ordinary" },
]
max_effort = "high"
[workspace_models.recommendations.easy]
endpoint = "house"
model = "ordinary"
effort = "low"
[workspace_models.recommendations.medium]
endpoint = "house"
model = "ordinary"
effort = "medium"
[workspace_models.recommendations.hard]
endpoint = "house"
model = "ordinary"
effort = "high"
"#,
    );
    let widened = crate::inscribe_choice(&multiline, "house", "strong", kernel::Effort::High);
    let widened_policy = evaluate(&addr("shop"), &widened)
        .unwrap()
        .workspace_models()
        .unwrap()
        .clone();
    assert!(
        widened_policy
            .allowed()
            .iter()
            .any(|each| each.model == "strong"),
        "a multiline allowed pool gains the chosen model"
    );
    let fresh = crate::inscribe_choice(&ordinary(""), "house", "ordinary", kernel::Effort::High);
    let opened = evaluate(&addr("shop"), &fresh).unwrap();
    assert_eq!(
        opened
            .workspace_models()
            .unwrap()
            .choice()
            .unwrap()
            .model()
            .model,
        "ordinary"
    );
    let from_missing = crate::inscribe_choice("", "house", "ordinary", kernel::Effort::Medium);
    let missing = evaluate(&addr("shop"), &from_missing).unwrap();
    assert_eq!(
        missing
            .workspace_models()
            .unwrap()
            .choice()
            .unwrap()
            .model()
            .model,
        "ordinary",
        "an empty RULES.toml opens with ordinary confidential/write before the choice"
    );
    let quoted =
        crate::inscribe_choice(&ordinary(""), "house", "vendor\"model", kernel::Effort::Low);
    assert!(
        quoted.contains('\'') || quoted.contains("\\\""),
        "model ids with quotes are TOML-escaped: {quoted}"
    );
    let quoted_policy = evaluate(&addr("shop"), &quoted).unwrap();
    assert_eq!(
        quoted_policy
            .workspace_models()
            .unwrap()
            .choice()
            .unwrap()
            .model()
            .model,
        "vendor\"model"
    );
    let commented = format!(
        "{}\n[workspace_models.choice] # selected\nendpoint = \"house\"\nmodel = \"ordinary\"\neffort = \"low\"\n",
        workspace_rules()
    );
    let replaced = crate::inscribe_choice(&commented, "house", "strong", kernel::Effort::High);
    assert_eq!(
        replaced.matches("[workspace_models.choice]").count(),
        1,
        "a choice header with a trailing comment is replaced, not duplicated"
    );
    let replaced_policy = evaluate(&addr("shop"), &replaced).unwrap();
    assert_eq!(
        replaced_policy
            .workspace_models()
            .unwrap()
            .choice()
            .unwrap()
            .model()
            .model,
        "strong"
    );
}

#[test]
fn a_workspace_policy_rejects_an_empty_pool_an_outside_model_and_an_effort_above_the_cap() {
    for text in [
        workspace_rules().replace(
            "allowed = [{ endpoint = \"house\", model = \"ordinary\" }, { endpoint = \"house\", model = \"strong\" }]",
            "allowed = []",
        ),
        workspace_rules().replace("model = \"strong\"\neffort = \"high\"", "model = \"other\"\neffort = \"high\""),
        workspace_rules().replace("max_effort = \"high\"", "max_effort = \"low\""),
        workspace_rules().replace("effort = \"low\"", "effort = \"none\""),
    ] {
        assert_eq!(
            evaluate(&addr("shop"), &text).unwrap_err().code(),
            &AxCode::ConfigInvalid,
            "{text}"
        );
    }
}

#[test]
fn the_prose_a_person_writes_is_part_of_the_schema() {
    evaluate(
        &addr("lab"),
        &ordinary("does = \"a lab\"\nconventions = \"tests first\"\n"),
    )
    .unwrap();
    let err = evaluate(&addr("lab"), &ordinary("convention = \"tests first\"\n")).unwrap_err();
    assert!(err.subject().contains("convention"), "{}", err.subject());
}

#[test]
fn the_form_a_building_is_laid_out_with_is_one_this_reader_accepts() {
    let dir = tempfile::tempdir().unwrap();
    for (template, confidential) in [
        (crate::BuildingTemplate::Minimal, false),
        (crate::BuildingTemplate::Confidential, true),
    ] {
        let at = addr(if confidential { "vault" } else { "lab" });
        crate::building::create(dir.path(), &at, template).unwrap();
        let rules = load(dir.path(), &at).unwrap();
        assert_eq!(rules.policy().confidential, confidential, "{template:?}");
    }
}

#[test]
fn a_user_can_hand_edit_the_rules_in_the_buildings_reserved_subtree() {
    let dir = tempfile::tempdir().unwrap();
    let lab = addr("lab");
    crate::building::create(dir.path(), &lab, crate::BuildingTemplate::Minimal).unwrap();
    let path = rules_path(dir.path(), &lab);
    let edited = std::fs::read_to_string(&path)
        .unwrap()
        .replace("review = false", "review = true");
    std::fs::write(&path, edited).unwrap();

    assert!(load(dir.path(), &lab).unwrap().review());
}

#[test]
fn a_malformed_user_edit_is_not_reported_as_a_resident_permission_refusal() {
    let err = evaluate(&addr("lab"), "confidential = false\nwrite = [").unwrap_err();
    assert_eq!(err.code(), &AxCode::ConfigInvalid);
    assert!(err.recovery().contains("lab/.sprawling/RULES.toml"));
    assert!(err.recovery().contains("User can edit"));
    assert!(!err.recovery().contains("resident cannot write"));
}

mod surfaces;
