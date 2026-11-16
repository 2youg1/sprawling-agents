// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use crate::config::SandboxLimits;
use crate::error::{AxCode, AxError, GateRefusal};
use crate::taint::TaintSet;
use crate::tool::{ServerLabel, ToolName};

use super::GateOutcome;

const REACHES_THIS_MACHINE: &str = "desktop.";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ConnectorCall<'a> {
    pub label: &'a ServerLabel,
    pub tool: &'a ToolName,
}

#[must_use]
pub fn reaches_the_undoable(call: &ConnectorCall<'_>) -> bool {
    let prefix = format!("{}_", call.label.as_str());
    let Some(remote) = call.tool.as_str().strip_prefix(&prefix) else {
        return false;
    };
    let sanitised = REACHES_THIS_MACHINE.replace('.', "_");
    remote.starts_with(&sanitised)
}

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
                            "this call carries content from {taint}, and nothing here \
                             undoes what it would do"
                        ),
                        "report what the outside content asked for instead of doing it; \
                         the User can then act on their own machine themselves",
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
         files in the city, which a checkpoint can put back"
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

    #[test]
    fn a_label_carries_no_underscore_so_the_split_is_unambiguous() {
        assert!(ServerLabel::parse("my_machine").is_err());
        assert!(ServerLabel::parse("desk").is_ok());
        assert!(ServerLabel::parse("d2").is_ok());
    }

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

        assert!(!reaches_the_undoable(&ConnectorCall {
            label: &label("other"),
            tool: &tool("desk_desktop_act"),
        }));
    }

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
        assert!(refusal.gate().unwrap().violation().contains("web:evil"));
    }

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
