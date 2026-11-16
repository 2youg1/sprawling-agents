// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The three outcomes `crates/city/spec/Format.lean` §8-42 names, each
//! met through the door production uses.

use super::*;

const UNVERSIONED: &str = "[model]\neffort = \"high\"\n";

fn backups(dir: &tempfile::TempDir) -> PathBuf {
    dir.path().join("backups")
}

/// A file that states no format is at the current one, and bringing it
/// forward writes nothing: the previous release must still read it.
#[test]
fn a_file_nobody_versioned_is_left_byte_for_byte() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("CONFIG.toml");
    std::fs::write(&path, UNVERSIONED).unwrap();
    let before = std::fs::metadata(&path).unwrap().modified().unwrap();

    for file in [
        FormatFile::PersonConfig,
        FormatFile::LayerConfig,
        FormatFile::Rules,
    ] {
        assert_eq!(
            bring_forward(&path, file, &backups(&dir)).unwrap(),
            Forwarded::Current
        );
    }
    assert_eq!(std::fs::read_to_string(&path).unwrap(), UNVERSIONED);
    assert_eq!(
        std::fs::metadata(&path).unwrap().modified().unwrap(),
        before
    );
    assert!(!backups(&dir).exists(), "nothing was backed up");
}

/// A file from a newer build is refused whole, the refusal names the
/// upgrade, and the file is not touched.
#[test]
fn a_file_from_a_newer_build_is_refused_and_names_the_upgrade() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("RULES.toml");
    let newer = format!("format = 2\n{UNVERSIONED}");
    std::fs::write(&path, &newer).unwrap();

    let refused = bring_forward(&path, FormatFile::Rules, &backups(&dir)).unwrap_err();
    assert_eq!(refused.code(), &AxCode::ConfigInvalid);
    assert!(refused.subject().contains("newer sprawling"), "{refused}");
    assert!(
        refused.recovery().contains("upgrade sprawling"),
        "{refused}"
    );
    assert_eq!(std::fs::read_to_string(&path).unwrap(), newer);

    let refused = admit_format(FormatFile::LayerConfig, &newer).unwrap_err();
    assert!(
        refused.recovery().contains("upgrade sprawling"),
        "{refused}"
    );
}

#[test]
fn a_format_that_is_not_a_positive_integer_is_refused() {
    for raw in [0, -1, i64::from(u32::MAX) + 1] {
        assert_eq!(
            admit_format(FormatFile::Rules, &format!("format = {raw}\n"))
                .unwrap_err()
                .code(),
            &AxCode::ConfigInvalid
        );
    }
    assert_eq!(
        FormatFile::LayerConfig
            .stated("format = \"two\"\n")
            .unwrap_err()
            .code(),
        &AxCode::ConfigInvalid
    );
    assert!(admit_format(FormatFile::Rules, UNVERSIONED).is_ok());
    assert!(
        admit_format(FormatFile::Rules, "nothing declared\n").is_ok(),
        "text that is not TOML is the reader's to refuse, in its own words"
    );
    assert!(admit_format(FormatFile::Rules, "format = 1\n").is_ok());
}

#[test]
fn an_absent_file_is_answered_absent() {
    let dir = tempfile::tempdir().unwrap();
    assert_eq!(
        bring_forward(
            &dir.path().join("CONFIG.toml"),
            FormatFile::LayerConfig,
            &backups(&dir)
        )
        .unwrap(),
        Forwarded::Absent
    );
}

/// A backup already standing with the same bytes is the same backup; a
/// different one is never overwritten.
#[test]
fn a_backup_is_written_once_and_never_replaced_by_other_bytes() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("CONFIG.toml");
    let first = keep(&path, Format::FIRST, b"one", &backups(&dir)).unwrap();
    assert!(first.starts_with(backups(&dir)));
    assert!(first.to_string_lossy().ends_with("-CONFIG.toml.bak-1"));
    assert_eq!(
        keep(&path, Format::FIRST, b"one", &backups(&dir)).unwrap(),
        first
    );
    assert_eq!(
        keep(&path, Format::FIRST, b"two", &backups(&dir))
            .unwrap_err()
            .code(),
        &AxCode::StorageFatal
    );
    assert_eq!(std::fs::read(&first).unwrap(), b"one");
}

fn to_two(text: &str) -> Result<String, AxError> {
    Ok(format!("{FORMAT_KEY} = 2\n{text}"))
}

/// An older file climbs every rung, its original bytes land in the
/// backup directory first, and a second climb finds it current.
#[test]
fn an_older_file_climbs_and_keeps_its_original() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("CONFIG.toml");
    std::fs::write(&path, UNVERSIONED).unwrap();
    let two = Format::FIRST.next().unwrap();
    let rungs = [Rung {
        file: FormatFile::LayerConfig,
        from: Format::FIRST,
        rewrite: to_two,
    }];
    let ladder = Ladder {
        to: two,
        rungs: &rungs,
    };

    let moved = climb(&path, FormatFile::LayerConfig, &ladder, &backups(&dir)).unwrap();
    let Forwarded::Moved { from, to, backup } = moved else {
        panic!("expected a move, got {moved:?}");
    };
    assert_eq!((from, to), (Format::FIRST, two));
    assert_eq!(std::fs::read_to_string(&backup).unwrap(), UNVERSIONED);
    assert_eq!(
        FormatFile::LayerConfig
            .stated(&std::fs::read_to_string(&path).unwrap())
            .unwrap(),
        Some(two)
    );
    assert_eq!(
        climb(&path, FormatFile::LayerConfig, &ladder, &backups(&dir)).unwrap(),
        Forwarded::Current
    );
    assert_eq!(
        climb(&path, FormatFile::Rules, &ladder, &backups(&dir)).unwrap(),
        Forwarded::Current,
        "the file already states format 2"
    );
}

/// Opening a city brings every versioned file forward and answers only
/// the files it moved or refused; a refused file holds no other back.
#[test]
fn a_city_brings_each_file_forward_and_a_refused_one_holds_none_back() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("city");
    let lab = kernel::Address::parse("lab").unwrap();
    crate::building::create(&root, &lab, crate::building::BuildingTemplate::Minimal).unwrap();
    let city_config = kernel::layout::CityLayout::new(&root).city_config();
    write_making_parents(&city_config, UNVERSIONED);
    let lab_config = crate::config_layers::Layer::Building
        .file(&root, &lab)
        .unwrap();
    std::fs::write(&lab_config, "format = 9\n").unwrap();
    let rules = crate::policy::rules_path(&root, &lab);
    let two = Format::FIRST.next().unwrap();
    let rungs = [FormatFile::LayerConfig, FormatFile::Rules].map(|file| Rung {
        file,
        from: Format::FIRST,
        rewrite: to_two,
    });
    let ladder = |_| Ladder {
        to: two,
        rungs: &rungs,
    };

    let answered: Vec<(PathBuf, Option<Format>, Option<AxCode>)> =
        climb_city(&root, ladder, &backups(&dir))
            .unwrap()
            .into_iter()
            .map(|forwarding| match forwarding.outcome {
                Ok(Forwarded::Moved { to, .. }) => (forwarding.path, Some(to), None),
                Ok(other) => panic!("{other:?} needs no line"),
                Err(err) => (forwarding.path, None, Some(*err.code())),
            })
            .collect();
    assert_eq!(
        answered,
        vec![
            (city_config.clone(), Some(two), None),
            (lab_config.clone(), None, Some(AxCode::ConfigInvalid)),
            (rules.clone(), Some(two), None),
        ]
    );
    let again: Vec<PathBuf> = climb_city(&root, ladder, &backups(&dir))
        .unwrap()
        .into_iter()
        .map(|forwarding| forwarding.path)
        .collect();
    assert_eq!(
        again,
        vec![lab_config],
        "only the refused file is answered again"
    );
}

/// This build's own ladder is empty, so opening a city writes nothing.
#[test]
fn opening_a_city_on_this_build_writes_nothing() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("city");
    let lab = kernel::Address::parse("lab").unwrap();
    crate::building::create(&root, &lab, crate::building::BuildingTemplate::Minimal).unwrap();
    let rules = crate::policy::rules_path(&root, &lab);
    let before = std::fs::read(&rules).unwrap();

    assert!(
        bring_city_forward(&root, &backups(&dir))
            .unwrap()
            .is_empty()
    );
    assert_eq!(std::fs::read(&rules).unwrap(), before);
    assert!(!backups(&dir).exists());
}

fn write_making_parents(path: &Path, text: &str) {
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, text).unwrap();
}
