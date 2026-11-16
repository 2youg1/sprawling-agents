// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors


import type { Address, Carry } from "../wire";
import { cancel } from "./commands";
import type { given } from "./tags";
import { readTag } from "./tags";
import type { SlashCall, SlashHands } from "./slash_hands";

const CARRY = "--carry";


export function fresh(hands: SlashHands, call: SlashCall): void {
  if (hands.here === null) return;
  opened(hands, hands.here, call.words.includes(CARRY) ? "handoff" : "nothing");
}


function opened(hands: SlashHands, room: Address, carry: Carry): void {
  if (!hands.openSession(room, carry)) return;
  hands.write("");
}


export function compact(hands: SlashHands): void {
  const room = hands.here;
  if (room === null || hands.opening()) return;
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


export function retag(hands: SlashHands, call: SlashCall, change: typeof given): void {
  const tag = readTag(call.rest);
  const session = hands.tagged;
  if (tag === null || session === null) return;
  if (hands.retag(change([session], session, tag))) hands.write("");
}
