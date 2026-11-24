// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

export type GlyphName =
  | "talk"
  | "city"
  | "record"
  | "cost"
  | "setup"
  | "app-grid"
  | "folder-tree"
  | "session-list"
  | "refresh"
  | "add"
  | "hand"
  | "search"
  | "chevron"
  | "go-back"
  | "reveal"
  | "ring"
  | "pulse"
  | "cross"
  | "check"
  | "tool"
  | "stop"
  | "pause"
  | "capped"
  | "send"
  | "layers"
  | "switch-subject"
  | "inbox"
  | "settings"
  | "gate"
  | "sandbox"
  | "terminal"
  | "propose"
  | "pin"
  | "more"
  | "branch"
  | "copy"
  | "door"
  | "key"
  | "mic"
  | "pdf-crop"
  | "pdf-magnify"
  | "pdf-original"
  | "pdf-brightness"
  | "pdf-details"
  | "pdf-reference";

export type Weight = "quiet" | "live" | "alert";

export type Status = "idle" | "live" | "waiting" | "refused" | "done";

export interface StatusLook {
  readonly glyph: GlyphName;

  readonly weight: Weight;
}

const STATUS: Record<Status, StatusLook> = {
  idle: { glyph: "ring", weight: "quiet" },
  live: { glyph: "pulse", weight: "live" },
  waiting: { glyph: "hand", weight: "alert" },
  refused: { glyph: "cross", weight: "alert" },
  done: { glyph: "check", weight: "quiet" },
};

export function statusLook(state: Status): StatusLook {
  return STATUS[state];
}

export type GlyphSize = "sm" | "md" | "key" | "stop";

export interface GlyphProps {
  readonly name: GlyphName;
  readonly size?: GlyphSize | undefined;
  readonly solid?: boolean | undefined;
  readonly class?: string | readonly (string | false | null | undefined)[] | undefined;
}
