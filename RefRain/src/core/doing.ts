// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import type { EventKind, ReplyEnd, Waiting } from "../wire";
import type { Key } from "./lang";

export type Doing =
  | { readonly kind: "unknown" }
  | { readonly kind: "thinking" }
  | { readonly kind: "calling"; readonly tool: string | null; readonly subject: string | null }
  | { readonly kind: "waiting" }
  | { readonly kind: "awaiting_reply"; readonly wait: Waiting }
  | { readonly kind: "frozen"; readonly completion: string | null };

export type Sending = "dispatch" | "steer" | "queued";

export function sendingInto(doing: Doing | undefined): Sending {
  if (doing === undefined) return "dispatch";
  switch (doing.kind) {
    case "frozen":
      return "dispatch";
    case "thinking":
      return "steer";

    case "unknown":
    case "calling":
    case "waiting":
    case "awaiting_reply":
      return "queued";
  }
}

export function afterWait(doing: Doing, end: ReplyEnd): Doing {
  if (doing.kind !== "awaiting_reply") return doing;
  switch (end) {
    case "reply":
    case "timeout":
      return { kind: "thinking" };
    case "left":
      return { kind: "unknown" };
  }
}

const MOVING = [
  "run_started",
  "model_called",
  "tool_result",
  "tool_called",
  "approval_requested",
  "run_frozen",
] as const;

type Moving = (typeof MOVING)[number];

export function moves(kind: EventKind): kind is Moving {
  return MOVING.some((each) => each === kind);
}

export const PHASES: Record<Moving, Doing> = {
  run_started: { kind: "thinking" },
  model_called: { kind: "thinking" },
  tool_result: { kind: "thinking" },
  approval_requested: { kind: "waiting" },
  tool_called: { kind: "calling", tool: null, subject: null },
  run_frozen: { kind: "frozen", completion: null },
};

export const POSTURE_WORD: Record<Doing["kind"], Key> = {
  unknown: "city_at_work",
  thinking: "run_doing_thinking",
  calling: "run_doing_calling",
  waiting: "run_doing_waiting",
  awaiting_reply: "run_doing_awaiting_reply",
  frozen: "run_doing_frozen",
};
