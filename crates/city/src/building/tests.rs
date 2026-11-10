// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use super::template::NAME_PLACEHOLDER;
use super::*;
use crate::policy::{self, ModelPool};

fn addr(raw: &str) -> Address {
    Address::parse(raw).unwrap()
}

#[test]
fn a_created_building_is_read_back_by_the_citys_own_parser() {
    let dir = tempfile::tempdir().unwrap();
    let building = create(dir.path(), &addr("lab"), BuildingTemplate::Minimal).unwrap();
    assert_eq!(building.addr(), &addr("lab"));

    let rules = policy::load(dir.path(), &addr("lab")).unwrap();
    assert!(!rules.policy().confidential);
    assert_eq!(rules.model_pool(), ModelPool::Any);
    // With nothing declared, a new building may write itself and no more.
    assert_eq!(rules.write_domain().unwrap().prefixes().count(), 1);

    let text =
        std::fs::read_to_string(crate::policy::rules_path(dir.path(), &addr("lab"))).unwrap();
    assert!(
        text.contains("lab"),
        "a building's own rules name the building"
    );
    assert!(!text.contains(NAME_PLACEHOLDER));
    assert!(
        building
            .root(dir.path())
            .join(crate::spine_files::ROADMAP_FILE)
            .exists(),
        "a building exists with its plan, not only with its rules"
    );
}

#[test]
fn the_confidential_template_produces_a_building_the_city_treats_as_confidential() {
    let dir = tempfile::tempdir().unwrap();
    create(dir.path(), &addr("vault"), BuildingTemplate::Confidential).unwrap();

    let rules = policy::load(dir.path(), &addr("vault")).unwrap();
    assert!(rules.policy().confidential);
    assert_eq!(rules.model_pool(), ModelPool::LocalOnly);
    assert!(
        rules
            .egress()
            .is_some_and(kernel::EgressAllowlist::is_empty)
    );
}

/// The template writes no `egress` key, so a new building is held to no
/// domain list until the person writes one; an empty list written by
/// hand reaches no public host (`crates/city/spec/Policy.lean` D27).
#[test]
fn a_new_building_has_no_egress_list_and_an_empty_one_reaches_nothing() {
    let dir = tempfile::tempdir().unwrap();
    create(dir.path(), &addr("lab"), BuildingTemplate::Minimal).unwrap();
    let laid_out = policy::load(dir.path(), &addr("lab")).unwrap();
    let written = "confidential = false
write = \"everything\"
egress = []
";
    let emptied = policy::evaluate(&addr("lab"), written).unwrap();
    assert_eq!(
        (laid_out.egress(), emptied.egress()),
        (None, Some(&kernel::EgressAllowlist::default()))
    );
}

#[test]
fn a_second_birth_is_refused_and_the_first_rules_survive() {
    let dir = tempfile::tempdir().unwrap();
    create(dir.path(), &addr("vault"), BuildingTemplate::Confidential).unwrap();

    let err = create(dir.path(), &addr("vault"), BuildingTemplate::Minimal).unwrap_err();
    assert_eq!(err.code(), &AxCode::InvalidArgs);
    assert!(err.recovery().contains("already has rules"));
    assert!(
        policy::load(dir.path(), &addr("vault"))
            .unwrap()
            .policy()
            .confidential,
        "the refusal left the confidential rules in place"
    );
}

#[test]
fn a_room_address_is_refused_and_the_refusal_names_the_building_to_create() {
    let dir = tempfile::tempdir().unwrap();
    let err = create(dir.path(), &addr("lab/room1"), BuildingTemplate::Minimal).unwrap_err();
    assert_eq!(err.code(), &AxCode::InvalidArgs);
    assert!(err.recovery().contains("`lab`"));
    assert!(!dir.path().join("lab").join("room1").exists());
    assert!(
        !dir.path().join("lab").exists(),
        "a refused building leaves nothing behind"
    );
}

#[test]
fn the_reserved_subtree_belongs_to_no_building() {
    let dir = tempfile::tempdir().unwrap();
    let err = Building::of(&addr(".sprawling/cas/ab")).unwrap_err();
    assert!(err.recovery().contains("reserved subtree"));

    let err = create(dir.path(), &addr(".sprawling"), BuildingTemplate::Minimal).unwrap_err();
    assert_eq!(err.code(), &AxCode::InvalidArgs);
    assert!(!dir.path().join(".sprawling").join(RULES_FILE).exists());
}

#[test]
fn a_building_holds_its_own_rooms_and_no_others() {
    let lab = Building::of(&addr("lab/room1")).unwrap();
    assert_eq!(lab.addr(), &addr("lab"));
    assert!(lab.holds(&addr("lab")));
    assert!(lab.holds(&addr("lab/room1/notes.md")));
    assert!(!lab.holds(&addr("laboratory/room1")));
    assert!(!lab.holds(&addr("vault")));
}

#[test]
fn an_unknown_template_is_refused_with_the_set_this_version_lays_out() {
    let err = BuildingTemplate::parse("workshop").unwrap_err();
    assert_eq!(err.code(), &AxCode::InvalidArgs);
    assert!(err.recovery().contains("minimal"));
    assert!(err.recovery().contains("confidential"));
    assert_eq!(
        BuildingTemplate::parse(BuildingTemplate::Confidential.name()).unwrap(),
        BuildingTemplate::Confidential
    );
}

#[test]
fn the_record_carries_the_address_and_the_template_and_nothing_else() {
    let building = Building::of(&addr("lab")).unwrap();
    let payload = created_payload(&building, BuildingTemplate::Confidential).unwrap();
    let map = payload.as_map();
    assert_eq!(map.len(), 2);
    assert_eq!(map.get("addr").and_then(|v| v.as_str()), Some("lab"));
    assert_eq!(
        map.get("template").and_then(|v| v.as_str()),
        Some("confidential")
    );
}

#[test]
fn adopting_an_existing_directory_keeps_every_file_it_found() {
    let city = tempfile::tempdir().unwrap();
    let repo = city.path().join("imported");
    std::fs::create_dir_all(repo.join("src")).unwrap();
    std::fs::write(repo.join("src").join("main.rs"), "fn main() {}\n").unwrap();
    std::fs::write(repo.join("Roadmap.md"), "# my own roadmap\n").unwrap();

    let building = adopt(city.path(), &addr("imported")).unwrap();
    assert_eq!(building.addr().as_str(), "imported");
    // What was there is untouched; what was missing is laid.
    assert_eq!(
        std::fs::read_to_string(repo.join("src").join("main.rs")).unwrap(),
        "fn main() {}\n"
    );
    assert_eq!(
        std::fs::read_to_string(repo.join("Roadmap.md")).unwrap(),
        "# my own roadmap\n",
        "an adopted roadmap is the owner's, not the template's"
    );
    // The rules land in the building's reserved subtree; the spine
    // files a person reads and writes stay where they were.
    assert!(
        crate::policy::rules_path(city.path(), &addr("imported")).is_file(),
        "an adopted directory has no rules of its own"
    );
    assert!(
        !repo.join(crate::policy::RULES_FILE).exists(),
        "the rules belong in the reserved subtree, not in the project root"
    );
    assert!(repo.join("Memo.md").is_file());
    // Adopting twice refuses: it is already a building.
    assert!(adopt(city.path(), &addr("imported")).is_err());
    // Adopting nothing refuses toward create.
    let err = adopt(city.path(), &addr("ghost")).unwrap_err();
    assert!(err.recovery().contains("create"), "{err}");
    // The record says it was an adoption.
    let payload = adopted_payload(&building, BuildingTemplate::Minimal).unwrap();
    assert_eq!(
        payload.as_map().get("adopted"),
        Some(&serde_json::Value::Bool(true))
    );
}

/// The asymmetry: what a building promises is in
/// history, what one session was thinking is not.
#[test]
fn a_raised_building_ignores_its_roadmap_and_tracks_its_spec() {
    let dir = tempfile::tempdir().unwrap();
    let building = create(dir.path(), &addr("lab"), BuildingTemplate::Minimal).unwrap();
    let root = building.root(dir.path());
    assert!(
        root.join(crate::spine_files::SPEC_FILE).is_file(),
        "a raised building starts with the SPEC form it is built from"
    );
    let ignored = std::fs::read_to_string(root.join(crate::gitignore::GITIGNORE_FILE)).unwrap();
    for line in [
        crate::spine_files::ROADMAP_FILE,
        "Memo.md",
        crate::spine_files::HANDOFF_FILE,
    ] {
        assert!(
            ignored.lines().any(|row| row.trim() == line),
            "{line} is one session's thinking and does not belong in history"
        );
    }
    for kept in [
        "!SPEC.md",
        "!/.sprawling/RULES.toml",
        "!/.sprawling/CONFIG.toml",
    ] {
        assert!(
            ignored.lines().any(|row| row.trim() == kept),
            "{kept} is what the building promises and must survive an outer ignore rule"
        );
    }
    assert!(
        !ignored.lines().any(|row| row.trim() == "!.sprawling/"),
        "one negation over the whole reserved subtree would commit the ledger and the vault references with it"
    );
}

/// Places `target` at `link` as a link, on a machine that has no symlink
/// privilege: Windows gets a junction, which `symlink_metadata` reports
/// through the same name-surrogate predicate a symlink is reported by.
#[cfg(windows)]
#[expect(clippy::disallowed_methods, reason = "test fixture (child D4)")]
fn make_link(link: &Path, target: &Path) {
    let made = std::process::Command::new("cmd")
        .args(["/c", "mklink", "/J"])
        .arg(link)
        .arg(target)
        .status()
        .unwrap();
    assert!(made.success(), "the fixture could not make a link");
}

#[cfg(unix)]
fn make_link(link: &Path, target: &Path) {
    std::os::unix::fs::symlink(target, link).unwrap();
}

/// A host `.git` store is skipped on import: it is not the workspace's
/// files, and copying it would block the city on an ordinary project.
/// The skip applies at every depth, because a vendored or nested
/// repository keeps its store the same way.
#[test]
fn importing_skips_a_git_directory() {
    let city = tempfile::tempdir().unwrap();
    let source = tempfile::tempdir().unwrap();
    std::fs::write(source.path().join("KEEP.md"), "keep\n").unwrap();
    let git = source.path().join(".git");
    std::fs::create_dir_all(git.join("objects")).unwrap();
    std::fs::write(git.join("HEAD"), "ref: refs/heads/main\n").unwrap();
    std::fs::write(git.join("objects/pack"), "heavy\n").unwrap();
    let nested = source.path().join("vendor/pkg/.git");
    std::fs::create_dir_all(&nested).unwrap();
    std::fs::write(nested.join("config"), "nested\n").unwrap();
    std::fs::write(source.path().join("vendor/pkg/keep.txt"), "here\n").unwrap();

    raise(
        city.path(),
        Raising {
            addr: &addr("lab"),
            template: BuildingTemplate::Minimal,
            rules: RulesText::Template,
            source: Some(source.path()),
        },
    )
    .unwrap();

    let root = city.path().join("lab");
    assert_eq!(
        std::fs::read_to_string(root.join("KEEP.md")).unwrap(),
        "keep\n"
    );
    assert!(
        !root.join(".git").exists(),
        "the host .git store must not land in the building"
    );
    assert!(
        !root.join("vendor/pkg/.git").exists(),
        "a nested .git store is skipped at its own depth"
    );
    assert_eq!(
        std::fs::read_to_string(root.join("vendor/pkg/keep.txt")).unwrap(),
        "here\n"
    );
    assert!(
        std::fs::read_to_string(source.path().join(".git/HEAD"))
            .unwrap()
            .contains("refs/heads"),
        "the source .git stays on the host"
    );
}

/// A file named `.git` is the pointer a linked worktree or a submodule
/// keeps, not the version-control store: it comes in with the rest of
/// the folder.
#[test]
fn importing_copies_a_file_named_git() {
    let city = tempfile::tempdir().unwrap();
    let source = tempfile::tempdir().unwrap();
    std::fs::write(
        source.path().join(".git"),
        "gitdir: /elsewhere/.git/worktrees/lab\n",
    )
    .unwrap();

    raise(
        city.path(),
        Raising {
            addr: &addr("lab"),
            template: BuildingTemplate::Minimal,
            rules: RulesText::Template,
            source: Some(source.path()),
        },
    )
    .unwrap();

    assert_eq!(
        std::fs::read_to_string(city.path().join("lab").join(".git")).unwrap(),
        "gitdir: /elsewhere/.git/worktrees/lab\n"
    );
}

/// A link named `.git` is a link, not the store's directory: the skip
/// does not take it past the rule that refuses every link.
#[test]
fn importing_refuses_a_link_named_git() {
    let city = tempfile::tempdir().unwrap();
    let source = tempfile::tempdir().unwrap();
    let elsewhere = tempfile::tempdir().unwrap();
    std::fs::write(elsewhere.path().join("HEAD"), "ref: refs/heads/main\n").unwrap();
    make_link(&source.path().join(".git"), elsewhere.path());

    let err = raise(
        city.path(),
        Raising {
            addr: &addr("lab"),
            template: BuildingTemplate::Minimal,
            rules: RulesText::Template,
            source: Some(source.path()),
        },
    )
    .unwrap_err();

    assert_eq!(err.code(), &AxCode::InvalidArgs);
    assert!(err.recovery().contains("links are not copied"), "{err}");
    assert!(
        !city.path().join("lab").exists(),
        "a refused import lays nothing at the target"
    );
}

/// A folder that is already a building keeps its rules. Importing it
/// would have to overwrite them, so the copy never starts and the empty
/// target is not what the refusal names.
#[test]
fn importing_a_folder_that_already_has_rules_copies_nothing() {
    let city = tempfile::tempdir().unwrap();
    let source = tempfile::tempdir().unwrap();
    let rules = source.path().join(kernel::RESERVED_PREFIX).join(RULES_FILE);
    std::fs::create_dir_all(rules.parent().unwrap()).unwrap();
    std::fs::write(&rules, "confidential = false\n").unwrap();
    std::fs::write(source.path().join("KEEP.md"), "keep\n").unwrap();

    let err = raise(
        city.path(),
        Raising {
            addr: &addr("lab"),
            template: BuildingTemplate::Minimal,
            rules: RulesText::Template,
            source: Some(source.path()),
        },
    )
    .unwrap_err();

    assert_eq!(err.code(), &AxCode::InvalidArgs);
    // `canonicalize` on Windows spells the same directory with a `\\?\`
    // prefix and may expand a short temp name, so the text tempfile
    // handed back is not a substring of the path the refusal names.
    assert_eq!(
        std::fs::canonicalize(std::path::Path::new(err.subject())).unwrap(),
        std::fs::canonicalize(&rules).unwrap(),
        "the refusal names the source rules file, not the empty target: {err}"
    );
    assert!(
        !err.recovery().contains("nobody occupies"),
        "an empty target is not the problem: {err}"
    );
    assert!(!city.path().join("lab").exists());
    assert!(!city.path().join(".lab.importing").exists());
    assert_eq!(
        std::fs::read_to_string(source.path().join("KEEP.md")).unwrap(),
        "keep\n"
    );
    assert_eq!(
        std::fs::read_to_string(&rules).unwrap(),
        "confidential = false\n"
    );
}

/// Adoption draws the same line over a directory that already had one
/// of its own, and takes nothing away from it.
#[test]
fn an_adopted_directory_keeps_what_it_already_ignored() {
    let city = tempfile::tempdir().unwrap();
    let repo = city.path().join("imported");
    std::fs::create_dir_all(&repo).unwrap();
    std::fs::write(repo.join(".gitignore"), "target/\n*.log\n").unwrap();

    adopt(city.path(), &addr("imported")).unwrap();
    let ignored = std::fs::read_to_string(repo.join(".gitignore")).unwrap();
    for kept in ["target/", "*.log"] {
        assert!(
            ignored.lines().any(|row| row.trim() == kept),
            "adoption overwrote what the directory already ignored"
        );
    }
    assert!(ignored.lines().any(|row| row.trim() == "Roadmap.md"));
    assert!(ignored.lines().any(|row| row.trim() == "!SPEC.md"));

    // Placing it twice appends nothing: the block is already there.
    let again = crate::gitignore::place(&repo)
        .map(|()| std::fs::read_to_string(repo.join(".gitignore")).unwrap_or_default());
    assert_eq!(again.unwrap(), ignored, "the block was appended twice");
}
