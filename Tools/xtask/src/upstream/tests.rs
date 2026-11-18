// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use super::*;

const TABLE: &str = "\
# Third-party projects

## 1 Where the intelligence comes from

| Project | Licence | What is followed | Where to look | Tracked to |
|---|---|---|---|---|
| [yetone/magpie](https://github.com/yetone/magpie) | MIT | the local gateway | `docs/reference.md` | `d01e0a465aaf` |
| [yetone/magpie](https://github.com/yetone/magpie) | MIT | the catalog | `docs/integrating.md` | `15e1d65bcd48` |

## 4 What the client ships that others wrote

| File | Family | Licence |
|---|---|---|
| `RefRain/src/fonts/MapleMono-OFL.txt` | Maple Mono, release tag `v7.9` | OFL-1.1 |
";

fn one_row(where_to_look: &str, tracked: &str) -> String {
    format!(
        "| Project | Licence | What is followed | Where to look | Tracked to |\n\
         |---|---|---|---|---|\n\
         | [yetone/magpie](https://github.com/yetone/magpie) | MIT | the gateway | {where_to_look} | {tracked} |\n"
    )
}

#[test]
fn one_path_and_one_commit_per_row_is_accepted() {
    assert!(judge(TABLE).is_empty(), "{:?}", judge(TABLE));
}

#[test]
fn two_paths_in_one_row_are_refused() {
    let text = one_row(
        "`docs/reference.md`, `docs/integrating.md`",
        "`3438b7bd9c56`",
    );
    let found = judge(&text);
    let violation = found.first().expect("one violation");
    assert_eq!(violation.gate, "upstream");
    assert!(violation.location.ends_with(":3"), "{}", violation.location);
    assert!(
        violation.violation.contains("exactly one backticked path"),
        "{}",
        violation.violation
    );
    assert!(
        violation.alternative.contains("own row"),
        "{}",
        violation.alternative
    );
}

#[test]
fn a_watermark_that_is_not_a_commit_is_refused() {
    let text = one_row("`docs/reference.md`", "`moved`");
    let found = judge(&text);
    let violation = found.first().expect("one violation");
    assert!(
        violation.rule.contains("12 to 40 hexadecimal"),
        "{}",
        violation.rule
    );
    assert!(
        violation.violation.contains("moved"),
        "{}",
        violation.violation
    );
}

#[test]
fn a_row_without_a_github_project_is_refused() {
    let text = "\
| Project | Licence | What is followed | Where to look | Tracked to |
|---|---|---|---|---|
| [magpie](https://example.invalid/magpie) | MIT | the gateway | `docs/reference.md` | `d01e0a465aaf` |
";
    let found = judge(text);
    let violation = found.first().expect("one violation");
    assert!(
        violation.rule.contains("GitHub project"),
        "{}",
        violation.rule
    );
}

#[test]
fn a_cell_holding_no_path_is_refused() {
    let text = one_row("docs/reference.md", "`d01e0a465aaf`");
    let found = judge(&text);
    let violation = found.first().expect("one violation");
    assert!(
        violation.violation.contains("exactly one backticked path"),
        "{}",
        violation.violation
    );
}

#[test]
fn rows_outside_a_watched_table_are_not_judged() {
    let text = "\
| File | Family | Licence |
|---|---|---|
| `RefRain/src/fonts/MapleMono-OFL.txt` | Maple Mono, release tag `v7.9` | OFL-1.1 |
";
    assert!(judge(text).is_empty(), "{:?}", judge(text));
}

#[test]
fn this_checkout_passes() {
    let found = check(crate::root::this_checkout()).unwrap();
    assert!(found.is_empty(), "{found:#?}");
}

#[test]
fn a_cell_with_text_beside_the_path_is_refused() {
    let empty_pair = format!("{tick}{tick}", tick = '`');
    for cell in [
        "`docs/reference.md`/extra",
        "`docs/reference.md` and `docs/x.md`",
        empty_pair.as_str(),
    ] {
        let text = one_row(cell, "`d01e0a465aaf`");
        let found = judge(&text);
        let violation = found.first().expect("one violation");
        assert!(
            violation.violation.contains("exactly one backticked path"),
            "{cell}: {}",
            violation.violation
        );
    }
}
