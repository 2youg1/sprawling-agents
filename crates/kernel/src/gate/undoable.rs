// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Which connector tools reach effects nothing here can take back, and
//! the person that puts in front of.
//!
//! Every consequential path in this city ships with a way back. A
//! `Discard` cannot be constructed without a `Restoration`; a tool wave
//! sits between two git fences; a write outside the domain is refused.
//! The desktop connector matches none of that. The key `desktop.act`
//! presses on somebody's own machine, and the text `desktop.clipboard`
//! replaces, are not values this city ever held — so there is nothing
//! here to put back.
//!
//! **So this door escalates rather than denies.** Denying would make the
//! tool equivalent to absent, and letting it through would leave a model
//! pressing keys on somebody's keyboard while nobody is looking. The
//! middle answer is the one a `GateOutcome` already has, and it is the
//! reason gates answer with three verdicts rather than a bool: this is
//! the person's decision.

use crate::approval::ApprovalClass;
use crate::locator::Locator;
use crate::taint::TaintSet;
use crate::tool::{ServerLabel, ToolName};

use super::item;
use super::{GateContext, GateOutcome};

/// The remote-name prefix this server's tools carry. It is the desktop
/// connector's own vocabulary (`desktop/desktop-SPEC.md` §8-7), quoted
/// here because this is the side that has to recognise it.
const REACHES_THIS_MACHINE: &str = "desktop.";

/// Which connector tool one call names.
///
/// One value rather than two parameters because neither decides
/// anything alone: the judgement is precisely "what is left of the tool
/// name once its label is taken off the front", so the label and the
/// name are two halves of one fact.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ConnectorCall<'a> {
    pub label: &'a ServerLabel,
    pub tool: &'a ToolName,
}

/// Whether this connector tool reaches something nothing here can undo.
///
/// The judgement is exact rather than a guess. A connector tool is named
/// `{label}_{sanitised remote name}`, and the label travels in the
/// `Effect::Connector` that routed the call, so removing the label gives
/// back the remote name — no substring search, and therefore no chance
/// of catching a building's own `notes_desktop_layout` in the same net.
///
/// The remote name arrives sanitised, so `desktop.act` reads as
/// `desktop_act`: the separator is compared in the form the name
/// actually has rather than the form it had before registration.
#[must_use]
pub fn reaches_the_undoable(call: &ConnectorCall<'_>) -> bool {
    let prefix = format!("{}_", call.label.as_str());
    let Some(remote) = call.tool.as_str().strip_prefix(&prefix) else {
        return false;
    };
    let sanitised = REACHES_THIS_MACHINE.replace('.', "_");
    remote.starts_with(&sanitised)
}

/// The door: a call that reaches this machine's own desktop is put in
/// front of a person; everything else passes.
///
/// Never a `Deny`. A refusal here would be indistinguishable from the
/// tool not existing, and the building already said it wanted this
/// connector — what it did not say is that every individual click may
/// happen unwatched.
///
/// The cluster is the **connector**, not the tool. The person is being
/// asked whether this connector may reach their machine, which is one
/// question with one answer; asking once per tool would train them to
/// click through it, and that is the habit this door exists to prevent.
#[must_use]
pub fn undoable(
    ctx: &GateContext,
    call: &ConnectorCall<'_>,
    artifact: &Locator,
    taint: &TaintSet,
) -> GateOutcome {
    if !reaches_the_undoable(call) {
        return GateOutcome::Allow;
    }
    GateOutcome::Escalate {
        item: item(
            ctx,
            ApprovalClass::Undoable,
            call.label.as_str().to_owned(),
            format!(
                "`{}` reaches this machine's own desktop through `{}`, and nothing here can \
                 undo what it does there",
                call.tool.as_str(),
                call.label.as_str()
            ),
            artifact.clone(),
            !taint.is_empty(),
        ),
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
    use crate::approval::ApprovalId;
    use crate::event::TimeMs;

    fn ctx() -> GateContext {
        GateContext {
            actor: "lab/room1".into(),
            now: TimeMs::new(9),
            item_id: ApprovalId::new("item-1").unwrap(),
        }
    }

    fn artifact() -> Locator {
        Locator::parse(&format!("cas:b3-{}", "bb".repeat(32))).unwrap()
    }

    fn label(named: &str) -> ServerLabel {
        ServerLabel::parse(named).unwrap()
    }

    fn tool(named: &str) -> ToolName {
        ToolName::parse(named).unwrap()
    }

    /// Every one of the six, however the building labelled the server.
    #[test]
    fn every_desktop_tool_reaches_the_undoable_under_any_label() {
        for named in ["desk", "desktop", "d2"] {
            let server = label(named);
            for remote in [
                "desktop_windows",
                "desktop_snapshot",
                "desktop_act",
                "desktop_screenshot",
                "desktop_record",
                "desktop_clipboard",
            ] {
                let full = tool(&format!("{named}_{remote}"));
                assert!(
                    reaches_the_undoable(&ConnectorCall {
                        label: &server,
                        tool: &full
                    }),
                    "{named} / {remote} was not recognised"
                );
            }
        }
    }

    /// The property the prefix strip rests on: a `ServerLabel` holds no
    /// underscore, so `{label}_{remote}` splits at exactly one place and
    /// the remote name comes back whole. If a label could contain one,
    /// this door would be guessing where the boundary was.
    #[test]
    fn a_label_carries_no_underscore_so_the_split_is_unambiguous() {
        assert!(ServerLabel::parse("my_machine").is_err());
        assert!(ServerLabel::parse("desk").is_ok());
        assert!(ServerLabel::parse("d2").is_ok());
    }

    /// The judgement is on the remote name, not on the whole string. A
    /// building's own tool that happens to contain the word is not this
    /// connector, and catching it would put a person in front of a
    /// decision that has nothing to do with their machine.
    #[test]
    fn a_tool_that_merely_contains_the_word_is_not_this_connector() {
        let server = label("notes");
        for innocent in [
            "notes_desktop_layout_read",
            "notes_write_desktop",
            "notes_search",
        ] {
            let full = tool(innocent);
            let reaches = reaches_the_undoable(&ConnectorCall {
                label: &server,
                tool: &full,
            });
            assert_eq!(
                reaches,
                innocent.starts_with("notes_desktop_"),
                "{innocent} was judged wrong"
            );
        }
        // A tool registered under a different label is not this one
        // either, even though the tail matches.
        assert!(!reaches_the_undoable(&ConnectorCall {
            label: &label("other"),
            tool: &tool("desk_desktop_act"),
        }));
    }

    /// The door's shape: escalate, never deny, and cluster by the
    /// connector so one answer covers it.
    #[test]
    fn the_door_asks_a_person_once_per_connector_and_never_refuses() {
        let outcome = undoable(
            &ctx(),
            &ConnectorCall {
                label: &label("desk"),
                tool: &tool("desk_desktop_act"),
            },
            &artifact(),
            &TaintSet::empty(),
        );
        let GateOutcome::Escalate { item } = outcome else {
            panic!("a desktop action is the person's decision, not this gate's")
        };
        assert_eq!(item.cluster_key.class, ApprovalClass::Undoable);
        assert_eq!(item.cluster_key.detail, "desk");
        // Clustered by the connector, so the two tools below share one
        // question rather than asking twice.
        let second = undoable(
            &ctx(),
            &ConnectorCall {
                label: &label("desk"),
                tool: &tool("desk_desktop_clipboard"),
            },
            &artifact(),
            &TaintSet::empty(),
        );
        let GateOutcome::Escalate { item: other } = second else {
            panic!("the clipboard is the same kind of decision")
        };
        assert_eq!(item.cluster_key, other.cluster_key);
        assert!(
            item.action_desc.contains("nothing here can undo"),
            "the person is told why: {}",
            item.action_desc
        );
    }

    /// Anything that is not this connector passes without a question.
    #[test]
    fn every_other_connector_tool_passes_this_door_untouched() {
        let outcome = undoable(
            &ctx(),
            &ConnectorCall {
                label: &label("mail"),
                tool: &tool("mail_send_message"),
            },
            &artifact(),
            &TaintSet::empty(),
        );
        assert!(matches!(outcome, GateOutcome::Allow));
    }

    /// Tainted input is carried onto the item, so a person answering
    /// sees that the arguments came from outside.
    #[test]
    fn taint_reaches_the_item_a_person_answers() {
        let source = crate::taint::TaintSource::new("tool_result")
            .expect("`tool_result` is a usable taint label");
        let tainted = TaintSet::of(source);
        let outcome = undoable(
            &ctx(),
            &ConnectorCall {
                label: &label("desk"),
                tool: &tool("desk_desktop_act"),
            },
            &artifact(),
            &tainted,
        );
        let GateOutcome::Escalate { item } = outcome else {
            panic!("still the person's decision")
        };
        assert!(item.tainted);
    }
}
