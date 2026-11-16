// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use super::*;

#[test]
fn a_confidential_building_locks_the_model_pool_and_its_own_subtree() {
    let rules = evaluate(
        &addr("vault"),
        "confidential = true\nwrite = \"everything\"\n\
         prefixes = [\"vault/work\", \"vault/notes\"]\n",
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
        &shut("prefixes = [\"vault/work\", \"lab/shared\"]\n"),
    )
    .unwrap();
    let err = rules.write_domain().unwrap_err();
    assert!(err.to_string().contains("lab/shared"));
    assert!(err.recovery().contains("confidential = true"));
}

#[test]
fn a_building_reaches_the_domains_it_names_and_nothing_else() {
    let rules = evaluate(
        &addr("lab"),
        &ordinary("egress = [\"crates.io\", \"docs.rs\"]\n"),
    )
    .unwrap();
    let listed = rules.egress().unwrap();
    assert!(listed.admits("static.crates.io") && !listed.admits("pastebin.test"));
}

#[test]
fn a_confidential_building_that_also_lists_domains_is_a_contradiction_and_is_refused() {
    let err = evaluate(&addr("vault"), &shut("egress = [\"example.com\"]\n")).unwrap_err();
    assert!(err.recovery().contains("does not leave"));
}

#[test]
fn a_confidential_building_reaches_nothing_public() {
    let rules = evaluate(&addr("vault"), &shut("")).unwrap();
    assert_eq!(rules.egress(), Some(&EgressAllowlist::default()));
}

#[test]
fn the_checkpoint_entropy_allowlist_is_exact_validated_paths_and_empty_by_default() {
    let empty = evaluate(&addr("lab"), &ordinary("")).unwrap();
    assert!(!empty.checkpoint_entropy_allow().allows("lab/SPEC.md"));

    let rules = evaluate(
        &addr("lab"),
        &ordinary("checkpoint_entropy_allow = [\"lab/SPEC.md\", \"shared/NOTES.md\"]\n"),
    )
    .unwrap();
    assert!(rules.checkpoint_entropy_allow().allows("lab/SPEC.md"));
    assert!(rules.checkpoint_entropy_allow().allows("shared/NOTES.md"));
    assert!(!rules.checkpoint_entropy_allow().allows("lab"));
    assert!(!rules.checkpoint_entropy_allow().allows("lab/other.md"));

    let refused = evaluate(
        &addr("lab"),
        &ordinary("checkpoint_entropy_allow = [\"../outside.md\"]\n"),
    )
    .unwrap_err();
    assert_eq!(refused.code(), &AxCode::InvalidArgs);
}

#[test]
fn an_ordinary_building_may_declare_prefixes_beyond_itself() {
    let rules = evaluate(
        &addr("lab"),
        &ordinary("prefixes = [\"lab\", \"shared/notes\"]\n"),
    )
    .unwrap();
    assert_eq!(rules.write_domain().unwrap().prefixes().count(), 2);
}

#[test]
fn the_hall_template_writes_documents_and_never_the_plan() {
    let dir = tempfile::tempdir().unwrap();
    let hall = Address::parse(kernel::consts_policy::HALL_BUILDING).unwrap();
    crate::building::create(dir.path(), &hall, crate::BuildingTemplate::Hall).unwrap();
    let rules = load(dir.path(), &hall).unwrap();
    assert!(
        !rules.policy().confidential,
        "the hall reads every building"
    );
    assert_eq!(rules.reach(), crate::DomainReach::Documents);

    let domain = rules.write_domain().unwrap();
    assert!(matches!(
        domain.admits(&Address::parse("hall/Memo.md").unwrap()),
        kernel::DomainVerdict::Within
    ));
    assert!(matches!(
        domain.admits(&Address::parse("hall/build.rs").unwrap()),
        kernel::DomainVerdict::NotWritable {
            reason: kernel::DocumentReason::NotMarkdown
        }
    ));
    assert!(matches!(
        domain.admits(&Address::parse("hall/Roadmap.md").unwrap()),
        kernel::DomainVerdict::NotWritable {
            reason: kernel::DocumentReason::ThePlan
        }
    ));
}

#[test]
fn a_write_reach_that_reads_as_a_typo_is_refused_and_so_is_its_absence() {
    let lab = Address::parse("lab").unwrap();
    let typo = evaluate(&lab, "confidential = false\nwrite = \"anything\"\n").unwrap_err();
    assert_eq!(typo.code(), &AxCode::ConfigInvalid);
    let absent = evaluate(&lab, "confidential = false\n").unwrap_err();
    assert_eq!(absent.code(), &AxCode::ConfigInvalid);
    assert!(absent.subject().contains("write"), "{}", absent.subject());
}

#[test]
fn a_building_that_says_nothing_about_a_browser_has_none() {
    let rules = evaluate(&addr("lab"), &ordinary("")).unwrap();
    assert!(
        !rules.browser(),
        "a tool nobody asked for is a tool nobody gets"
    );
    let asked = evaluate(&addr("lab"), &ordinary("browser = true\n")).unwrap();
    assert!(asked.browser());
}

#[test]
fn a_confidential_building_cannot_ask_for_a_browser() {
    let err = evaluate(&addr("lab"), &shut("browser = true\n")).unwrap_err();
    assert_eq!(err.code(), &AxCode::ConfigInvalid);
    assert!(err.recovery().contains("does not leave"));
    assert!(!evaluate(&addr("lab"), &shut("")).unwrap().browser());
}

#[test]
fn a_browser_line_that_reads_as_a_typo_is_refused_rather_than_guessed() {
    let err = evaluate(&addr("lab"), &ordinary("browser = \"yes\"\n")).unwrap_err();
    assert_eq!(err.code(), &AxCode::ConfigInvalid);
    assert!(err.subject().contains("browser"), "{}", err.subject());
}

#[test]
fn a_building_that_says_nothing_about_the_desktop_has_none() {
    let silent = evaluate(&addr("lab"), &ordinary("")).unwrap();
    assert!(
        !silent.desktop(),
        "a building that never mentioned the desktop was given one"
    );
    let asked = evaluate(&addr("lab"), &ordinary("desktop = true\n")).unwrap();
    assert!(asked.desktop());

    assert!(!asked.browser());
}

#[test]
fn a_confidential_building_cannot_ask_for_the_desktop() {
    let err = evaluate(&addr("lab"), &shut("desktop = true\n")).unwrap_err();
    assert_eq!(err.code(), &AxCode::ConfigInvalid);
    assert!(
        err.recovery().contains("desktop = true"),
        "the refusal says which line to remove: {}",
        err.recovery()
    );
}

#[test]
fn a_desktop_line_that_reads_as_a_typo_is_refused_rather_than_guessed() {
    let err = evaluate(&addr("lab"), &ordinary("desktop = \"yes\"\n")).unwrap_err();
    assert_eq!(err.code(), &AxCode::ConfigInvalid);
    assert!(err.subject().contains("desktop"), "{}", err.subject());
}

#[test]
fn the_person_browser_is_switched_on_by_the_address_they_declared() {
    let silent = evaluate(&addr("lab"), &ordinary("")).unwrap();
    assert!(silent.usersbrowser().is_none(), "absent means no");
    let waiting = evaluate(&addr("lab"), &ordinary("usersbrowser = true\n")).unwrap();
    assert!(matches!(waiting.usersbrowser(), Some(UserBrowser::Waiting)));
    let asked = evaluate(
        &addr("lab"),
        &ordinary("usersbrowser = \"ws://127.0.0.1:9222/session\"\n"),
    )
    .unwrap();
    let Some(UserBrowser::At(endpoint)) = asked.usersbrowser() else {
        panic!("an address switches the tool on");
    };
    assert_eq!(endpoint.url(), "ws://127.0.0.1:9222/session");
    assert_eq!(endpoint.host(), "127.0.0.1");
    let off = evaluate(&addr("lab"), &ordinary("usersbrowser = false\n")).unwrap();
    assert!(off.usersbrowser().is_none());
}

#[test]
fn the_person_browser_and_the_citys_own_browser_are_two_settings() {
    let both = evaluate(
        &addr("lab"),
        &ordinary("browser = true\nusersbrowser = \"ws://127.0.0.1:9222/session\"\n"),
    )
    .unwrap();
    assert!(both.browser());
    assert!(matches!(both.usersbrowser(), Some(UserBrowser::At(_))));
    let only_person = evaluate(
        &addr("lab"),
        &ordinary("usersbrowser = \"ws://127.0.0.1:9222/session\"\n"),
    )
    .unwrap();
    assert!(!only_person.browser());
}

#[test]
fn a_confidential_building_cannot_ask_for_the_persons_browser() {
    let err = evaluate(&addr("lab"), &shut("usersbrowser = true\n")).unwrap_err();
    assert_eq!(err.code(), &AxCode::ConfigInvalid);
    assert!(err.recovery().contains("usersbrowser"));
    let err = evaluate(
        &addr("lab"),
        &shut("usersbrowser = \"ws://127.0.0.1:9222/session\"\n"),
    )
    .unwrap_err();
    assert_eq!(err.code(), &AxCode::ConfigInvalid);
}

#[test]
fn a_person_browser_value_that_is_neither_a_switch_nor_an_address_is_refused() {
    for bad in ["yes", "ftp://127.0.0.1:9222", "ws://", "127.0.0.1:9222"] {
        let err = evaluate(
            &addr("lab"),
            &ordinary(&format!("usersbrowser = \"{bad}\"\n")),
        )
        .unwrap_err();
        assert_eq!(err.code(), &AxCode::ConfigInvalid, "{bad}");
    }
}

#[test]
fn a_context_lock_is_read_from_rules_and_zero_is_refused() {
    let rules = evaluate(&addr("lab"), &ordinary("context_lock_tokens = 262144\n")).unwrap();
    assert_eq!(
        rules.context_lock().map(kernel::ContextLock::tokens),
        Some(kernel::Tokens::new(262_144))
    );
    let err = evaluate(&addr("lab"), &ordinary("context_lock_tokens = 0\n")).unwrap_err();
    assert_eq!(err.code(), &AxCode::InvalidArgs);
}
