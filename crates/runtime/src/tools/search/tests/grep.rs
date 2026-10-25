// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! What a search answers when it is asked in Claude Code's `Grep` shape
//! (`crates/runtime/spec/Tools/Search.lean` §8-30-2, D117, D120, D121).

use super::*;

/// The paths a `files` answer lists, in the order it lists them.
fn files(outcome: &ToolOutcome) -> Vec<String> {
    outcome.result.as_map()["files"]
        .as_array()
        .unwrap()
        .iter()
        .map(|path| path.as_str().unwrap().to_owned())
        .collect()
}

/// D120: a pattern is a regular expression in ripgrep's syntax, matched
/// one line at a time, and `-i` drops the case.
#[test]
fn a_pattern_is_a_regular_expression_and_dash_i_drops_the_case() {
    let dir = city();
    let tool = SearchTool::new(dir.path(), everywhere()).unwrap();
    let hit_lines = |args: &[(&str, Value)]| -> Vec<u64> {
        let mut asked = vec![("output_mode", Value::String("content".to_owned()))];
        asked.extend_from_slice(args);
        tool.invoke(&call(&asked)).unwrap().result.as_map()["matches"]
            .as_array()
            .unwrap()
            .iter()
            .map(|hit| hit["line"].as_u64().unwrap())
            .collect()
    };

    assert_eq!(
        [
            hit_lines(&[("pattern", Value::String(r"ledger\s+(is|again)$".to_owned()))]),
            hit_lines(&[("pattern", Value::String("^THE LEDGER".to_owned()))]),
            hit_lines(&[
                ("pattern", Value::String("^THE LEDGER".to_owned())),
                ("-i", Value::Bool(true)),
            ]),
        ],
        [vec![1], vec![], vec![1, 1]]
    );
}

/// A pattern the engine cannot compile is refused with the engine's
/// reason, so the model can correct it.
#[test]
fn a_pattern_that_does_not_compile_is_refused() {
    let dir = city();
    let tool = SearchTool::new(dir.path(), everywhere()).unwrap();

    let refused = tool.invoke(&pattern("fn main(")).unwrap_err();
    assert_eq!(
        (refused.code(), refused.subject().contains("regex")),
        (&AxCode::InvalidArgs, true),
        "{}",
        refused.subject()
    );
}

/// D117: a glob without a pattern lists the files whose names match, in
/// name order; one without `/` is matched against the file name, one
/// with `/` against the path below `path`.
#[test]
fn a_glob_alone_lists_the_files_its_name_matches() {
    let dir = city();
    std::fs::write(dir.path().join("lab").join("main.rs"), "fn main() {}\n").unwrap();
    let tool = SearchTool::new(dir.path(), everywhere()).unwrap();
    let globbed = |args: &[(&str, &str)]| {
        let asked: Vec<(&str, Value)> = args
            .iter()
            .map(|(key, value)| (*key, Value::String((*value).to_owned())))
            .collect();
        files(&tool.invoke(&call(&asked)).unwrap())
    };

    assert_eq!(
        [
            globbed(&[("glob", "*.md")]),
            globbed(&[("glob", "room1/*.md"), ("path", "lab")]),
            globbed(&[("glob", "*.{rs,toml}")]),
        ],
        [
            vec!["lab/Notes.md".to_owned(), "lab/room1/Memo.md".to_owned()],
            vec!["lab/room1/Memo.md".to_owned()],
            vec!["lab/main.rs".to_owned()],
        ]
    );
}

/// A glob beside a pattern keeps the search to the files it names.
#[test]
fn a_glob_beside_a_pattern_narrows_the_files_searched() {
    let dir = city();
    std::fs::write(dir.path().join("lab").join("ledger.rs"), "// the ledger\n").unwrap();
    let tool = SearchTool::new(dir.path(), everywhere()).unwrap();

    let outcome = tool
        .invoke(&call(&[
            ("pattern", Value::String("the ledger".to_owned())),
            ("glob", Value::String("*.rs".to_owned())),
        ]))
        .unwrap();
    assert_eq!(files(&outcome), ["lab/ledger.rs"]);
}

/// Finding a file by name opens nothing, so a file too large to search
/// and a file that is not text are listed like any other.
#[test]
fn a_glob_alone_lists_files_it_would_not_search() {
    let dir = city();
    std::fs::write(dir.path().join("lab").join("Big.md"), "x\n".repeat(600_000)).unwrap();
    std::fs::write(dir.path().join("lab").join("blob.md"), [0xff, 0xfe, 0x00]).unwrap();
    let tool = SearchTool::new(dir.path(), everywhere()).unwrap();

    let outcome = tool
        .invoke(&call(&[("glob", Value::String("lab/*.md".to_owned()))]))
        .unwrap();
    assert_eq!(
        (files(&outcome), &outcome.result.as_map()["unreadable"]),
        (
            vec![
                "lab/Big.md".to_owned(),
                "lab/Notes.md".to_owned(),
                "lab/blob.md".to_owned()
            ],
            &Value::from(0)
        )
    );
}

/// D115: a call spelled the way an older build spelled a search names
/// neither a pattern nor a glob, and is refused in words that name both.
/// A glob alone has no lines to show or count.
#[test]
fn a_call_with_no_pattern_is_refused_unless_it_finds_files_by_name() {
    let dir = city();
    let tool = SearchTool::new(dir.path(), everywhere()).unwrap();

    let older = tool
        .invoke(&call(&[("text", Value::String("ledger".to_owned()))]))
        .unwrap_err();
    let lines_of_names = tool
        .invoke(&call(&[
            ("glob", Value::String("*.md".to_owned())),
            ("output_mode", Value::String("content".to_owned())),
        ]))
        .unwrap_err();
    assert_eq!(
        [
            (
                older.code(),
                older.recovery().contains("`pattern`") && older.recovery().contains("`glob`")
            ),
            (
                lines_of_names.code(),
                lines_of_names.recovery().contains("`pattern`")
            ),
        ],
        [(&AxCode::InvalidArgs, true), (&AxCode::InvalidArgs, true)]
    );
}

/// D121: an argument whose answer would mislead if it were not read is
/// refused, with the way this search answers the same question; one that
/// only shortens the answer is not read.
#[test]
fn an_argument_ignoring_would_mislead_is_refused_and_one_it_would_not_is_left() {
    let dir = city();
    let tool = SearchTool::new(dir.path(), everywhere()).unwrap();
    let with = |key: &str, value: Value| {
        tool.invoke(&call(&[
            ("pattern", Value::String("the ledger".to_owned())),
            (key, value),
        ]))
    };

    let refused: Vec<(AxCode, bool)> = [
        with("multiline", Value::Bool(true)),
        with("type", Value::String("rust".to_owned())),
        with("offset", Value::from(64)),
    ]
    .into_iter()
    .map(|asked| {
        let err = asked.unwrap_err();
        (*err.code(), !err.recovery().is_empty())
    })
    .collect();
    assert_eq!(refused, vec![(AxCode::InvalidArgs, true); 3]);
    for (key, value) in [
        ("-n", Value::Bool(true)),
        ("head_limit", Value::from(5)),
        ("multiline", Value::Bool(false)),
        ("offset", Value::from(0)),
    ] {
        assert_eq!(
            with(key, value).unwrap().result.as_map()["count"],
            2,
            "{key}"
        );
    }
}

/// `-B` and `-A` ask for context on one side each, and override the
/// two-sided `context` (or its alias `-C`) on their side.
#[test]
fn context_is_asked_for_each_side_or_both() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("lab")).unwrap();
    std::fs::write(
        dir.path().join("lab").join("a.md"),
        "one\ntwo\nmark\nfour\nfive\n",
    )
    .unwrap();
    let tool = SearchTool::new(dir.path(), everywhere()).unwrap();
    let block = |args: &[(&str, u64)]| {
        let mut asked = vec![
            ("pattern", Value::String("mark".to_owned())),
            ("output_mode", Value::String("content".to_owned())),
        ];
        asked.extend(args.iter().map(|(key, n)| (*key, Value::from(*n))));
        let outcome = tool.invoke(&call(&asked)).unwrap();
        outcome.result.as_map()["matches"][0]["text"]
            .as_str()
            .unwrap()
            .to_owned()
    };

    assert_eq!(
        [
            block(&[("-B", 1)]),
            block(&[("-A", 2)]),
            block(&[("-C", 1)]),
            block(&[("context", 2), ("-A", 0)]),
        ],
        [
            "two\nmark".to_owned(),
            "mark\nfour\nfive".to_owned(),
            "two\nmark\nfour".to_owned(),
            "one\ntwo\nmark".to_owned(),
        ]
    );
}
