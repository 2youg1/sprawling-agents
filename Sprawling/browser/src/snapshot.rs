// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! What a model is allowed to see of a page.
//!
//! Raw DOM never enters the window. Not because it is large — though it
//! is — but because it is the page author's text, and a page author is
//! a stranger. What crosses is an accessibility tree flattened into
//! named roles with short labels, each carrying a reference the actor
//! can name back.
//!
//! References are positions in this snapshot, not identities in the
//! page. A reference from an older snapshot addressed a page that has
//! since moved, so [`PageSnapshot::resolve`] refuses it rather than
//! acting on whatever now sits in that position.

use kernel::{AxCode, AxError};
use serde_json::Value;

/// One line of what a model sees.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Node {
    /// What the caller names to act on this node.
    pub reference: String,
    pub role: String,
    pub name: String,
}

/// The page as text, plus the references that text mentions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PageSnapshot {
    /// Which snapshot this is. A reference minted here is refused by a
    /// later snapshot: the page moved, and acting anyway would act on
    /// whatever took that node's place.
    generation: u64,
    nodes: Vec<Node>,
}

/// How much of a label survives. Long enough to tell two controls apart,
/// short enough that a page of them is still a page: the label is a
/// handle, and the page author chooses how long it is.
const LABEL_MAX: usize = 120;

/// What an element is, when its author did not say.
///
/// The one authority on the role vocabulary: the script that collects
/// the tree is generated from this table ([`role_lookup_js`]), and the
/// filter below admits what this table can produce. Before the table
/// existed the script called an element by its tag name and the filter
/// looked for ARIA roles, so the two agreed on five words out of
/// fourteen: a page that did not hand-write `role=` had no links and no
/// input fields in its snapshot, and a model had nothing to act on.
///
/// Rows are (tag name, the `type` this row is about, role). A row's
/// `type` is `None` when the element has no `type` attribute worth
/// consulting. An `<input>` with no `type` is a text field, which is
/// what the HTML default says, so the generated script reads an absent
/// `type` as `text`; an `<input>` whose type no row names — `hidden`,
/// `file` — gets no role and does not cross.
pub(crate) const ROLE_MAP: [(&str, Option<&str>, &str); 27] = [
    ("a", None, "link"),
    ("button", None, "button"),
    ("input", Some("button"), "button"),
    ("input", Some("submit"), "button"),
    ("input", Some("reset"), "button"),
    ("input", Some("image"), "button"),
    ("input", Some("checkbox"), "checkbox"),
    ("input", Some("radio"), "radio"),
    ("input", Some("text"), "textbox"),
    ("input", Some("search"), "textbox"),
    ("input", Some("email"), "textbox"),
    ("input", Some("tel"), "textbox"),
    ("input", Some("url"), "textbox"),
    ("input", Some("password"), "textbox"),
    ("textarea", None, "textbox"),
    ("select", None, "combobox"),
    ("datalist", None, "listbox"),
    ("option", None, "option"),
    ("h1", None, "heading"),
    ("h2", None, "heading"),
    ("h3", None, "heading"),
    ("h4", None, "heading"),
    ("h5", None, "heading"),
    ("h6", None, "heading"),
    ("dialog", None, "dialog"),
    ("table", None, "table"),
    ("form", None, "form"),
];

/// Roles no element implies, admitted because a page that states one
/// states something a model has to know: which control is the current
/// tab, and what the page is complaining about.
const ARIA_ONLY: [&str; 2] = ["tab", "alert"];

/// Whether a role reaches the window. A closed vocabulary rather than a
/// filter, because "everything except" grows silently whenever the
/// platform adds a role, and the thing it grows into is the raw DOM
/// again.
fn shown(role: &str) -> bool {
    ROLE_MAP.iter().any(|(_, _, mapped)| *mapped == role) || ARIA_ONLY.contains(&role)
}

/// The role table as the JavaScript object the collecting script looks
/// an element up in: `"tag|type"` for a row that names a type, `"tag"`
/// for one that does not.
///
/// Built with `serde_json` so quoting has one author, and generated
/// rather than written out so the script and the filter cannot come to
/// disagree about what a `<select>` is.
pub(crate) fn role_lookup_js() -> String {
    let mut map = serde_json::Map::new();
    for (tag, kind, role) in ROLE_MAP {
        let key = match kind {
            Some(kind) => format!("{tag}|{kind}"),
            None => tag.to_owned(),
        };
        map.insert(key, Value::String(role.to_owned()));
    }
    Value::Object(map).to_string()
}

fn truncate_label(raw: &str) -> String {
    let cleaned: String = raw
        .chars()
        .map(|c| if c.is_control() { ' ' } else { c })
        .collect();
    let trimmed = cleaned.split_whitespace().collect::<Vec<&str>>().join(" ");
    if trimmed.chars().count() <= LABEL_MAX {
        return trimmed;
    }
    let mut out: String = trimmed.chars().take(LABEL_MAX).collect();
    out.push('…');
    out
}

impl PageSnapshot {
    /// Flattens a driver's accessibility tree.
    ///
    /// # Errors
    /// Refuses a value that is not the array of nodes this version
    /// reads. A partial parse would hand a model a page with things
    /// silently missing from it, which is worse than no page.
    pub fn read(generation: u64, tree: &Value) -> Result<PageSnapshot, AxError> {
        let entries = tree.as_array().ok_or_else(|| {
            AxError::failure(
                AxCode::WireMismatch,
                "read an accessibility tree",
                "not an array of nodes",
            )
            .with_recovery("check the script that collects the tree")
        })?;
        let mut nodes = Vec::new();
        for entry in entries {
            let Some(role) = entry.get("role").and_then(Value::as_str) else {
                continue; // a node without a role is furniture
            };
            if !shown(role) {
                continue;
            }
            let name = truncate_label(
                entry
                    .get("name")
                    .and_then(Value::as_str)
                    .unwrap_or_default(),
            );
            let index = nodes.len();
            nodes.push(Node {
                reference: format!("e{}", index.saturating_add(1)),
                role: role.to_owned(),
                name,
            });
        }
        Ok(PageSnapshot { generation, nodes })
    }

    #[must_use]
    pub fn generation(&self) -> u64 {
        self.generation
    }

    #[must_use]
    pub fn nodes(&self) -> &[Node] {
        &self.nodes
    }

    /// The compact text that goes into the window. Deterministic by
    /// construction: same tree, same bytes.
    #[must_use]
    pub fn to_text(&self) -> String {
        let mut out = String::new();
        for node in &self.nodes {
            out.push_str(&node.reference);
            out.push(' ');
            out.push_str(&node.role);
            if !node.name.is_empty() {
                out.push_str(" \"");
                out.push_str(&node.name);
                out.push('"');
            }
            out.push('\n');
        }
        out
    }

    /// Turns a reference back into the node it named.
    ///
    /// # Errors
    /// Refuses a reference this snapshot did not mint, and one minted
    /// against an older generation. Both mean the same thing to the
    /// caller — look again — and both are refusals rather than a
    /// best-effort match on position.
    pub fn resolve(&self, reference: &str) -> Result<&Node, AxError> {
        self.nodes
            .iter()
            .find(|node| node.reference == reference)
            .ok_or_else(|| {
                AxError::failure(
                    AxCode::InvalidArgs,
                    "resolve a page reference",
                    reference.to_owned(),
                )
                .with_recovery(format!(
                    "take a fresh snapshot; this one has {} references, e1 upwards",
                    self.nodes.len()
                ))
            })
    }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "test code"
)]
mod tests {
    use super::*;
    use serde_json::json;

    fn tree() -> Value {
        json!([
            { "role": "heading", "name": "Orders" },
            { "role": "script", "name": "window.__data" },
            { "role": "button", "name": "Place order" },
            { "name": "no role at all" },
            { "role": "textbox", "name": "Quantity" },
        ])
    }

    #[test]
    fn only_the_named_roles_cross_and_the_rest_is_furniture() {
        let snapshot = PageSnapshot::read(1, &tree()).unwrap();
        assert_eq!(snapshot.nodes().len(), 3);
        let text = snapshot.to_text();
        assert!(text.contains("e1 heading \"Orders\""));
        assert!(text.contains("e2 button \"Place order\""));
        assert!(
            !text.contains("window.__data"),
            "a page author's script is not part of what a model reads: {text}"
        );
    }

    /// The defect this table was made for: a page that writes no
    /// `role=` attribute still has links and fields, and the script has
    /// to call them what the filter is looking for.
    #[test]
    fn every_role_an_element_implies_is_a_role_the_window_shows() {
        for (tag, kind, role) in ROLE_MAP {
            assert!(
                shown(role),
                "<{tag}> (type {kind:?}) is collected as `{role}`, which nothing shows"
            );
        }
    }

    /// The other direction: a role in the vocabulary that no element
    /// implies and no page can state is a word with no referent.
    #[test]
    fn every_role_the_window_shows_can_reach_it() {
        for aria in ARIA_ONLY {
            assert!(shown(aria));
        }
        for role in [
            "button", "link", "textbox", "checkbox", "radio", "combobox", "listbox", "option",
            "heading", "dialog", "table", "form",
        ] {
            assert!(
                ROLE_MAP.iter().any(|(_, _, mapped)| *mapped == role),
                "`{role}` is shown but no tag is collected as one"
            );
        }
    }

    #[test]
    fn the_lookup_the_script_uses_says_what_the_table_says() {
        let js = role_lookup_js();
        let parsed: Value = serde_json::from_str(&js).unwrap();
        assert_eq!(parsed.get("a").and_then(Value::as_str), Some("link"));
        assert_eq!(
            parsed.get("input|checkbox").and_then(Value::as_str),
            Some("checkbox")
        );
        assert_eq!(
            parsed.get("select").and_then(Value::as_str),
            Some("combobox")
        );
        assert!(
            parsed.get("input|hidden").is_none(),
            "an input nobody can see is not a control"
        );
    }

    #[test]
    fn the_same_tree_produces_the_same_bytes() {
        let once = PageSnapshot::read(1, &tree()).unwrap().to_text();
        let twice = PageSnapshot::read(1, &tree()).unwrap().to_text();
        assert_eq!(once, twice);
    }

    #[test]
    fn a_label_long_enough_to_be_a_page_is_cut_to_a_handle() {
        let long = "x".repeat(400);
        let snapshot = PageSnapshot::read(1, &json!([{ "role": "button", "name": long }])).unwrap();
        let node = snapshot.resolve("e1").unwrap();
        assert_eq!(node.name.chars().count(), LABEL_MAX + 1);
        assert!(node.name.ends_with('…'));
    }

    #[test]
    fn control_characters_in_a_label_do_not_reach_the_window() {
        let snapshot = PageSnapshot::read(
            1,
            &json!([{ "role": "link", "name": "click\n\there\u{0}now" }]),
        )
        .unwrap();
        let text = snapshot.to_text();
        assert!(text.contains("e1 link \"click here now\""), "{text}");
        assert_eq!(text.lines().count(), 1, "a label cannot invent a new line");
    }

    #[test]
    fn a_reference_this_snapshot_did_not_mint_is_refused_with_the_range() {
        let snapshot = PageSnapshot::read(1, &tree()).unwrap();
        let err = snapshot.resolve("e9").unwrap_err();
        assert_eq!(err.code(), &AxCode::InvalidArgs);
        assert!(err.recovery().contains("fresh snapshot"));
        assert!(err.recovery().contains('3'));
    }

    #[test]
    fn a_tree_this_version_cannot_read_is_refused_rather_than_half_read() {
        let err = PageSnapshot::read(1, &json!({ "nodes": [] })).unwrap_err();
        assert_eq!(err.code(), &AxCode::WireMismatch);
    }
}
