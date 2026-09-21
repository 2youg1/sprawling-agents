// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::path::{Path, PathBuf};

use super::{PLATFORMS, matrix, restated, shim, spent_suffixes, suffixes};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("xtask sits one level under the repository root")
        .to_path_buf()
}

/// Two rows claiming one platform would make which binary a person
/// gets depend on the order the assets were read in.
#[test]
fn no_two_rows_claim_one_platform_or_one_suffix() {
    for (index, row) in PLATFORMS.iter().enumerate() {
        for other in PLATFORMS.iter().skip(index.saturating_add(1)) {
            assert_ne!((row.os, row.cpu), (other.os, other.cpu));
            assert_ne!(row.suffix, other.suffix);
            assert_ne!(row.package, other.package);
            assert_ne!(
                (row.runner, row.cargo_target),
                (other.runner, other.cargo_target)
            );
        }
    }
}

/// A matrix row is a runner and the target it passes down; a host build
/// states an empty target and must be read as one rather than skipped.
#[test]
fn a_matrix_row_is_read_as_its_runner_and_its_target() {
    let text = "        include:\n          - os: windows-latest\n            target: \"\"\n\
                          - os: ubuntu-latest\n            target: x86_64-unknown-linux-musl\n";
    assert_eq!(
        matrix(text),
        vec![
            ("windows-latest".to_owned(), String::new()),
            (
                "ubuntu-latest".to_owned(),
                "x86_64-unknown-linux-musl".to_owned()
            ),
        ]
    );
}

/// The shim's table maps a machine to a package and the file to exec.
#[test]
fn a_shim_entry_is_read_as_its_key_its_package_and_its_binary() {
    let text = "  \"win32 x64\": { package: \"@sprawling/sprawling-windows-x64\", \
                binary: \"sprawling.exe\" },\n  const RELEASES = \"https://example.invalid\";\n";
    assert_eq!(
        shim(text),
        vec![(
            "win32 x64".to_owned(),
            "@sprawling/sprawling-windows-x64".to_owned(),
            "sprawling.exe".to_owned()
        )]
    );
}

/// A name written out in full is compared; one built by interpolation
/// is not a spelling, and guessing at it would convict a correct script.
#[test]
fn only_a_suffix_written_out_in_full_is_read() {
    assert_eq!(
        suffixes(
            "        linux-x86_64) suffixes=\"-x86_64-unknown-linux-musl.zip -linux-x86_64.zip\" ;;"
        ),
        vec![
            "-x86_64-unknown-linux-musl.zip".to_owned(),
            "-linux-x86_64.zip".to_owned()
        ]
    );
    assert!(suffixes("        *) suffixes=\"-${os}-${arch}.zip\" ;;").is_empty());
    assert!(suffixes("$suffix = \"-windows-$arch.zip\"").is_empty());
    assert!(suffixes("          path: target/package/*.zip").is_empty());
}

/// The register cleans itself in both directions: a recorded suffix the
/// release now builds, and one nobody spells any more, are each struck.
#[test]
fn a_recorded_suffix_that_is_no_longer_needed_is_reported() {
    let built = PLATFORMS[0].suffix.to_owned();
    let recorded = vec![built.clone(), "-linux-x86_64.zip".to_owned()];
    let found = spent_suffixes(&recorded, std::slice::from_ref(&built));
    assert_eq!(found.len(), 2, "{found:#?}");
    assert!(found[0].violation.contains(&built));
    assert!(found[1].violation.contains("nobody spells it"));
    assert!(spent_suffixes(&recorded[1..], &["-linux-x86_64.zip".to_owned()]).is_empty());
}

/// Every file that restates the table agrees with it today.
#[test]
fn the_repository_itself_passes_the_check_it_ships() {
    let found = restated(&root()).unwrap();
    assert!(found.is_empty(), "{found:#?}");
}
