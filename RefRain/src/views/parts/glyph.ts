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
  
  
  | "hand"
  
  | "search"
  
  | "chevron"
  
  
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
  
  
  | "mic";


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
