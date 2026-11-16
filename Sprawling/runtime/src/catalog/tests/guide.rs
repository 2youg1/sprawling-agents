// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use super::*;

#[test]
fn detail_answers_a_guide_by_name_and_find_names_by_words() {
    let mut catalog = Catalog::new();
    catalog.file(&sized("apps_mail")).unwrap();
    catalog.file(&meta("read")).unwrap();
    catalog.admit_skill(skill("review")).unwrap();
    catalog.set_mode(Mode::Chat);

    let guide = catalog.detail("apps_mail").unwrap();
    assert!(guide.contains("\"additionalProperties\":false"), "{guide}");
    assert!(
        guide.contains("outside content"),
        "the whole disclosure, uncut"
    );
    assert!(
        guide.contains("\"op\": \"run\""),
        "a dormant tool says how to run it"
    );
    assert!(catalog.detail("read").unwrap().contains("call it directly"));
    let skill_guide = catalog.detail("skill review").unwrap();
    assert!(skill_guide.contains("`read`"), "{skill_guide}");

    let found = catalog.find("service instruction");
    assert!(found.starts_with("apps:\n- apps_mail:"), "{found}");
    // Punctuation a model writes around its words is not part of them.
    assert_eq!(catalog.find("\"mail?\""), catalog.find("mail"));
    assert!(catalog.find("zebra").starts_with("Nothing admitted"));
    let unknown = catalog.detail("apps_mai").unwrap_err();
    assert_eq!(*unknown.code(), AxCode::InvalidArgs);
    assert!(
        unknown.nearby().iter().any(|name| name == "apps_mail"),
        "{unknown:?}"
    );
}

/// The acceptance case for namespaces: a server offering two hundred
/// tools costs the index one line, and the words a model would ask with
/// find the tool it means among the first [`FIND_HITS`].
#[test]
fn two_hundred_tools_of_one_server_take_one_line_and_are_found() {
    let verbs = [
        "create", "list", "get", "update", "delete", "search", "merge", "close", "comment",
        "assign",
    ];
    let things = [
        "issue",
        "pull_request",
        "branch",
        "release",
        "label",
        "milestone",
        "commit",
        "review",
        "workflow",
        "gist",
        "repository",
        "team",
        "project",
        "discussion",
        "webhook",
        "secret",
        "deployment",
        "check",
        "tag",
        "file",
    ];
    let mut catalog = Catalog::new();
    for verb in verbs {
        for thing in things {
            let name = format!("github_{verb}_{thing}");
            let mut tool = sized(&name);
            tool.disclosure = format!(
                "{} a {} in a GitHub repository",
                verb,
                thing.replace('_', " ")
            );
            catalog.file(&tool).unwrap();
        }
    }
    catalog.file(&sized("read")).unwrap();
    catalog.set_mode(Mode::Work);

    let block = dormant_block(&catalog.render());
    let lines: Vec<&str> = block
        .lines()
        .filter(|line| line.starts_with("- "))
        .collect();
    assert_eq!(lines.len(), 1, "{block}");
    assert!(lines[0].starts_with("- github (200): "), "{block}");
    assert!(block.len() <= DORMANT_INDEX_CEILING);

    for (query, wanted) in [
        ("create issue", "github_create_issue"),
        ("merge pull request", "github_merge_pull_request"),
        ("delete branch", "github_delete_branch"),
        ("list release", "github_list_release"),
        ("comment on a review", "github_comment_review"),
        ("github_close_issue", "github_close_issue"),
    ] {
        let found = catalog.find(query);
        let hits: Vec<&str> = found
            .lines()
            .filter_map(|line| line.strip_prefix("- "))
            .filter_map(|line| line.split_once(':').map(|(name, _)| name))
            .collect();
        assert!(hits.len() <= FIND_HITS, "{found}");
        assert!(hits.contains(&wanted), "{query}: {found}");
    }
    assert!(catalog.list(Some("github")).lines().count() == 200);
}

/// A `tool` `run` becomes the call it stands for, with the model's id;
/// the ways it stands for nothing are each refused.
#[test]
fn resolve_run_hands_back_the_target_or_one_refusal() {
    let mut catalog = Catalog::new();
    catalog.file(&sized("apps_mail")).unwrap();

    let resolved = catalog
        .resolve_run(&call(serde_json::json!({
            "op": "run", "name": "apps_mail", "args": { "query": "invoices", "limit": 3 }
        })))
        .unwrap();
    assert_eq!(
        resolved,
        ToolCall {
            id: "toolu_1".to_owned(),
            name: ToolName::parse("apps_mail").unwrap(),
            args: Payload::of(&serde_json::json!({ "query": "invoices", "limit": 3 })).unwrap(),
        }
    );

    let itself = catalog
        .resolve_run(&call(
            serde_json::json!({ "op": "run", "name": "tool", "args": {} }),
        ))
        .unwrap_err();
    assert_eq!(*itself.code(), AxCode::InvalidArgs);
    let unknown = catalog
        .resolve_run(&call(
            serde_json::json!({ "op": "run", "name": "apps_mai", "args": {} }),
        ))
        .unwrap_err();
    assert_eq!(*unknown.code(), AxCode::InvalidArgs);
    assert!(unknown.nearby().iter().any(|name| name == "apps_mail"));
    for misfit in [
        serde_json::json!({ "op": "run", "name": "apps_mail", "args": { "limit": 3 } }),
        serde_json::json!({ "op": "run", "name": "apps_mail", "args": { "query": 7 } }),
        serde_json::json!({ "op": "run", "name": "apps_mail", "args": { "query": "x", "folder": "in" } }),
        serde_json::json!({ "op": "run", "name": "apps_mail", "args": "invoices" }),
    ] {
        let refused = catalog.resolve_run(&call(misfit)).unwrap_err();
        assert_eq!(*refused.code(), AxCode::InvalidArgs);
        assert!(
            refused.recovery().contains("\"required\":[\"query\"]"),
            "the refusal carries the schema: {}",
            refused.recovery()
        );
    }
}
