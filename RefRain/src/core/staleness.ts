// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import type { EventKind } from "../wire";

export type Reach = "every" | "none" | "same_run" | "newest_page";

export function reaches(reach: Reach, key: string, run: string): boolean {
  switch (reach) {
    case "every":
      return true;
    case "none":
      return false;
    case "same_run":
      return key.includes(run);
    case "newest_page":
      return key.includes('"before":null');
  }
}

function reached(moved: boolean): Reach {
  return moved ? "every" : "none";
}

const PROVIDER_MOVED: ReadonlySet<EventKind> = new Set<EventKind>([
  "endpoint_attached",
  "endpoint_lost",
  "endpoint_probed",
  "model_selected",
  "login_started",
  "provider_degraded",
]);

const GOVERNANCE_MOVED: ReadonlySet<EventKind> = new Set<EventKind>([
  "approval_resolved",
  "autonomy_changed",
  "policy_created",
  "policy_revoked",
]);

const BUILDING_MOVED: ReadonlySet<EventKind> = new Set<EventKind>([
  "building_created",
  "building_configured",
  "building_removed",
  "roadmap_claimed",
  "roadmap_finished",
  "file_restored",
  "roadmap_released",
  "roadmap_split",
  "roadmap_blocked",
  "pursuit_changed",
  "checkpoint_committed",
  "handoff_written",
  "run_started",
  "run_frozen",
  "pr_merged",
  "asset_archived",
  "governed_document_written",
  "document_written",
  "rules_changed",
]);

const PROPOSALS_MOVED: ReadonlySet<EventKind> = new Set<EventKind>([
  "proposal_offered",
  "proposal_decided",
  "proposal_withdrawn",
  "document_written",
]);

const LAYERS_MOVED: ReadonlySet<EventKind> = new Set<EventKind>([
  "governed_document_written",
  "document_written",
  "checkpoint_committed",
  "file_restored",
]);

export function reachOf(name: string, kind: EventKind): Reach {
  switch (name) {
    case "city_view":
    case "metrics":
    case "history":
      return "every";
    case "rounds":
    case "evidence":
    case "run_view":
    case "run_history":
      return "same_run";
    case "approval_queue":
      return reached(kind === "approval_requested" || kind === "approval_resolved");
    case "governance":
      return reached(GOVERNANCE_MOVED.has(kind));
    case "endpoint_view":
    case "model_figures":
      return reached(PROVIDER_MOVED.has(kind));
    case "inbox_view":
      return reached(kind === "signal_enqueued" || kind === "signal_consumed");
    case "discard_view":
      return reached(kind === "file_discarded" || kind === "discard_restored");
    case "registry_view":
      return reached(kind === "asset_archived");
    case "cost_view":
    case "cost_of":
      return reached(kind === "model_returned" || kind === "roadmap_claimed");
    case "prefix":
    case "content":
    case "range":
    case "preview":
    case "reply":
    case "bytes":
    case "export":
      return "none";
    case "prompt_shape":
      return kind === "prompt_shape_compared" ? "same_run" : "none";
    case "skills":
      return reached(
        kind === "building_configured" || kind === "run_started" || kind === "rules_changed",
      );
    case "git_status":
      return reached(
        kind === "checkpoint_committed" || kind === "pr_merged" || kind === "file_restored",
      );
    case "commits":
      return kind === "checkpoint_committed" || kind === "pr_merged" ? "newest_page" : "none";
    case "building_view":
    case "listing":
    case "find":
    case "document":
    case "versions":
    case "archive_search":
      return reached(BUILDING_MOVED.has(kind));
    case "proposals":
      return reached(PROPOSALS_MOVED.has(kind));
    case "layers":
      return reached(LAYERS_MOVED.has(kind));
    case "open_proposals":
      return reached(
        kind === "proposal_offered" || kind === "proposal_decided" || kind === "proposal_withdrawn",
      );
    case "identity":
      return reached(kind === "governed_document_written");
    case "config":
      return reached(kind === "rules_changed" || kind === "building_configured");
    case "catalog":
      return reached(
        kind === "rules_changed" || kind === "building_configured" || kind === "run_frozen",
      );
    case "sessions":
      return reached(kind === "session_opened" || kind === "run_started" || kind === "run_frozen");
    case "skill_usage":
    case "mcp_usage":
    case "shells":
      return reached(kind === "run_frozen" || kind === "skill_audited");
    default:
      return "none";
  }
}
