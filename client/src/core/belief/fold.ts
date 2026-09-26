// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// One run's reading, moved forward by one record. The store in
// `belief.ts` decides which run a record belongs to and whether it is
// new; this decides what the record does to that run.

import { PHASES } from "../doing";
import { askOf, branchOf, completionOf, modelOf, taskOf, toolCall } from "../reading";

import type { EventRecord, RunId, Seq } from "../../wire";

import type { RunBelief } from "./shape";

// A run the page meets for the first time, at the position it met it.
export function unseen(run: RunId, at: Seq): RunBelief {
  return {
    run,
    addr: null,
    started: null,
    task: null,
    lastSeq: at,
    doing: { kind: "unknown" },
    model: null,
    pr: null,
    ask: null,
    local: true,
    saying: "",
    thinking: "",
  };
}

// One record forward, answering the run it produced and the name of the
// first field in it this build could not read.
export function fold(held: RunBelief, record: EventRecord): [RunBelief, string | null] {
  // Every record ends the ask; only the request itself states one.
  const moved: RunBelief = { ...held, lastSeq: record.seq, ask: null };
  switch (record.kind) {
    case "run_started": {
      const [task, bad] = taskOf(record);
      return [
        {
          ...moved,
          addr: record.addr ?? null,
          started: record.t,
          task,
          doing: PHASES.run_started,
        },
        bad,
      ];
    }
    case "model_called": {
      const [model, bad] = modelOf(record);
      return [
        { ...moved, doing: PHASES.model_called, model: model ?? held.model, saying: "", thinking: "" },
        bad,
      ];
    }
    case "model_returned":
      return [{ ...moved, saying: "", thinking: "" }, null];
    case "tool_called": {
      const [call, bad] = toolCall(record);
      return [
        { ...moved, doing: { kind: "calling", tool: call.name, subject: call.subject } },
        bad,
      ];
    }
    case "tool_result":
      return [{ ...moved, doing: PHASES.tool_result }, null];
    case "approval_requested": {
      const [ask, bad] = askOf(record);
      return [{ ...moved, doing: PHASES.approval_requested, ask }, bad];
    }
    case "pr_opened": {
      const [pr, bad] = branchOf(record);
      return [{ ...moved, pr }, bad];
    }
    case "run_frozen": {
      const [completion, bad] = completionOf(record);
      return [
        {
          ...moved,
          saying: "",
          thinking: "",
          doing: { kind: "frozen", completion },
        },
        bad,
      ];
    }
    // Every other kind only advances the position. Listed rather than
    // defaulted so a new kind is a decision here, not a silence, and
    // grouped a family to a line so the list reads as one block: the
    // reader's question is which kinds move a run, not where one name
    // sits among sixty.
    case "session_opened":
    case "city_initialized": case "city_halted": case "building_created":
    case "building_configured": case "building_removed": case "run_forked": case "prompt_assembled":
    case "prompt_shape_compared":
    case "result_offloaded": case "log_truncated": case "gate_checked":
    case "gate_denied": case "approval_resolved": case "policy_created":
    case "policy_revoked": case "steer_received": case "cancel_received":
    case "watchdog_fired": case "budget_limit": case "backpressure_shed":
    case "signal_enqueued": case "signal_consumed": case "draft_held":
    case "draft_resolved": case "goal_registered": case "goal_conflict":
    case "arbitration_verdict": case "pursuit_changed": case "repair_started":
    case "repair_reused": case "worktree_opened": case "checkpoint_committed":
    case "handoff_written": case "pr_merged":
    case "pr_rejected": case "roadmap_claimed": case "roadmap_finished":
    case "roadmap_released": case "roadmap_split": case "roadmap_blocked":
    case "endpoint_attached": case "endpoint_lost": case "endpoint_probed":
    case "model_selected": case "provider_degraded": case "login_started":
    case "digest_invalidated": case "eval_run": case "asset_archived":
    case "toolkit_link_opened": case "credential_lent": case "secret_captured":
    case "secret_egress_blocked": case "file_discarded": case "discard_restored":
    case "went_back": case "file_restored":
    case "autonomy_changed": case "taint_promoted": case "cross_building_transfer":
    case "governed_document_written":
    case "spine_document_written": case "rules_changed": case "cache_renewed":
    case "embedding_called": case "rerank_called":
    case "adviser_asked": case "adviser_answered": case "adviser_fell_back":
      return [moved, null];
  }
}
