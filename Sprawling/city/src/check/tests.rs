// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use super::*;
use crate::building::{BuildingTemplate, create};
use kernel::layout::CityLayout;

fn write(path: &Path, text: &str) {
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, text).unwrap();
}

fn at(line: usize, column: usize) -> Option<Position> {
    Some(Position { line, column })
}

#[test]
fn a_city_whose_files_all_parse_has_no_findings() {
    let dir = tempfile::tempdir().unwrap();
    let lab = Address::parse("lab").unwrap();
    create(dir.path(), &lab, BuildingTemplate::Minimal).unwrap();

    let report = check(dir.path()).unwrap();

    assert_eq!(report.findings, Vec::new());
    assert!(report.read >= 1, "the building's RULES.toml was read");
}

/// One bad sample per kind of file; each points at the key its parser
/// stops on, so the position a person is sent to is the one they wrote.
#[test]
fn each_kind_of_file_is_refused_at_the_line_and_column_of_its_error() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    let lab = Address::parse("lab").unwrap();
    create(root, &lab, BuildingTemplate::Minimal).unwrap();
    let rules = rules_path(root, &lab);
    let written = std::fs::read_to_string(&rules).unwrap();
    write(&rules, &format!("\n   stray = 1\n{written}"));
    let city_config = CityLayout::new(root).city_config();
    write(&city_config, "# the city\n  wrong = true\n");
    let lab_config = Layer::Building.file(root, &lab).unwrap();
    write(&lab_config, "\n\n[sandbox]\nmounts = []\nwat = 1\n");
    write(&schedule_path(root), "[[job]]\nname = \"a\"\n    zap = 2\n");
    write(&watch_path(root), "\n[[source]]\nname = \"a\"\n\tzap = 1\n");

    let report = check(root).unwrap();
    let found: Vec<(PathBuf, Option<Position>, &AxCode)> = report
        .findings
        .iter()
        .map(|finding| (finding.path.clone(), finding.at, finding.error.code()))
        .collect();

    let invalid = &AxCode::ConfigInvalid;
    assert_eq!(
        found,
        vec![
            (city_config, at(2, 3), invalid),
            (lab_config, at(5, 1), invalid),
            (rules, at(2, 4), invalid),
            (schedule_path(root), at(3, 5), invalid),
            (watch_path(root), at(4, 2), invalid),
        ]
    );
    assert_eq!(report.read, 5);
}
