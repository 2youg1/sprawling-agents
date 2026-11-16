// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use super::*;

fn files(outcome: &ToolOutcome) -> Vec<String> {
    outcome.result.as_map()["files"]
        .as_array()
        .unwrap()
        .iter()
        .map(|path| path.as_str().unwrap().to_owned())
        .collect()
}

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

#[test]
fn a_pattern_that_spells_a_line_break_is_refused() {
    let dir = city();
    let tool = SearchTool::new(dir.path(), everywhere()).unwrap();

    let refused: Vec<AxCode> = [r"before\nthe", "before\nthe", r"before[\n]", r"x\x0Ay"]
        .into_iter()
        .map(|asked| *tool.invoke(&pattern(asked)).unwrap_err().code())
        .collect();
    assert_eq!(refused, vec![AxCode::InvalidArgs; 4]);
    assert_eq!(
        tool.invoke(&pattern(r"the\s+ledger"))
            .unwrap()
            .result
            .as_map()["count"],
        2
    );
}
