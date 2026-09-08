// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::path::PathBuf;

use super::*;

/// The badge files that exist on disk, sorted.
fn rendered_files(root: &Path) -> Vec<PathBuf> {
    let Ok(entries) = std::fs::read_dir(root.join(DIR)) else {
        return Vec::new();
    };
    let mut out: Vec<PathBuf> = entries.flatten().map(|e| e.path()).collect();
    out.sort();
    out
}

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf()
}

fn test_palette() -> Palette {
    palette(&repo_root()).unwrap()
}

#[test]
fn white_and_black_survive_the_colour_conversion() {
    assert_eq!(srgb_channels(1.0, 0.0, 0.0), (255, 255, 255));
    assert_eq!(srgb_channels(0.0, 0.0, 0.0), (0, 0, 0));
}

#[test]
fn the_palette_is_the_products_own_and_stays_above_the_information_floor() {
    let source = std::fs::read_to_string(repo_root().join(color::THEME)).unwrap();
    let ramp = color::grey_ramp(&source);
    let floor = ramp
        .iter()
        .find(|(name, _)| name == "G7")
        .map(|(_, l)| *l)
        .unwrap();
    for name in ["G8", "G10"] {
        let ink = ramp
            .iter()
            .find(|(row, _)| row == name)
            .map(|(_, l)| *l)
            .unwrap();
        assert!(ink >= floor, "{name} is below the information floor");
    }
    let theme = test_palette();
    assert_ne!(theme.label_fill, theme.value_fill);
    assert_ne!(theme.label_ink, theme.value_ink);
}

#[test]
fn bytes_read_the_way_a_person_reads_them() {
    assert_eq!(human(7_676_416), "7.32 MiB");
    assert_eq!(human(461_921), "451.0 KiB");
    assert_eq!(human(999), "999 B");
}

#[test]
fn rendering_is_deterministic_and_says_both_halves() {
    let theme = test_palette();
    let first = render("binary", "7.32 MiB", &theme);
    let second = render("binary", "7.32 MiB", &theme);
    assert_eq!(first, second);
    assert!(first.contains(">binary<"));
    assert!(first.contains(">7.32 MiB<"));
    assert!(first.contains("aria-label=\"binary: 7.32 MiB\""));
    // Nothing is fetched when this renders: the one URL in the file
    // is the SVG namespace, which no reader resolves.
    let hosts = first.match_indices("//").count();
    assert_eq!(
        hosts, 1,
        "a badge may name one URL, and it is the namespace"
    );
    assert!(first.contains("xmlns=\"http://www.w3.org/2000/svg\""));
}

#[test]
fn a_wider_string_makes_a_wider_badge() {
    assert!(text_width("7.32 MiB") > text_width("9 B"));
    let theme = test_palette();
    assert!(render("binary", "12.00 MiB", &theme).len() > render("binary", "9 B", &theme).len());
}

#[test]
fn a_row_without_a_label_asks_for_no_badge() {
    let register: toml::Value = toml::from_str(
        r#"
        [with_badge]
        badge_label = "binary"
        badge_platform = "windows-x86_64"

        [without_badge]
        status = "gated"
        "#,
    )
    .unwrap();
    let plans = plans(&register);
    assert_eq!(plans.len(), 1);
    assert_eq!(plans[0].metric, "with_badge");
    assert_eq!(plans[0].platform.as_deref(), Some("windows-x86_64"));
}

#[test]
fn the_committed_badges_match_what_this_machine_would_render() {
    let root = repo_root();
    let violations = check(&root).unwrap();
    assert!(
        violations.is_empty(),
        "stale badge: {:?}",
        violations.first().map(|v| v.violation.clone())
    );
    for path in rendered_files(&root) {
        let text = std::fs::read_to_string(&path).unwrap();
        assert!(text.starts_with("<svg"), "{path:?} is not an svg");
    }
}
