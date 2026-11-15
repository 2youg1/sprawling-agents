// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "test code"
)]

    use std::collections::VecDeque;

    use super::*;

    fn request() -> Value {
        json!({"model": "m", "messages": [{"role": "user", "content": "go"}]})
    }

    fn response(stop: &str, content: Value) -> Value {
        json!({"stop_reason": stop, "content": content})
    }

    fn drive(answers: Vec<Value>) -> (Result<String>, Value, String, Vec<String>) {
        let mut answers = VecDeque::from(answers);
        let mut request = request();
        let mut trace = Vec::new();
        let mut executed = Vec::new();
        let result = run(
            &mut request,
            |_, _| Ok(answers.pop_front().unwrap()),
            |code| {
                executed.push(code.to_owned());
                Ok(format!("out:{code}"))
            },
            &mut trace,
        );
        (result, request, String::from_utf8(trace).unwrap(), executed)
    }

    #[test]
    fn a_direct_answer_preserves_stdout_and_history() {
        let (answer, request, trace, executed) = drive(vec![response("end_turn", json!("answer"))]);
        assert_eq!(answer.unwrap(), "answer");
        assert!(executed.is_empty());
        assert_eq!(request["messages"].as_array().unwrap().len(), 2);
        assert_eq!(request["messages"][1]["content"], json!("answer"));
        let trace: Vec<Value> = trace
            .lines()
            .map(|line| serde_json::from_str(line).unwrap())
            .collect();
        assert_eq!(
            trace,
            vec![
                json!({"role": "user", "content": "go"}),
                json!({"role": "assistant", "content": "answer", "stop_reason": "end_turn"})
            ]
        );
    }

    #[test]
    fn usage_is_traced_as_mei_obs_and_not_appended_to_messages() {
        let mut answers = VecDeque::from(vec![json!({
            "stop_reason": "end_turn",
            "content": "answer",
            "usage": {"input_tokens": 3, "output_tokens": 1}
        })]);
        let mut request = request();
        let mut trace = Vec::new();
        let answer = run(
            &mut request,
            |_, _| Ok(answers.pop_front().unwrap()),
            |_| {
                Err(Error::from(protocol(
                    "execute Python",
                    "usage fixture must not call a tool",
                    "keep the fixture on end_turn without tool_use",
                )))
            },
            &mut trace,
        )
        .unwrap();
        assert_eq!(answer, "answer");
        assert_eq!(request["messages"].as_array().unwrap().len(), 2);
        let trace: Vec<Value> = String::from_utf8(trace)
            .unwrap()
            .lines()
            .map(|line| serde_json::from_str(line).unwrap())
            .collect();
        assert_eq!(
            trace,
            vec![
                json!({"type": "mei.obs", "kind": "usage", "usage": {"input_tokens": 3, "output_tokens": 1}}),
                json!({"role": "user", "content": "go"}),
                json!({"role": "assistant", "content": "answer", "stop_reason": "end_turn"})
            ]
        );
    }

    #[test]
    fn a_wave_runs_in_order_and_preserves_thinking() {
        let thinking = json!({"type": "thinking", "thinking": "why", "signature": "signed"});
        let first = response(
            "tool_use",
            json!([
                thinking.clone(),
                {"type": "tool_use", "name": "python", "id": "a", "input": {"code": "x = 40"}},
                {"type": "tool_use", "name": "python", "id": "b", "input": {"code": "x += 2; print(x)"}}
            ]),
        );
        let (answer, request, trace, executed) =
            drive(vec![first, response("end_turn", json!("42"))]);
        assert_eq!(answer.unwrap(), "42");
        assert_eq!(executed, ["x = 40", "x += 2; print(x)"]);
        assert_eq!(request["messages"][1]["content"][0], thinking);
        let results = request["messages"][2]["content"].as_array().unwrap();
        assert_eq!(results.len(), 2);
        assert_eq!(results[0]["tool_use_id"], "a");
        assert_eq!(results[0]["content"], "out:x = 40");
        assert_eq!(results[1]["tool_use_id"], "b");
        assert_eq!(results[1]["content"], "out:x += 2; print(x)");
        assert_eq!(trace.lines().count(), 4);
    }

    #[test]
    fn pause_reposts_the_same_assistant_without_a_user_message() {
        let (answer, request, _, executed) = drive(vec![
            response(
                "pause_turn",
                json!([{"type": "thinking", "thinking": "wait", "signature": "s"}]),
            ),
            response("end_turn", json!("done")),
        ]);
        assert_eq!(answer.unwrap(), "done");
        assert!(executed.is_empty());
        assert_eq!(request["messages"].as_array().unwrap().len(), 3);
        assert_eq!(request["messages"][1]["role"], "assistant");
        assert_eq!(request["messages"][2]["role"], "assistant");
    }

    #[test]
    fn an_invalid_member_prevents_the_whole_wave() {
        let (answer, request, _, executed) = drive(vec![response(
            "tool_use",
            json!([
                {"type": "tool_use", "name": "python", "id": "a", "input": {"code": "write()"}},
                {"type": "tool_use", "name": "shell", "id": "b", "input": {"code": "remove()"}}
            ]),
        )]);
        assert!(answer.is_err());
        assert!(executed.is_empty());
        assert_eq!(request["messages"].as_array().unwrap().len(), 2);
    }

    #[test]
    fn a_duplicate_id_prevents_the_whole_wave() {
        let (answer, _, _, executed) = drive(vec![response(
            "tool_use",
            json!([
                {"type": "tool_use", "name": "python", "id": "same", "input": {"code": "first()"}},
                {"type": "tool_use", "name": "python", "id": "same", "input": {"code": "second()"}}
            ]),
        )]);
        assert!(answer.is_err());
        assert!(executed.is_empty());
    }

    #[test]
    fn max_tokens_and_empty_waves_do_not_execute() {
        for answer in [
            response("max_tokens", json!([])),
            response("tool_use", json!([])),
        ] {
            let (result, _, _, executed) = drive(vec![answer]);
            assert!(result.is_err());
            assert!(executed.is_empty());
        }
    }

    #[test]
    fn an_execution_failure_does_not_invent_a_complete_result_message() {
        let mut request = request();
        let mut trace = Vec::new();
        let result = run(
            &mut request,
            |_, _| {
                Ok(response(
                    "tool_use",
                    json!([{"type": "tool_use", "name": "python", "id": "a", "input": {"code": "exit()"}}]),
                ))
            },
            |_| {
                Err(Error::from(protocol(
                    "execute Python",
                    "the interpreter died",
                    "start a new Mei process",
                )))
            },
            &mut trace,
        );
        assert!(result.is_err());
        assert_eq!(request["messages"].as_array().unwrap().len(), 2);
        assert_eq!(String::from_utf8(trace).unwrap().lines().count(), 2);
    }

    #[test]
    fn missing_control_fields_are_protocol_errors() {
        let (result, _, _, _) = drive(vec![json!({"content": "missing stop"})]);
        assert!(result.is_err());
        let (result, _, _, _) = drive(vec![response("end_turn", json!([{"type": "text"}]))]);
        assert!(result.is_err());
    }

    #[test]
    fn protocol_errors_name_the_offending_path_and_value() {
        let (result, _, _, _) = drive(vec![json!({"content": "missing stop"})]);
        let shown = result.unwrap_err().to_string();
        assert!(shown.contains("path = response.stop_reason"), "{shown}");
        assert!(shown.contains("value = <missing>"), "{shown}");
        let wave = response(
            "tool_use",
            json!([
                {"type": "tool_use", "name": "python", "id": "a", "input": {"code": "first()"}},
                {"type": "tool_use", "name": "python", "input": {"code": "second()"}}
            ]),
        );
        let (result, _, _, executed) = drive(vec![wave]);
        assert!(executed.is_empty());
        let shown = result.unwrap_err().to_string();
        assert!(
            shown.contains("path = messages[1].content[1].id"),
            "{shown}"
        );
    }
