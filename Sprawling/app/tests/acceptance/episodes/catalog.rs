// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::path::Path;

use kernel::EventKind;
use serde_json::{Value, json};

use crate::script::Step;

use super::{
    Episode, Setup, address, answered, answered_text, ensure, failed, notes,
    started_under,
};

const MARK: &str = "kiln-temperature-1280";

pub(super) fn status() -> Episode {
    Episode {
        step: Step {
            tool: "status",
            args: json!({}),
        },
        holds: |seen| {
            let line = format!("addr: {}", seen.setup.room);
            let text = answered_text(seen.answer)?;
            ensure(
                text.lines().any(|at| at == line),
                format!("no `{line}` line in {text:?}"),
            )
        },
    }
}

pub(super) fn neighbours() -> Episode {
    Episode {
        step: Step {
            tool: "neighbours",
            args: json!({}),
        },
        holds: |seen| {
            let entry = format!("- {}:", seen.setup.neighbour);
            let text = answered_text(seen.answer)?;
            ensure(
                text.lines().any(|at| at.starts_with(&entry)),
                format!("{} is not listed in {text:?}", seen.setup.neighbour),
            )
        },
    }
}

pub(super) fn edit(setup: &Setup) -> Episode {
    Episode {
        step: Step {
            tool: "edit",
            args: json!({
                "file_path": notes(setup),
                "old_string": "",
                "new_string": format!("# Notes\n\n{MARK}\n"),
            }),
        },
        holds: |seen| {
            let result = answered(seen.answer)?;
            ensure(
                result.get("diff").is_some(),
                format!("a creation answered {result:?}"),
            )?;
            ensure(
                seen.history
                    .wrote(seen.lead, EventKind::CheckpointCommitted),
                "no checkpoint was committed after the write".to_owned(),
            )
        },
    }
}

pub(super) fn read(setup: &Setup) -> Episode {
    Episode {
        step: Step {
            tool: "read",
            args: json!({ "file_path": notes(setup) }),
        },
        holds: |seen| {
            let text = answered_text(seen.answer)?;
            ensure(
                text.contains(MARK),
                format!("the notes read back as {text:?}"),
            )
        },
    }
}

pub(super) fn search() -> Episode {
    Episode {
        step: Step {
            tool: "search",
            args: json!({ "pattern": MARK, "output_mode": "content" }),
        },
        holds: |seen| {
            let result = answered(seen.answer)?;
            let at = notes(seen.setup);
            let found = result
                .get("matches")
                .and_then(Value::as_array)
                .is_some_and(|hits| hits.iter().any(|hit| hit.get("path") == Some(&json!(at))));
            ensure(found, format!("{at} is not among {result:?}"))
        },
    }
}

pub(super) fn archive() -> Episode {
    Episode {
        step: Step {
            tool: "archive",
            args: json!({
                "action": "record",
                "kind": "decision",
                "text": "The kiln is fired at night.",
            }),
        },
        holds: |seen| {
            let result = answered(seen.answer)?;
            ensure(
                result.get("recorded") == Some(&json!(true))
                    && result.get("kind") == Some(&json!("decision")),
                format!("the record answered {result:?}"),
            )?;
            ensure(
                seen.history
                    .says(EventKind::AssetArchived, "The kiln is fired at night."),
                "no asset_archived line carries the decision".to_owned(),
            )
        },
    }
}

pub(super) fn goal(setup: &Setup) -> Episode {
    Episode {
        step: Step {
            tool: "goal",
            args: json!({
                "statement": "keep the notes",
                "paths": [notes(setup)],
            }),
        },
        holds: |seen| {
            let result = answered(seen.answer)?;
            ensure(
                result.get("registered") == Some(&json!(true)),
                format!("the goal answered {result:?}"),
            )?;
            ensure(
                seen.history
                    .says(EventKind::GoalRegistered, "keep the notes"),
                "no goal_registered line carries the statement".to_owned(),
            )
        },
    }
}

pub(super) fn plan() -> Episode {
    Episode {
        step: Step {
            tool: "plan",
            args: json!({ "action": "claim", "node": "1" }),
        },
        holds: |seen| {
            let result = answered(seen.answer)?;
            ensure(
                result.get("item") == Some(&json!("wire the kiln")),
                format!("the claim answered {result:?}"),
            )?;
            ensure(
                seen.history
                    .says(EventKind::RoadmapClaimed, "\"node\":\"1\""),
                "no roadmap_claimed line names node 1".to_owned(),
            )
        },
    }
}

pub(super) fn signal(setup: &Setup) -> Episode {
    Episode {
        step: Step {
            tool: "signal",
            args: json!({
                "action": "send",
                "to": setup.neighbour,
                "text": "the kiln is free",
            }),
        },
        holds: |seen| {
            let result = answered(seen.answer)?;
            ensure(
                result.get("delivered") == Some(&json!(true))
                    && result.get("to") == Some(&json!(seen.setup.neighbour)),
                format!("the send answered {result:?}"),
            )?;
            ensure(
                seen.history
                    .says(EventKind::SignalEnqueued, seen.setup.neighbour),
                format!("no signal_enqueued line names {}", seen.setup.neighbour),
            )
        },
    }
}

pub(super) fn pr_open() -> Episode {
    Episode {
        step: Step {
            tool: "pr",
            args: json!({ "action": "open" }),
        },
        holds: |seen| {
            let result = answered(seen.answer)?;
            let Some(branch) = result.get("branch").and_then(Value::as_str) else {
                return Err(format!("the request names no branch: {result:?}"));
            };
            ensure(
                seen.history.says(EventKind::PrOpened, branch),
                format!("no pr_opened line names {branch}"),
            )
        },
    }
}

pub(super) fn pr_list() -> Episode {
    Episode {
        step: Step {
            tool: "pr",
            args: json!({ "action": "list" }),
        },
        holds: |seen| {
            let result = answered(seen.answer)?;
            ensure(
                result.get("requests").is_some_and(Value::is_array),
                format!("the list answered {result:?}"),
            )
        },
    }
}

pub(super) fn draft(setup: &Setup) -> String {
    format!("{}/draft.md", setup.room)
}

pub(super) const DRAFT: &str = "# Draft\n\nThe kiln is fired at noon. It cools overnight.\n";

pub(super) fn draft_path(city: &Path, setup: &Setup) -> std::path::PathBuf {
    draft(setup)
        .split('/')
        .fold(city.to_path_buf(), |at, part| at.join(part))
}

pub(crate) fn lay_draft(city: &Path, setup: &Setup) {
    let path = draft_path(city, setup);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, DRAFT).unwrap();
}

pub(super) fn proposal(setup: &Setup) -> Episode {
    Episode {
        step: Step {
            tool: "proposal",
            args: json!({
                "action": "offer",
                "path": draft(setup),
                "old": "It cools overnight.",
                "new": "It cools by dawn.",
            }),
        },
        holds: |seen| {
            let result = answered(seen.answer)?;
            let doc = address(&draft(seen.setup));
            let open = match accounting::views::ask(seen.city, &wire::Query::Proposals(doc)) {
                Ok(wire::Answer::Proposals(cards)) => cards.open,
                other => return Err(format!("the cards answered {other:?}")),
            };
            ensure(
                open.iter().any(|card| {
                    result.get("proposal") == Some(&json!(card.id.to_string()))
                        && card.run == seen.lead
                }),
                format!("the offer answered {result:?} and the open cards are {open:?}"),
            )?;
            let now =
                std::fs::read_to_string(draft_path(seen.city, seen.setup)).unwrap_or_default();
            ensure(
                now == DRAFT,
                format!("the draft changed under an offer: {now:?}"),
            )
        },
    }
}

pub(super) fn rules() -> Episode {
    Episode {
        step: Step {
            tool: "rules",
            args: json!({ "op": "propose", "text": "confidential = true\nwrite = \"everything\"\n" }),
        },
        holds: |seen| {
            failed(seen.answer)?;
            let path = ::city::rules_path(seen.city, &address(seen.setup.building));
            let now = std::fs::read(&path).unwrap_or_default();
            ensure(
                now == seen.rules_before,
                format!("{} changed under a refused proposal", path.display()),
            )
        },
    }
}

pub(super) fn exec() -> Episode {
    Episode {
        step: Step {
            tool: "exec",
            args: json!({ "command": "echo hi > hi.txt" }),
        },
        holds: |seen| {
            let error = failed(seen.answer)?;
            ensure(
                error.get("code") == Some(&json!("E_INVALID_ARGS"))
                    && error.get("subject") == Some(&json!("`>` outside quotes")),
                format!("the command answered {error:?}"),
            )
        },
    }
}

pub(super) fn delegate(setup: &Setup) -> Episode {
    Episode {
        step: Step {
            tool: "delegate",
            args: json!({
                "room": format!("{}/helper", setup.building),
                "task": "measure the kiln",
                "goal": "a number, then stop",
            }),
        },
        holds: |seen| {
            answered(seen.answer)?;
            started_under(seen, &format!("{}/helper", seen.setup.building))
        },
    }
}

pub(super) fn workshop(setup: &Setup) -> Episode {
    Episode {
        step: Step {
            tool: "workshop",
            args: json!({
                "op": "lay_out",
                "nodes": [{
                    "room": format!("{}/reader", setup.building),
                    "goal": "read the notes",
                    "done_check": "the notes were read",
                    "stop": "when the notes were read",
                    "depends_on": [],
                }],
            }),
        },
        holds: |seen| {
            answered(seen.answer)?;
            started_under(seen, &format!("{}/reader", seen.setup.building))
        },
    }
}

pub(super) fn succeed() -> Episode {
    Episode {
        step: Step {
            tool: "succeed",
            args: json!({ "reason": "the window is full" }),
        },
        holds: |seen| {
            answered(seen.answer)?;
            ensure(
                seen.history
                    .started()
                    .iter()
                    .any(|started| started.record.predecessor == Some(seen.lead)),
                "no run started as this run's successor".to_owned(),
            )
        },
    }
}

pub(super) fn detail() -> Episode {
    Episode {
        step: Step {
            tool: "tool",
            args: json!({ "op": "detail", "name": "archive" }),
        },
        holds: |seen| {
            let text = answered_text(seen.answer)?;
            ensure(
                text.starts_with("tool archive (sprawling): run it with `tool`")
                    && text.contains("input schema:"),
                format!("the guide of a dormant tool read {text:?}"),
            )
        },
    }
}

pub(super) fn list(namespace: &'static str) -> Episode {
    Episode {
        step: Step {
            tool: "tool",
            args: json!({ "op": "list", "namespace": namespace }),
        },
        holds: |seen| {
            let text = answered_text(seen.answer)?;
            ensure(
                text.lines().all(|line| line.starts_with("- ")),
                format!("a namespace listed {text:?}"),
            )
        },
    }
}

pub(super) fn run_itself() -> Episode {
    Episode {
        step: Step {
            tool: "tool",
            args: json!({ "op": "run", "name": "tool", "args": {} }),
        },
        holds: |seen| {
            let error = failed(seen.answer)?;
            ensure(
                error.get("subject") == Some(&json!("`tool` cannot run itself")),
                format!("a `tool` `run` naming itself answered {error:?}"),
            )
        },
    }
}

pub(super) fn city() -> Episode {
    Episode {
        step: Step {
            tool: "city",
            args: json!({ "action": "raise", "name": "kiln" }),
        },
        holds: |seen| {
            failed(seen.answer)?;
            ensure(
                !seen.city.join("kiln").exists(),
                "a refused raise left a building behind".to_owned(),
            )
        },
    }
}
