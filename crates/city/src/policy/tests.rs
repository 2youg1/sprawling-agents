// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use super::*;

fn addr(raw: &str) -> Address {
    Address::parse(raw).unwrap()
}

#[test]
fn a_building_without_a_file_is_an_ordinary_building() {
    let dir = tempfile::tempdir().unwrap();
    let rules = load(dir.path(), &addr("lab")).unwrap();
    assert!(!rules.policy().confidential);
    assert_eq!(rules.model_pool(), ModelPool::Any);
    // With nothing declared, a building may write itself and no more.
    let domain = rules.write_domain().unwrap();
    assert_eq!(domain.prefixes().count(), 1);
}

/// The rules of a building are not writable by the runs they govern,
/// and the write domain those rules declare is the one that has to
/// fail to reach them.
#[test]
fn a_buildings_rules_sit_where_its_own_runs_cannot_write() {
    let dir = tempfile::tempdir().unwrap();
    let lab = addr("lab");
    let file = building_path(dir.path(), &lab);
    let relative = file
        .strip_prefix(dir.path())
        .unwrap()
        .to_string_lossy()
        .replace('\\', "/");
    let target = Address::parse(&relative).unwrap();
    assert!(target.is_reserved(), "{relative}");

    // The domain a building with no declarations gets: itself. It
    // must still fail to reach the file that would have declared
    // something else.
    let domain = load(dir.path(), &lab).unwrap().write_domain().unwrap();
    assert!(
        matches!(
            domain.admits(&target),
            kernel::DomainVerdict::Outside { .. }
        ),
        "a run in this building can rewrite the rules that govern it"
    );
}

/// A city raised before the move must not come back with its rules
/// silently defaulted: `load` treats an absent file as an ordinary
/// building, so a confidential one would quietly stop being
/// confidential.
#[test]
fn rules_left_at_the_old_address_are_refused_rather_than_ignored() {
    let dir = tempfile::tempdir().unwrap();
    let lab = addr("lab");
    std::fs::create_dir_all(dir.path().join("lab")).unwrap();
    std::fs::write(
        dir.path().join("lab").join(BUILDING_FILE),
        "# BUILDING.md\n\n## confidential\n\n`confidential: true`\n",
    )
    .unwrap();
    let err = load(dir.path(), &lab).unwrap_err();
    assert_eq!(err.code(), &AxCode::ConfigInvalid);
    assert!(
        err.recovery().contains(".sprawling"),
        "the refusal does not say where the file goes: {}",
        err.recovery()
    );
}

#[test]
fn a_file_that_does_not_say_is_refused_rather_than_assumed_open() {
    let err = evaluate(&addr("lab"), "# BUILDING.md\n\nno declaration here\n").unwrap_err();
    assert_eq!(err.code(), &AxCode::ConfigInvalid);
    assert!(err.recovery().contains("confidential: false"));
}

#[test]
fn a_typo_in_the_privacy_setting_does_not_resolve_to_the_permissive_side() {
    let err = evaluate(&addr("lab"), "`confidential: yes`\n").unwrap_err();
    assert!(err.to_string().contains("neither true nor false"));
}

#[test]
fn a_confidential_building_locks_the_model_pool_and_its_own_subtree() {
    let rules = evaluate(
        &addr("vault"),
        "# BUILDING.md\n\n## confidential\n\n`confidential: true`\n\n\
         ## Write domains\n\n- vault/work\n- vault/notes\n",
    )
    .unwrap();
    assert!(rules.policy().confidential);
    assert_eq!(rules.model_pool(), ModelPool::LocalOnly);
    let domain = rules.write_domain().unwrap();
    assert_eq!(domain.prefixes().count(), 2);
}

#[test]
fn a_confidential_building_reaching_outside_itself_is_refused_by_name() {
    let rules = evaluate(
        &addr("vault"),
        "`confidential: true`\n\n## Write domains\n\n- vault/work\n- lab/shared\n",
    )
    .unwrap();
    let err = rules.write_domain().unwrap_err();
    assert!(err.to_string().contains("lab/shared"));
    assert!(err.recovery().contains("confidential: true"));
}

#[test]
fn a_building_reaches_the_domains_it_names_and_nothing_else() {
    let rules = evaluate(
        &addr("lab"),
        "`confidential: false`\n\n## Egress\n\n- crates.io\n- `docs.rs`\n",
    )
    .unwrap();
    assert!(rules.egress().admits("static.crates.io"));
    assert!(!rules.egress().admits("pastebin.test"));
}

#[test]
fn a_confidential_building_that_also_lists_domains_is_a_contradiction_and_is_refused() {
    let err = evaluate(
        &addr("vault"),
        "`confidential: true`\n\n## Egress\n\n- example.com\n",
    )
    .unwrap_err();
    assert!(err.recovery().contains("does not leave"));
}

#[test]
fn a_confidential_building_reaches_nothing_public() {
    let rules = evaluate(&addr("vault"), "`confidential: true`\n").unwrap();
    assert!(rules.egress().is_empty());
    assert!(!rules.egress().admits("example.com"));
}

#[test]
fn an_ordinary_building_may_declare_prefixes_beyond_itself() {
    let rules = evaluate(
        &addr("lab"),
        "`confidential: false`\n\n## Write domains\n\n- lab\n- shared/notes\n",
    )
    .unwrap();
    assert_eq!(rules.write_domain().unwrap().prefixes().count(), 2);
}
