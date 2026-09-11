// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use super::*;

fn found(src: &str) -> Vec<String> {
    handed_to_a_reader(src)
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

/// A type argument list ends with the same character an element does.
/// The first run of this scanner reported ten of these.
#[test]
fn a_generic_is_not_an_element() {
    let src = r"
      const [view, setView] = createSignal<View>(DEFAULT_VIEW);
      const held: Record<string, Held> | null = null;
      const draw = (entry: Entry) => entry.label;
    ";
    assert!(found(src).is_empty(), "{:?}", found(src));
}

#[test]
fn both_seats_are_read() {
    let text = r"<span>12 waiting</span>";
    assert_eq!(found(text), ["a text node: 12 waiting"]);
    let spoken = r#"<button aria-label="dismiss" />"#;
    assert_eq!(found(spoken), ["a spoken attribute: dismiss"]);
    // `aria-current` is not a spoken attribute: its value is a state a
    // reader never hears as a word.
    let obeyed = r#"<button aria-current="true" />"#;
    assert!(found(obeyed).is_empty(), "{:?}", found(obeyed));
}

/// What the city supplies is not what the view wrote. A run of slots
/// with punctuation between them hands a reader nothing.
#[test]
fn the_citys_own_values_are_not_the_views_words() {
    let src = r"<span>{percent}%</span><span>{room}/</span><span>+{added}</span>";
    assert!(found(src).is_empty(), "{:?}", found(src));
}

/// A comment addresses a contributor, not somebody using the client.
#[test]
fn a_comment_is_not_a_page() {
    let src = r"// the composer grows to fit what somebody typed
      <span>{say('talk_send')}</span>";
    assert!(found(src).is_empty(), "{:?}", found(src));
}

#[test]
fn a_waiver_on_the_line_or_the_line_above_is_honoured() {
    let lines = vec![
        "// wording-ok: a provider's own name",
        r#"<input placeholder="openai" />"#,
        r#"<input placeholder="claude" />"#,
        r#"<input placeholder="groq" /> // wording-ok: a provider's own name"#,
    ];
    assert!(waived(&lines, 2), "the line above did not waive");
    assert!(waived(&lines, 4), "the line itself did not waive");
    // The reach is two lines, not the rest of the file: line 3 sits
    // under a line that carries no mark.
    assert!(
        !waived(&lines, 3),
        "the waiver reached further than one line"
    );
}
