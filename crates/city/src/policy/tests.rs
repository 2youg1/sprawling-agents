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

/// A `write:` value that is neither spelling is refused rather than read
/// as the permissive one.
#[test]
fn a_write_reach_that_reads_as_a_typo_is_refused() {
    let addr = Address::parse("lab").unwrap();
    let err = evaluate(&addr, "`confidential: false`\n\n`write: anything`\n").unwrap_err();
    assert_eq!(err.code(), &AxCode::ConfigInvalid);
    // Absent is Everything: every building written before this line
    // existed writes what it always wrote.
    let ordinary = evaluate(&addr, "`confidential: false`\n").unwrap();
    assert_eq!(ordinary.reach(), crate::DomainReach::Everything);
}

#[test]
fn a_building_that_says_nothing_about_a_browser_has_none() {
    let rules = evaluate(&addr("lab"), "confidential: false\n").unwrap();
    assert!(
        !rules.browser(),
        "a tool nobody asked for is a tool nobody gets"
    );
    let asked = evaluate(&addr("lab"), "confidential: false\nbrowser: true\n").unwrap();
    assert!(asked.browser());
}

#[test]
fn a_confidential_building_cannot_ask_for_a_browser() {
    let err = evaluate(&addr("lab"), "confidential: true\nbrowser: true\n").unwrap_err();
    assert_eq!(err.code(), &AxCode::ConfigInvalid);
    assert!(err.recovery().contains("does not leave"));
    assert!(
        !evaluate(&addr("lab"), "confidential: true\n")
            .unwrap()
            .browser()
    );
}

#[test]
fn a_browser_line_that_reads_as_a_typo_is_refused_rather_than_guessed() {
    let err = evaluate(&addr("lab"), "confidential: false\nbrowser: yes\n").unwrap_err();
    assert_eq!(err.code(), &AxCode::ConfigInvalid);
    assert!(err.subject().contains("browser: yes"));
}

/// card-7.3: a building hands its person's desktop over only by saying
/// so. Absent the line, no — the same reading `browser:` gets, and for a
/// stronger reason: a click on somebody's desktop has no way back.
#[test]
fn a_building_that_says_nothing_about_the_desktop_has_none() {
    let silent = evaluate(&addr("lab"), "confidential: false\n").unwrap();
    assert!(
        !silent.desktop(),
        "a building that never mentioned the desktop was given one"
    );
    let asked = evaluate(&addr("lab"), "confidential: false\ndesktop: true\n").unwrap();
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
    let err = evaluate(&addr("lab"), "confidential: true\ndesktop: true\n").unwrap_err();
    assert_eq!(err.code(), &AxCode::ConfigInvalid);
    assert!(
        err.recovery().contains("desktop: true"),
        "the refusal says which line to remove: {}",
        err.recovery()
    );
}

/// A value that reads as a typo does not resolve to the permissive side,
/// which is the rule every switch in this file is held to.
#[test]
fn a_desktop_line_that_reads_as_a_typo_is_refused_rather_than_guessed() {
    let err = evaluate(&addr("lab"), "confidential: false\ndesktop: yes\n").unwrap_err();
    assert_eq!(err.code(), &AxCode::ConfigInvalid);
    assert!(err.subject().contains("desktop: yes"));
}

/// card-7.3: the desktop allowlist is a governing document, not a
/// product. It says, window by window, what this building's runs may
/// touch on somebody's machine — so it lands where no write domain
/// reaches, and a resident cannot widen its own scope.
#[test]
fn the_desktop_allowlist_lands_where_no_write_domain_reaches() {
    let dir = tempfile::tempdir().unwrap();
    let lab = addr("lab");
    let written = write_desktop_scope(dir.path(), &lab, "windows = [\"*Notepad*\"]\n").unwrap();
    assert_eq!(
        std::fs::read_to_string(&written).unwrap(),
        "windows = [\"*Notepad*\"]\n"
    );
    assert_eq!(written, desktop_scope_path(dir.path(), &lab));

    let relative = written.strip_prefix(dir.path()).unwrap();
    let slashed = relative
        .to_string_lossy()
        .replace(std::path::MAIN_SEPARATOR, "/");
    let reached = Address::parse(&slashed).unwrap();
    assert!(
        reached.is_reserved(),
        "{} is somewhere a run could write",
        relative.display()
    );
    // It sits beside the rules it pairs with, in the same subtree.
    assert_eq!(written.parent(), building_path(dir.path(), &lab).parent());
}

/// A second save replaces the first: one box, one document. And the
/// bytes are the person's own — this side parses nothing, because the
/// server that reads it is the authority on its syntax and fails closed
/// on a file it cannot read.
#[test]
fn the_allowlist_is_written_whole_and_not_parsed_here() {
    let dir = tempfile::tempdir().unwrap();
    let lab = addr("lab");
    write_desktop_scope(dir.path(), &lab, "windows = [\"a\"]\n").unwrap();
    let second = write_desktop_scope(dir.path(), &lab, "windows = [").unwrap();
    assert_eq!(std::fs::read_to_string(&second).unwrap(), "windows = [");
}
