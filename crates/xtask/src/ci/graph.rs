// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Verification nodes and their dependency closure (xtask `spec/Ci.lean`).

use std::collections::{BTreeMap, BTreeSet};

use crate::report::XtaskError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Reuse {
    SuccessOnly,
    Never,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Operation {
    Gate(&'static str),
    Just(&'static str),
    Process {
        program: &'static str,
        args: &'static [&'static str],
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Input {
    pub(crate) root: &'static str,
    pub(crate) extensions: &'static [&'static str],
}

impl Input {
    pub(crate) const fn all(root: &'static str) -> Self {
        Self {
            root,
            extensions: &[],
        }
    }

    pub(crate) const fn only(root: &'static str, extensions: &'static [&'static str]) -> Self {
        Self { root, extensions }
    }

    pub(crate) fn matches(&self, path: &str) -> bool {
        let under = self.root.is_empty()
            || path == self.root
            || path
                .strip_prefix(self.root)
                .is_some_and(|tail| tail.starts_with('/'));
        let extension = path.rsplit('.').next().unwrap_or(path);
        under && (self.extensions.is_empty() || self.extensions.contains(&extension))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Action {
    pub(crate) id: &'static str,
    pub(crate) operation: Operation,
    pub(crate) inputs: &'static [Input],
    pub(crate) needs: &'static [&'static str],
    pub(crate) reuse: Reuse,
    pub(crate) required: bool,
}

pub(crate) fn validate(actions: &[Action]) -> Result<(), XtaskError> {
    let mut ids = BTreeMap::new();
    for action in actions {
        if action.id.is_empty() || ids.insert(action.id, action).is_some() {
            return Err(invalid(
                action.id,
                "duplicate-node",
                "give every node a unique name",
            ));
        }
    }
    for action in actions {
        for need in action.needs {
            if !ids.contains_key(need) {
                return Err(invalid(
                    action.id,
                    "unknown-dependency",
                    "declare the dependency before using it",
                ));
            }
        }
    }
    for action in actions {
        let mut visiting = BTreeSet::new();
        let mut done = BTreeSet::new();
        visit(action, &ids, &mut visiting, &mut done)?;
    }
    Ok(())
}

fn visit<'a>(
    action: &'a Action,
    ids: &BTreeMap<&'static str, &'a Action>,
    visiting: &mut BTreeSet<&'static str>,
    done: &mut BTreeSet<&'static str>,
) -> Result<(), XtaskError> {
    if done.contains(action.id) {
        return Ok(());
    }
    if !visiting.insert(action.id) {
        return Err(invalid(
            action.id,
            "dependency-cycle",
            "break the cycle so every node has a finite prerequisite chain",
        ));
    }
    for need in action.needs {
        if let Some(next) = ids.get(need) {
            visit(next, ids, visiting, done)?;
        }
    }
    visiting.remove(action.id);
    done.insert(action.id);
    Ok(())
}

fn invalid(id: &str, code: &'static str, recovery: &'static str) -> XtaskError {
    XtaskError::Doc {
        file: format!("verification graph node {id} ({code})"),
        msg: recovery.to_owned(),
    }
}

/// Full verification includes every required node and every prerequisite, even
/// when that prerequisite is not itself a required verdict.
pub(crate) fn full(actions: &[Action]) -> BTreeSet<String> {
    let mut selected: BTreeSet<String> = actions
        .iter()
        .filter(|action| action.required)
        .map(|action| action.id.to_owned())
        .collect();
    loop {
        let before = selected.len();
        for action in actions {
            if selected.contains(action.id) {
                for need in action.needs {
                    selected.insert((*need).to_owned());
                }
            }
        }
        if selected.len() == before {
            return selected;
        }
    }
}

/// Select direct readers, their reverse dependencies, and every prerequisite.
#[cfg(test)]
pub(crate) fn affected(actions: &[Action], changed: &BTreeSet<String>) -> BTreeSet<String> {
    affected_with_base(actions, &[], changed)
}

/// Include a shared input change in every node, then close over dependants and prerequisites.
pub(crate) fn affected_with_base(
    actions: &[Action],
    base: &[Input],
    changed: &BTreeSet<String>,
) -> BTreeSet<String> {
    let shared = changed
        .iter()
        .any(|path| base.iter().any(|input| input.matches(path)));
    let mut selected: BTreeSet<String> = if shared {
        actions.iter().map(|action| action.id.to_owned()).collect()
    } else {
        actions
            .iter()
            .filter(|action| {
                action
                    .inputs
                    .iter()
                    .any(|input| changed.iter().any(|path| input.matches(path)))
            })
            .map(|action| action.id.to_owned())
            .collect()
    };
    loop {
        let before = selected.len();
        for action in actions {
            if action.needs.iter().any(|need| selected.contains(*need)) {
                selected.insert(action.id.to_owned());
            }
        }
        if selected.len() == before {
            break;
        }
    }
    loop {
        let before = selected.len();
        for action in actions {
            if selected.contains(action.id) {
                for need in action.needs {
                    selected.insert((*need).to_owned());
                }
            }
        }
        if selected.len() == before {
            break;
        }
    }
    selected
}

#[cfg(test)]
#[allow(clippy::unwrap_used, reason = "test code")]
mod tests {
    use super::*;

    const NOTHING: &[Input] = &[];
    const READS_A: &[Input] = &[Input::only("a", &["rs"])];
    const READS_B: &[Input] = &[Input::only("b", &["rs"])];
    const READS_C: &[Input] = &[Input::only("c", &["rs"])];

    fn node(id: &'static str, reads: &'static [Input], needs: &'static [&'static str]) -> Action {
        Action {
            id,
            operation: Operation::Gate(id),
            inputs: reads,
            needs,
            reuse: Reuse::SuccessOnly,
            required: true,
        }
    }

    #[test]
    fn affected_includes_reverse_dependencies_and_prerequisites() {
        let graph = [
            node("a", READS_A, &[]),
            node("b", READS_B, &["a"]),
            node("c", READS_C, &["b"]),
        ];
        let changed = BTreeSet::from(["a/lib.rs".to_owned()]);
        let selected = affected(&graph, &changed);
        assert_eq!(
            selected,
            BTreeSet::from(["a".to_owned(), "b".to_owned(), "c".to_owned()])
        );
    }

    #[test]
    fn prerequisites_of_affected_nodes_are_not_lost() {
        let graph = [node("base", NOTHING, &[]), node("a", READS_A, &["base"])];
        let selected = affected(&graph, &BTreeSet::from(["a/lib.rs".to_owned()]));
        assert!(selected.contains("base"));
    }

    #[test]
    fn unrelated_changes_select_nothing() {
        let graph = [node("a", READS_A, &[])];
        assert!(affected(&graph, &BTreeSet::from(["b/lib.rs".to_owned()])).is_empty());
    }

    #[test]
    fn cycles_and_unknown_dependencies_are_refused() {
        let cycle = [node("a", NOTHING, &["b"]), node("b", NOTHING, &["a"])];
        assert!(validate(&cycle).is_err());
        assert!(validate(&[node("a", NOTHING, &["missing"])]).is_err());
        assert!(validate(&[node("a", NOTHING, &[]), node("a", NOTHING, &[])]).is_err());
    }

    #[test]
    fn full_scope_keeps_optional_prerequisites() {
        let graph = [
            Action {
                required: false,
                ..node("base", NOTHING, &[])
            },
            node("verified", NOTHING, &["base"]),
        ];
        assert!(full(&graph).contains("base"));
    }

    #[test]
    fn input_matching_obeys_directory_and_extension_boundaries() {
        let input = Input::only("crates/a", &["rs"]);
        assert!(input.matches("crates/a/src/lib.rs"));
        assert!(!input.matches("crates/a/src/lib.ts"));
        assert!(!input.matches("crates/ab/src/lib.rs"));
        assert!(Input::all("").matches("README.md"));
    }
}
