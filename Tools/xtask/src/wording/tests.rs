// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use super::*;

fn found(src: &str) -> Vec<String> {
    markup::handed_to_a_reader(src)
        .into_iter()
        .map(|said| format!("{}: {}", said.seat, said.left))
        .collect()
}

#[test]
fn a_sentence_that_never_asked_the_table_is_caught() {
    let src = r#"
      <section>
        <p>nothing has cost anything yet</p>
        <span>{say("cost_total")}</span>
      </section>
    "#;
    assert_eq!(found(src), ["a text node: nothing has cost anything yet"]);
}

#[test]
fn class_lists_routes_and_wire_values_are_not_words_a_reader_was_given() {
    let src = r##"
      <a class="flex items-center gap-base text-label" href="#/city" role="link">
        {say("nav_city")}
      </a>
      <span data-kind="model_called">{run.addr}</span>
    "##;
    assert!(found(src).is_empty(), "{:?}", found(src));
}

#[test]
fn script_and_style_blocks_draw_nothing() {
    let src = r#"
      <script lang="ts">
        const hint = "this sentence reaches nobody";
      </script>
      <style>
        .figure::after { content: "nor this one"; }
      </style>
      <p>{say("cost_total")}</p>
    "#;
    assert!(found(src).is_empty(), "{:?}", found(src));
}

#[test]
fn an_expression_hole_is_not_a_text_node() {
    let src = r#"<span>{count < 10 ? say("cost_total") : say("cost_by_run")}</span>"#;
    assert!(found(src).is_empty(), "{:?}", found(src));
}

#[test]
fn a_comment_is_not_a_text_node() {
    let src = r#"
      <!-- <p>hidden words travel here</p> -->
      <span>the drawn half</span><!-- and these words do not -->
    "#;
    assert_eq!(found(src), ["a text node: the drawn half"]);
}

#[test]
fn both_seats_are_read() {
    let text = r#"<span aria-label="12 waiting">now speaking</span>"#;
    let said = found(text);
    assert_eq!(
        said,
        [
            "a text node: now speaking",
            "a spoken attribute: 12 waiting"
        ]
    );
}

#[test]
fn a_waiver_holds_across_the_lines_a_formatter_adds() {
    let lines = [
        r#"    <!-- wording-ok: the placeholder is an address -->"#,
        r#"    <Field"#,
        r#"      placeholder="https://api.openai.com/v1""#,
        r#"      label={say("setup_base_url")}"#,
        r#"    />"#,
    ];
    assert!(waived(&lines, 3));
}

#[test]
fn a_marker_inside_a_construct_covers_its_entries() {
    let lines = [
        r#"    rows: ["#,
        r#"      // wording-ok: fixture states the wire's own refusal"#,
        r#"      {"#,
        r#"        code: "E_CONFIG_INVALID","#,
        r#"      },"#,
    ];
    assert!(waived(&lines, 4));
}

#[test]
fn a_block_that_closed_above_the_word_waives_nothing() {
    let lines = [
        r#"  function earlier(): void {"#,
        r#"        // wording-ok: a name, not a sentence"#,
        r#"        const url = "https://api.openai.com/v1";"#,
        r#"  }"#,
        r#"  const later = "the city said no";"#,
    ];
    assert!(!waived(&lines, 5));
}

#[test]
fn a_marker_after_the_word_waives_its_line() {
    let lines = [r#"  const host = "api.openai.com"; // wording-ok: a host"#];
    assert!(waived(&lines, 1));
}
