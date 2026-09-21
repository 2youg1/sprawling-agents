// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use super::*;

fn addr(raw: &str) -> Address {
    Address::parse(raw).unwrap()
}

/// A rules file that declares the two required answers and whatever
/// else the test is about, so each test below states one thing.
fn ordinary(rest: &str) -> String {
    format!("confidential = false\nwrite = \"everything\"\n{rest}")
}

/// The same, for a building whose data does not leave.
fn shut(rest: &str) -> String {
    format!("confidential = true\nwrite = \"everything\"\n{rest}")
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
    let file = rules_path(dir.path(), &lab);
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

/// Rules held in a document this version does not read must not come
/// back silently defaulted: `load` treats an absent file as an ordinary
/// building, so a confidential one would quietly stop being
/// confidential. Two documents can be holding them — the Markdown this
/// format replaced, at the building root the layout moved away from or
/// in the reserved subtree — and both get the same refusal.
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

/// A key this version does not know is refused rather than passed over.
/// While the rules were prose the reader skipped what it did not
/// recognise, so a misspelled setting was a permission that silently
/// never arrived.
#[test]
fn a_key_this_version_does_not_know_is_refused_rather_than_skipped() {
    let err = evaluate(&addr("lab"), &ordinary("desktopp = true\n")).unwrap_err();
    assert_eq!(err.code(), &AxCode::ConfigInvalid);
    assert!(err.subject().contains("desktopp"), "{}", err.subject());
}

/// The prose keys are part of the schema, so a person who misspells one
/// is told rather than left writing a paragraph nothing keeps.
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

/// The blank form the city lays a building out with is a file this
/// reader accepts, and the confidential variant differs from it by the
/// one value. Read back rather than eyeballed, for the reason the
/// template's own refusal gives.
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
    assert!(rules.egress().admits("static.crates.io"));
    assert!(!rules.egress().admits("pastebin.test"));
}

#[test]
fn a_confidential_building_that_also_lists_domains_is_a_contradiction_and_is_refused() {
    let err = evaluate(&addr("vault"), &shut("egress = [\"example.com\"]\n")).unwrap_err();
    assert!(err.recovery().contains("does not leave"));
}

#[test]
fn a_confidential_building_reaches_nothing_public() {
    let rules = evaluate(&addr("vault"), &shut("")).unwrap();
    assert!(rules.egress().is_empty());
    assert!(!rules.egress().admits("example.com"));
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

/// City Hall's fixed rules give its residents a documents domain: they
/// write Markdown, and the plan file has one editing entrance that is
/// not `edit`.
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

/// A `write` value that is neither spelling is refused rather than read
/// as the permissive one — and so is no value at all. While the rules
/// were prose, an absent `write:` line resolved to `Everything`, so a
/// person who never met the setting got the widest one.
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

/// A building hands its person's desktop over only by saying
/// so. Absent the line, no — the same reading `browser:` gets, and for a
/// stronger reason: a click on somebody's desktop has no way back.
#[test]
fn a_building_that_says_nothing_about_the_desktop_has_none() {
    let silent = evaluate(&addr("lab"), &ordinary("")).unwrap();
    assert!(
        !silent.desktop(),
        "a building that never mentioned the desktop was given one"
    );
    let asked = evaluate(&addr("lab"), &ordinary("desktop = true\n")).unwrap();
    assert!(asked.desktop());
    // The two switches are independent: one is a browser, the other is
    // this machine.
    assert!(!asked.browser());
}

/// A confidential building never gets the desktop. What is on a desktop
/// belongs to whoever is sitting at it, not to this building, so
/// "data enters and does not leave" and "this building may photograph
/// this machine" cannot both be true.
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

/// A value that reads as a typo does not resolve to the permissive side,
/// which is the rule every switch in this file is held to.
#[test]
fn a_desktop_line_that_reads_as_a_typo_is_refused_rather_than_guessed() {
    let err = evaluate(&addr("lab"), &ordinary("desktop = \"yes\"\n")).unwrap_err();
    assert_eq!(err.code(), &AxCode::ConfigInvalid);
    assert!(err.subject().contains("desktop"), "{}", err.subject());
}

/// The person's browser is switched on by the address itself: a person
/// who wrote the line has answered both whether and where, and two lines
/// would be two answers that can disagree.
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

/// The key is its own: `usersbrowser` is not `browser` with a prefix,
/// so the two are two settings rather than one read twice.
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

/// Attaching to the person's browser reads every login in it, so a
/// confidential building cannot ask: the per-building isolation these
/// rules keep is exactly what the attachment dissolves.
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
