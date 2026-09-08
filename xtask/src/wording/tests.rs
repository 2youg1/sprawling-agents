// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use super::*;

fn found(src: &str) -> Vec<String> {
    handed_to_a_reader(src)
        .into_iter()
        .map(|said| said.left)
        .collect()
}

/// The ablation this gate exists for: the three sentences V3.50
/// took off the cost page, put back the way they were written.
#[test]
fn the_three_sentences_that_escaped_both_of_langs_assertions_are_caught() {
    let src = r##"
        fn view() -> Element {
            rsx! {
                p { class: "consumed",
                    "{render_tokens(usage.input)} in, {render_tokens(usage.output)} out"
                }
                p { class: "unpriced",
                    "{usage.unpriced_calls} call(s) came back with no price."
                }
                p { class: "spent-line",
                    "{render_usd(spent)} of that arrived through this page's own stream"
                }
            }
        }
    "##;
    let hits = found(src);
    assert_eq!(hits.len(), 3, "{hits:?}");
    assert!(hits[0].contains("in,"), "{hits:?}");
    assert!(hits[1].contains("came back with no price"), "{hits:?}");
    assert!(hits[2].contains("arrived through"), "{hits:?}");
}

/// The first cut hit 79 addresses. Each of these is the shape that
/// made it wrong, and the position rule refuses all of them without
/// naming a single one.
#[test]
fn class_names_wire_values_and_arguments_are_not_words_a_reader_was_given() {
    let src = r##"
        fn view() -> Element {
            let mode = pick("build the parser");
            rsx! {
                div { class: "panel composer", id: "compose",
                    span { class: if hot { "phase alert" } else { "phase" } }
                    button { onclick: move |_| send(Command::Dispatch { goal: "ship it" }) }
                    input { r#type: "text", value: "{addr}", name: "room" }
                    "{word(Msg::DispatchSend)}"
                    "{percent(row.share)}%"
                    "+{added}"
                    "\u{2212}{removed}"
                    "{room}/"
                }
            }
        }
    "##;
    assert!(found(src).is_empty(), "{:?}", found(src));
}

#[test]
fn a_word_in_a_text_node_or_spoken_attribute_is_caught_either_way() {
    let text = r##"fn v() { rsx! { span { class: "count", "{n} waiting" } } }"##;
    assert_eq!(found(text), vec!["waiting".to_owned()]);
    let spoken = r##"fn v() { rsx! { button { "aria-label": "dismiss" } } }"##;
    assert_eq!(found(spoken), vec!["dismiss".to_owned()]);
    let obeyed = r##"fn v() { rsx! { button { "aria-current": "true" } } }"##;
    assert!(found(obeyed).is_empty());
}

/// A match on strings sits in the middle of RSX all over this
/// client. Its arms are patterns; `rsx!` is how one gets back to
/// being content, and the walk has to tell those apart.
#[test]
fn match_arms_are_patterns_until_rsx_says_otherwise() {
    let src = r##"
        fn v() -> Element {
            rsx! {
                div {
                    match dialect {
                        "anthropic messages" => rsx! { span { "the wire spoke" } },
                        _ => rsx! { span { "{word(Msg::Unknown)}" } },
                    }
                }
            }
        }
    "##;
    assert_eq!(found(src), vec!["the wire spoke".to_owned()]);
}

#[test]
fn a_waiver_on_the_line_or_the_line_above_is_honoured() {
    let lines = [
        "a",
        "option { value: \"openai\", \"openai\" } // wording-ok: a name",
    ];
    assert!(waived(&lines, 2));
    let above = [
        "// wording-ok: the two dialects name themselves",
        "option {}",
    ];
    assert!(waived(&above, 2));
    assert!(!waived(&["plain", "plain"], 2));
}

/// Everything below a module's own `#[cfg(test)]` is evidence a
/// test wrote down, not a page.
#[test]
fn a_sentence_a_test_quotes_is_not_a_sentence_a_page_says() {
    let src = "fn v() { rsx! { p { \"live text\" } } }\n#[cfg(test)]\nmod t { const S: &str = \"quoted evidence\"; }";
    assert_eq!(found(drawn(src)), vec!["live text".to_owned()]);
}

#[test]
fn a_raw_identifier_is_not_a_raw_string() {
    let src = r##"fn v() { rsx! { input { r#type: "text", "a word here" } } }"##;
    assert_eq!(found(src), vec!["a word here".to_owned()]);
}
