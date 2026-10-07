// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use super::*;

/// The default supplier, reached for real: the measurement city D25's
/// mapping rests on, repeated through the production tool. It needs
/// the public network, so it stays out of the required suite; run it
/// by name with `--run-ignored only` when Exa's `tools/list` may have
/// moved, and change `default_search_supplier` if it turns red.
#[test]
#[ignore = "reaches https://mcp.exa.ai/mcp over the public network; run it by name with --run-ignored only"]
fn web_search_reaches_the_default_supplier_live() {
    let dir = tempfile::tempdir().unwrap();
    let worker = worker_holding(dir.path(), &[]);
    let tool = offered(&worker, dir.path(), &SearchConfiguration::Default);

    let answer = search(
        &tool,
        serde_json::json!({
            "query": "Model Context Protocol streamable HTTP transport specification",
            "objective": "the official specification page of the transport",
            "num_results": 2
        }),
    )
    .unwrap();

    let result = answer.result.as_map();
    assert_ne!(result.get("isError"), Some(&serde_json::json!(true)));
    let content = result.get("content").and_then(serde_json::Value::as_array);
    assert!(
        content.is_some_and(|items| !items.is_empty()),
        "Exa answered with content: {result:?}"
    );
}
