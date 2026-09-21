// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Which connector tools reach effects nothing here can take back, and
//! the rule that answers for them.
//!
//! Every consequential path in this city ships with a way back. A
//! `Discard` cannot be constructed without a `Restoration`; a tool wave
//! sits between two git fences; a write outside the domain is refused.
//! The desktop connector matches none of that. The key `desktop.act`
//! presses on somebody's own machine, and the text `desktop.clipboard`
//! replaces, are not values this city ever held — so there is nothing
//! here to put back.
//!
//! **So the floor answers in advance.** Which connector may reach this
//! machine is a sentence in `CONFIG.toml` under `[sandbox] trusted`,
//! and a run that reads content from outside reaches nothing there at
//! all: a person who wrote that sentence decided once, with the whole
//! list in front of them, which is a better moment to decide than the
//! one in which a model is waiting.

use crate::config::SandboxLimits;
use crate::error::{AxCode, AxError, GateRefusal};
use crate::taint::TaintSet;
use crate::tool::{ServerLabel, ToolName};

use super::GateOutcome;

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

/// The door: a call that reaches this machine's own desktop passes when
/// the floor trusts that connector and the run's arguments came from
/// inside the city; otherwise it is refused.
///
/// Taint outranks trust. A trusted connector still refuses a call a run
/// derived from fetched content, because what the floor trusted was the
/// connector, not whatever a web page asked it to type.
#[must_use]
pub fn undoable(
    call: &ConnectorCall<'_>,
    sandbox: &SandboxLimits,
    taint: &TaintSet,
) -> GateOutcome {
    if !reaches_the_undoable(call) {
        return GateOutcome::Allow;
    }
    let label = call.label.as_str();
    if !taint.is_empty() {
        return GateOutcome::Deny {
            refusal: Box::new(
                AxError::refusal(
                    AxCode::TaintedAction,
                    "reach this machine's desktop",
                    call.tool.as_str().to_owned(),
                    GateRefusal::new(
                        "an effect a run derived from outside content is refused (C15)",
                        format!(
                            "this call carries {} external source(s), and nothing here \
                             undoes what it would do",
                            taint.len()
                        ),
                        "report what the outside content asked for instead of doing it; \
                         a person can then act on their own machine themselves",
                    ),
                )
                .with_recovery(
                    "do this from work that did not start outside the city, or add that \
                     source to `[sandbox] trusted` in `CONFIG.toml`",
                ),
            ),
        };
    }
    if sandbox.trusts(call.label) {
        return GateOutcome::Allow;
    }
    let trusted: Vec<String> = sandbox
        .trusted
        .iter()
        .map(|allowed| allowed.as_str().to_owned())
        .collect();
    let alternative = if trusted.is_empty() {
        "this floor trusts no connector with the machine it runs on; do the work with \
         files in the city, which a fence can put back"
            .to_owned()
    } else {
        format!("connectors this floor trusts: {}", trusted.join(", "))
    };
    GateOutcome::Deny {
        refusal: Box::new(
            AxError::refusal(
                AxCode::GateDenied,
                "reach this machine's desktop",
                call.tool.as_str().to_owned(),
                GateRefusal::new(
                    "a connector reaches this machine only when the floor names it",
                    format!("`{label}` is not one of the connectors it names"),
                    alternative,
                ),
            )
            .with_nearby(trusted)
            .with_recovery(format!(
                "add `{label}` to `[sandbox] trusted` in `CONFIG.toml`; the sandbox is \
                 frozen at the start of a run, so the change takes effect in the next one"
            )),
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

    fn label(named: &str) -> ServerLabel {
        ServerLabel::parse(named).unwrap()
    }

    fn tool(named: &str) -> ToolName {
        ToolName::parse(named).unwrap()
    }

    fn floor(trusted: Vec<ServerLabel>) -> SandboxLimits {
        SandboxLimits {
            trusted,
            ..SandboxLimits::default()
        }
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
    /// connector, and catching it would refuse work that has nothing to
    /// do with anybody's machine.
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

    /// The floor's list is the answer, and the refusal names the file
    /// the person edits to change it.
    #[test]
    fn a_connector_the_floor_does_not_name_is_refused_by_name() {
        let desk = label("desk");
        let call = ConnectorCall {
            label: &desk,
            tool: &tool("desk_desktop_act"),
        };
        assert!(matches!(
            undoable(&call, &floor(vec![desk.clone()]), &TaintSet::empty()),
            GateOutcome::Allow
        ));
        let GateOutcome::Deny { refusal } = undoable(&call, &floor(Vec::new()), &TaintSet::empty())
        else {
            panic!("an untrusted connector is refused")
        };
        assert_eq!(refusal.code(), &AxCode::GateDenied);
        assert!(refusal.to_string().contains("desk_desktop_act"));
        assert!(refusal.gate().unwrap().violation().contains("desk"));
        assert!(refusal.recovery().contains("[sandbox] trusted"));
    }

    /// Taint outranks the floor's trust: what the person trusted was
    /// the connector, not a web page holding the keyboard.
    #[test]
    fn outside_content_never_reaches_the_machine_even_through_a_trusted_connector() {
        let desk = label("desk");
        let tainted = TaintSet::of(crate::taint::TaintSource::new("web:evil").unwrap());
        let GateOutcome::Deny { refusal } = undoable(
            &ConnectorCall {
                label: &desk,
                tool: &tool("desk_desktop_act"),
            },
            &floor(vec![desk.clone()]),
            &tainted,
        ) else {
            panic!("taint refuses whatever the floor trusts")
        };
        assert_eq!(refusal.code(), &AxCode::TaintedAction);
    }

    /// Anything that is not this connector passes without a question,
    /// trusted list or not.
    #[test]
    fn every_other_connector_tool_passes_this_door_untouched() {
        let mail = label("mail");
        assert!(matches!(
            undoable(
                &ConnectorCall {
                    label: &mail,
                    tool: &tool("mail_send_message"),
                },
                &floor(Vec::new()),
                &TaintSet::empty(),
            ),
            GateOutcome::Allow
        ));
    }
}
