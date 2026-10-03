// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use super::*;

#[test]
fn tool_name_grammar_is_fail_closed() {
    assert!(ToolName::parse("exec").is_ok());
    assert!(ToolName::parse("l2_search_v2").is_ok());
    for bad in ["", "Exec", "with space", "dash-ed", "cjk工具", "UPPER"] {
        assert!(ToolName::parse(bad).is_err(), "{bad:?} must be rejected");
    }
}

#[test]
fn a_label_that_could_not_start_a_tool_name_is_refused() {
    assert_eq!(ServerLabel::parse("apps").unwrap().as_str(), "apps");
    assert!(ServerLabel::parse("mail2").is_ok());
    for bad in ["", "Apps", "app-s", "应用", "with space"] {
        assert!(ServerLabel::parse(bad).is_err(), "{bad:?} must be rejected");
    }
    assert!(
        ServerLabel::parse("app_s").is_err(),
        "an underscore would make `app_s_send` two different splits of one name"
    );
}

#[test]
fn a_label_round_trips_through_its_serialised_form() {
    let label = ServerLabel::parse("apps").unwrap();
    let text = serde_json::to_string(&label).unwrap();
    assert_eq!(text, "\"apps\"");
    assert_eq!(serde_json::from_str::<ServerLabel>(&text).unwrap(), label);
    assert!(
        serde_json::from_str::<ServerLabel>("\"Apps\"").is_err(),
        "the grammar holds on the way in, or a file could mint one that parse refuses"
    );
}

#[test]
fn effect_serde_names_are_snake_case() {
    let write = Effect::Write {
        domain: Address::parse("b/room").unwrap(),
    };
    let json = serde_json::to_string(&write).unwrap();
    assert_eq!(json, "{\"write\":{\"domain\":\"b/room\"}}");
    assert_eq!(
        serde_json::to_string(&Temporal::Timestamped).unwrap(),
        "\"timestamped\""
    );
}

#[test]
fn exec_arm_shapes_serialize_distinctly() {
    let program = ExecArm::Program {
        path: "git".into(),
        args: vec!["status".into()],
    };
    let json = serde_json::to_string(&program).unwrap();
    assert!(json.contains("\"program\""));
    let python = ExecArm::Python {
        code: "print(1)".into(),
    };
    assert!(
        serde_json::to_string(&python)
            .unwrap()
            .contains("\"python\"")
    );
}

#[test]
fn meta_holds_all_eight_fields() {
    let meta = ToolMeta {
        name: ToolName::parse("status").unwrap(),
        disclosure: "reports run state; call it when you need now/usage/signals".into(),
        params: Payload::empty(),
        effect: Effect::Read,
        cost_tier: CostTier::Free,
        timeout: Some(TimeoutMs(1000)),
        render: RenderIntent::Generic,
        temporal: Temporal::Timestamped,
    };
    let json = serde_json::to_value(&meta).unwrap();
    let keys: std::collections::BTreeSet<String> =
        json.as_object().unwrap().keys().cloned().collect();
    let expected: std::collections::BTreeSet<String> = [
        "name",
        "disclosure",
        "params",
        "effect",
        "cost_tier",
        "timeout",
        "render",
        "temporal",
    ]
    .into_iter()
    .map(String::from)
    .collect();
    assert_eq!(keys, expected, "all eight fields must be present on wire");
}

/// The action of a call is its tool and its arguments and nothing else:
/// two calls that differ only by wire id are one action, and a change of
/// argument or of tool is another action.
#[test]
fn a_call_s_action_is_its_tool_and_its_arguments() {
    let call = |id: &str, name: &str, cmd: &str| ToolCall {
        id: id.to_owned(),
        name: ToolName::parse(name).unwrap(),
        args: Payload::of(&serde_json::json!({ "cmd": cmd })).unwrap(),
    };
    let first = call("a", "exec", "ls").action().unwrap();
    assert_eq!(call("b", "exec", "ls").action().unwrap(), first);
    assert_ne!(call("a", "exec", "pwd").action().unwrap(), first);
    assert_ne!(call("a", "edit", "ls").action().unwrap(), first);
}

/// Writes join as a semilattice: `Domain` absorbs, `Nothing` adds
/// nothing, and two lists of paths join without a duplicate, in the order
/// they were first named.
#[test]
fn writes_join_without_losing_or_repeating_a_path() {
    use writes::Writes;
    let path = |raw: &str| Address::parse(raw).unwrap();
    let these = Writes::Paths(vec![path("a.md"), path("b.md")]);
    let those = Writes::Paths(vec![path("b.md"), path("c.md")]);
    assert_eq!(
        these.clone().and(those.clone()),
        Writes::Paths(vec![path("a.md"), path("b.md"), path("c.md")])
    );
    assert_eq!(Writes::Nothing.and(those.clone()), those);
    assert_eq!(these.clone().and(Writes::Nothing), these);
    assert_eq!(these.and(Writes::Domain), Writes::Domain);
}
