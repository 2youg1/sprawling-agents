// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { PHASES, afterWait } from "../doing";
import {
  askOf,
  branchOf,
  completionOf,
  modelOf,
  openingOf,
  toolCall,
  waitEndOf,
  waitOf,
} from "../reading";

import { Seq } from "../../wire";
import type { EventRecord, RunId } from "../../wire";

import type { RunBelief } from "./shape";

export function unseen(run: RunId, at: Seq): RunBelief {
  return {
    run,
    addr: null,
    started: null,
    task: null,
    goal: null,
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

export function fold(held: RunBelief, record: EventRecord): [RunBelief, string | null] {
  const moved: RunBelief = { ...held, lastSeq: record.seq, ask: null };
  switch (record.kind) {
    case "run_started": {
      const [opening, bad] = openingOf(record);
      return [
        {
          ...moved,
          addr: record.addr ?? null,
          started: record.t,
          task: opening.task,
          goal: opening.goal,
          sessionBegan:
            typeof record.data.session_began === "number"
              ? Seq.make(record.data.session_began)
              : undefined,
          imageInput: record.data.image_input === true,
          doing: PHASES.run_started,
        },
        bad,
      ];
    }
    case "model_called": {
      const [model, bad] = modelOf(record);
      return [
        {
          ...moved,
          doing: PHASES.model_called,
          model: model ?? held.model,
          saying: "",
          thinking: "",
        },
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
    case "signal_wait_started": {
      const [wait, bad] = waitOf(record);
      return [wait === null ? moved : { ...moved, doing: { kind: "awaiting_reply", wait } }, bad];
    }
    case "signal_wait_ended": {
      const [end, bad] = waitEndOf(record);
      return [end === null ? moved : { ...moved, doing: afterWait(held.doing, end) }, bad];
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
    case "session_opened":
    case "city_initialized":
    case "city_halted":
    case "building_created":
    case "building_configured":
    case "building_removed":
    case "run_forked":
    case "prompt_assembled":
    case "prompt_shape_compared":
    case "result_offloaded":
    case "log_truncated":
    case "gate_checked":
    case "gate_denied":
    case "approval_resolved":
    case "policy_created":
    case "policy_revoked":
    case "steer_received":
    case "cancel_received":
    case "watchdog_fired":
    case "budget_limit":
    case "backpressure_shed":
    case "signal_enqueued":
    case "signal_consumed":
    case "draft_held":
    case "draft_resolved":
    case "goal_registered":
    case "goal_conflict":
    case "arbitration_verdict":
    case "pursuit_changed":
    case "repair_started":
    case "repair_reused":
    case "worktree_opened":
    case "checkpoint_committed":
    case "handoff_written":
    case "pr_merged":
    case "pr_rejected":
    case "roadmap_claimed":
    case "roadmap_finished":
    case "roadmap_released":
    case "roadmap_split":
    case "roadmap_blocked":
    case "endpoint_attached":
    case "endpoint_lost":
    case "endpoint_probed":
    case "model_selected":
    case "provider_degraded":
    case "login_started":
    case "digest_invalidated":
    case "eval_run":
    case "asset_archived":
    case "toolkit_link_opened":
    case "credential_lent":
    case "secret_captured":
    case "secret_egress_blocked":
    case "file_discarded":
    case "discard_restored":
    case "went_back":
    case "file_restored":
    case "autonomy_changed":
    case "taint_promoted":
    case "cross_building_transfer":
    case "governed_document_written":
    case "spine_document_written":
    case "rules_changed":
    case "cache_renewed":
    case "embedding_called":
    case "rerank_called":
    case "adviser_asked":
    case "adviser_answered":
    case "adviser_fell_back":
    case "harness_reported":
    case "harness_answered":
    case "remote_opened":
    case "remote_closed":
    case "device_paired":
    case "device_revoked":
    case "remote_session_started":
    case "document_written":
    case "proposal_offered":
    case "proposal_decided":
    case "proposal_withdrawn":
    case "session_named":
    case "run_policy_changed":
    case "skill_audited":
    case "skill_shelved":
    case "signal_landed":
    case "prefix_held":
      return [moved, null];
  }
}
