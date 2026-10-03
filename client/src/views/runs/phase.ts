// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// How a run's phase looks wherever a time bar draws it - the runs
// board and the city's building table - so the two bars are read with
// one legend.

import type { Doing } from "../../core/doing";
import { fill, say } from "../../core/lang";
import type { Key, Lang } from "../../core/lang";
import { lasted } from "../../core/time";
import type { GlyphName, Weight } from "../parts/glyph";
import { phaseOf } from "./lineage";
import type { Phase } from "./lineage";

export const PHASES: readonly Phase[] = ["model", "tool", "person", "reply", "idle", "done", "stopped", "capped"];

// The mark a run's row carries: each ending its own drawing, because a
// forced-colours mode repaints the ink and leaves only the shape.
export const PHASE_MARK: Record<Phase, { readonly glyph: GlyphName; readonly weight: Weight }> = {
  model: { glyph: "pulse", weight: "live" },
  tool: { glyph: "tool", weight: "live" },
  person: { glyph: "hand", weight: "alert" },
  reply: { glyph: "send", weight: "live" },
  idle: { glyph: "ring", weight: "quiet" },
  done: { glyph: "check", weight: "quiet" },
  stopped: { glyph: "stop", weight: "quiet" },
  capped: { glyph: "capped", weight: "alert" },
};

export const PHASE_FILL: Record<Phase, string> = {
  model: "bg-accent",
  tool: "bg-accent-solid",
  person: "bg-alert",
  reply: "bg-accent",
  idle: "bg-edge-input",
  done: "bg-text-disabled",
  stopped: "bg-text-disabled",
  capped: "bg-alert",
};

export const PHASE_WORD: Record<Phase, Key> = {
  model: "run_doing_thinking",
  tool: "run_doing_calling",
  person: "run_doing_waiting",
  reply: "run_doing_awaiting_reply",
  idle: "runs_phase_idle",
  done: "run_doing_frozen",
  stopped: "runs_phase_stopped",
  capped: "runs_phase_capped",
};

// A run's phase in words where a row has room for one phrase: the
// legend's word, except for a run waiting for a reply, which names the
// room it waits on and the time left before its deadline on the
// caller's clock (client/Spec.lean D88).
export function phaseSaid(lang: Lang, doing: Doing, now: number): string {
  if (doing.kind !== "awaiting_reply") return say(lang, PHASE_WORD[phaseOf(doing)]);
  const left = doing.wait.until - now;
  return left > 0
    ? fill(say(lang, "run_awaiting_reply_left"), { room: doing.wait.on, left: lasted(left) })
    : fill(say(lang, "run_awaiting_reply_due"), { room: doing.wait.on });
}
