// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// Which held answers one ledger record makes stale. The verdict is
// taken per question name, not per held answer, so a record costs one
// judgement for each name a page holds, however many answers sit under
// it; only the two reaches that depend on the question's own subject
// look at its key.

import type { EventKind } from "../wire";

export type Reach = "every" | "none" | "same_run" | "newest_page";

// Whether an answer held under `key` is inside the reach of a record
// written by `run`.
export function reaches(reach: Reach, key: string, run: string): boolean {
  switch (reach) {
    case "every":
      return true;
    case "none":
      return false;
    case "same_run":
      return key.includes(run);
    // Only the newest page can grow: an older page is bounded above by
    // a seq already written, and a commit's lineage walks backwards
    // from the run that made it, so a later successor never changes it.
    case "newest_page":
      return key.includes("\"before\":null");
  }
}

function reached(moved: boolean): Reach {
  return moved ? "every" : "none";
}

// What moves a provider's standing, a decision, and a building's
// papers. Named rather than written out at the arm that reads them: a
// set has a name a reader can hold, and the arm then states which
// question it belongs to and nothing else.
const PROVIDER_MOVED: ReadonlySet<EventKind> = new Set<EventKind>([
  "endpoint_attached", "endpoint_lost", "endpoint_probed",
  "model_selected", "login_started", "provider_degraded",
]);

const GOVERNANCE_MOVED: ReadonlySet<EventKind> = new Set<EventKind>([
  "approval_resolved", "autonomy_changed", "policy_created", "policy_revoked",
]);

const BUILDING_MOVED: ReadonlySet<EventKind> = new Set<EventKind>([
  "building_created", "building_configured", "building_removed", "roadmap_claimed", "roadmap_finished",
  "roadmap_released", "roadmap_split", "roadmap_blocked", "pursuit_changed",
  "checkpoint_committed", "handoff_written", "run_started", "run_frozen",
  "pr_merged", "asset_archived", "governed_document_written", "document_written",
]);

// What moves the proposal cards open on a document: a card offered,
// decided or taken back, and a save that makes a card stale.
const PROPOSALS_MOVED: ReadonlySet<EventKind> = new Set<EventKind>([
  "proposal_offered", "proposal_decided", "proposal_withdrawn", "document_written",
]);

// How far one record of this kind reaches into the answers held under
// one question name.
export function reachOf(name: string, kind: EventKind): Reach {
  switch (name) {
    case "city_view": case "metrics": case "history":
      return "every";
    case "rounds": case "evidence": case "run_view": case "run_history":
      return "same_run";
    case "approval_queue":
      return reached(kind === "approval_requested" || kind === "approval_resolved");
    case "governance":
      return reached(GOVERNANCE_MOVED.has(kind));
    case "endpoint_view":
      return reached(PROVIDER_MOVED.has(kind));
    case "inbox_view":
      return reached(kind === "signal_enqueued" || kind === "signal_consumed");
    case "discard_view":
      return reached(kind === "file_discarded" || kind === "discard_restored");
    case "registry_view":
      return reached(kind === "asset_archived");
    case "cost_view": case "cost_of":
      return reached(kind === "model_returned" || kind === "roadmap_claimed");
    // A prompt is frozen once for the life of a run and the object
    // behind a hash never changes, so none of these answers can go
    // stale: a range and a preview are read from one stored document
    // version, and a reply's blocks from the text the question carries.
    case "prefix": case "content": case "range": case "preview": case "reply":
      return "none";
    // A shelf moves when somebody edits the building's rules, and a
    // pin appears when a run starts under them.
    case "skills":
      return reached(kind === "building_configured" || kind === "run_started");
    // The working tree moves whenever a wave writes, and every wave
    // ends in a checkpoint.
    case "git_status":
      return reached(kind === "checkpoint_committed" || kind === "pr_merged");
    case "commits":
      return kind === "checkpoint_committed" || kind === "pr_merged" ? "newest_page" : "none";
    case "building_view": case "listing": case "document": case "archive_search":
      return reached(BUILDING_MOVED.has(kind));
    case "proposals":
      return reached(PROPOSALS_MOVED.has(kind));
    default:
      return "none";
  }
}
