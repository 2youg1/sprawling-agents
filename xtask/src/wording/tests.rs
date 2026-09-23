// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use super::*;

/// What a `.svelte` view hands a reader. The refusal seats a `.ts`
/// module writes are asserted beside their own reader.
fn found(src: &str) -> Vec<String> {
    markup::handed_to_a_reader(src)
        .into_iter()
        .map(|said| format!("{}: {}", said.seat, said.left))
        .collect()
}

/// The failure the gate exists for: a sentence that never calls `say`,
/// so neither of the phrase table's own assertions can see it. Three
/// lived on the cost page for a whole stage, and what found them was a
/// photograph of the running client.
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

/// The shape that produced 79 findings when the predicate was a
/// vocabulary instead of a position: every one of them was an address,
/// a class list or a wire value.
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

/// What the template does not draw is not read: a script block is
/// TypeScript and a style block is CSS, and both are full of literals
/// no reader ever meets.
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

/// An inline expression hole is code, and a comparison inside one is
/// not a tag: the `<` of `count < 10` opens nothing.
#[test]
fn an_expression_hole_is_not_a_text_node() {
    let src = r#"<span>{count < 10 ? say("cost_total") : say("cost_by_run")}</span>"#;
    assert!(found(src).is_empty(), "{:?}", found(src));
}

/// A comment draws nothing, however many words it packs.
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
