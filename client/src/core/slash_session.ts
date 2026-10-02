// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// What the session verbs of the slash table do: `/new` and `/clear`,
// `/compact`, `/tag` and `/untag`. Apart from `slash.ts` because these
// open sessions and change tags through the hands, while the table only
// spells and routes; `slash.ts` stays the one list of verbs.

import type { Address } from "../wire";
import { cancel, openSession } from "./commands";
import type { given } from "./tags";
import { readTag } from "./tags";
import type { SlashCall, SlashHands } from "./slash_hands";

const CARRY = "--carry";

// `/new` and `/clear` are one verb: a new session here, with nothing
// from the last one unless `--carry` says so. A room with no handoff
// answers `carried: false` rather than refusing, so the word passes on.
export function fresh(hands: SlashHands, call: SlashCall): void {
  if (hands.here === null) return;
  opened(hands, hands.here, call.words.includes(CARRY) ? "handoff" : "nothing");
}

// A new session at `room`, and main brought to it: sent from a past
// session in main, the new one is where the person is going.
function opened(hands: SlashHands, room: Address, carry: "nothing" | "handoff"): void {
  if (!hands.command(openSession(room, carry, null))) return;
  hands.go({ kind: "talk", address: room });
  hands.write("");
}

// `/compact` is `/new --carry`. A run still going is stopped first and
// the session opens once the belief holds it frozen: a frozen run has
// written the handoff, and no session opens under a working run.
export function compact(hands: SlashHands): void {
  const room = hands.here;
  if (room === null) return;
  const going = hands.live;
  if (going === null) {
    opened(hands, room, "handoff");
    return;
  }
  if (!hands.command(cancel(going.run))) return;
  hands.write("");
  hands.whenFrozen(going.run, () => {
    opened(hands, room, "handoff");
  });
}

// `/tag` and `/untag`: one word on or off the session in main. A word
// that is not a tag sends nothing and leaves the line to correct.
export function retag(hands: SlashHands, call: SlashCall, change: typeof given): void {
  const tag = readTag(call.rest);
  const session = hands.tagged;
  if (tag === null || session === null) return;
  if (hands.retag(change([session], session, tag))) hands.write("");
}
