// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// What a run did, read two ways from one question.
//
// The fold over a wave of tool calls says "explored 7 files, wrote 1,
// ran 4 commands"; the artifact panel says "this is the file it last
// touched, and this is what the last command printed". Both readings
// ask the same thing of a call - what class of work is this - so the
// question is answered once, here, and neither reader owns a second
// table.
//
// **The numbers are already in the ledger.** A wave's calls arrive in
// `Turn.calls`, folded from the `tool_called` and `tool_result` events
// the city wrote; nothing here needs a new event, and a count kept
// beside the ledger would be a second authority for a number the
// ledger already holds.
//
// **The class is derived from the tool's name, which is all the wire
// carries.** `kernel::ToolMeta` states `effect` and `render` for every
// registered tool and is the authority for both; neither field is on
// the wire today. This table stands in for them and is deleted the day
// `Call` carries them (client-SPEC 4-26).

import type { Call, Turn } from "../../wire";

// What a call did, in the words the fold uses. `other` is every tool
// that governs, spawns, plans or reaches an outside server: real work,
// but not work a summary of file-and-command traffic can describe.
export type Deed = "explored" | "wrote" | "ran" | "other";

const DEEDS = new Map<string, Deed>([
  ["read", "explored"],
  ["search", "explored"],
  ["edit", "wrote"],
  ["exec", "ran"],
]);

export function deedOf(tool: string): Deed {
  return DEEDS.get(tool) ?? "other";
}

export interface Tally {
  readonly explored: number;
  readonly wrote: number;
  readonly ran: number;
  readonly other: number;
}

export function tally(calls: readonly Call[]): Tally {
  let explored = 0;
  let wrote = 0;
  let ran = 0;
  let other = 0;
  for (const call of calls) {
    switch (deedOf(call.tool)) {
      case "explored":
        explored += 1;
        break;
      case "wrote":
        wrote += 1;
        break;
      case "ran":
        ran += 1;
        break;
      case "other":
        other += 1;
        break;
    }
  }
  return { explored, wrote, ran, other };
}

// The three panes of the artifact card: the newest file a call read,
// the newest file a call changed, and the newest command output. Any
// of them may be absent, and a run that has produced none of the
// three gets no card at all.
//
// **Three because `Deed` already says three.** The panel used to fold
// `explored` and `wrote` into one `file`, so whichever happened last
// won and the other was not drawn - a run that read a header and then
// rewrote a module showed the header. Reading a file and changing one
// are different things to look at: one wants line numbers and the
// other wants to know what moved. The split costs no new question on
// the wire, because `deedOf` was already the authority that told them
// apart for the fold's counting sentence.
export interface Artifacts {
  readonly read: Call | null;
  readonly wrote: Call | null;
  readonly terminal: Call | null;
}

export const NOTHING: Artifacts = { read: null, wrote: null, terminal: null };

// Whether this call produced something worth showing: it answered, and
// it answered in words this build can read. A call still waiting is not
// an artifact, and neither is one whose result was too large to ride
// the wire - that one names a locator the run page opens.
function produced(call: Call): boolean {
  const output = call.output;
  return output !== null && output !== undefined && output.head !== "";
}

// The newest of each half, searched backwards from the last turn. The
// panel follows the run rather than a route: what it shows is the last
// thing this run produced, so a person watching a thread and a person
// opening it later see the same panel.
export function artifactsIn(turns: readonly Turn[]): Artifacts {
  let read: Call | null = null;
  let wrote: Call | null = null;
  let terminal: Call | null = null;
  for (let turn = turns.length - 1; turn >= 0; turn -= 1) {
    const calls = turns[turn]?.calls ?? [];
    for (let at = calls.length - 1; at >= 0; at -= 1) {
      const call = calls[at];
      if (call === undefined || !produced(call)) continue;
      switch (deedOf(call.tool)) {
        case "explored":
          read ??= call;
          break;
        case "wrote":
          wrote ??= call;
          break;
        case "ran":
          terminal ??= call;
          break;
        case "other":
          break;
      }
      if (read !== null && wrote !== null && terminal !== null) {
        return { read, wrote, terminal };
      }
    }
  }
  return { read, wrote, terminal };
}
