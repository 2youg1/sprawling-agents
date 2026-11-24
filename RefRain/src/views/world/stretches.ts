// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import type { Belief } from "../../core/belief";
import { heldIn } from "../../core/belief/rooms";
import { readable, type Readable } from "../../core/store";
import { stretchesOf, type Stretch } from "../../core/stretches";
import type { Ui } from "../../ui";
import type { Address, SessionsAnswer } from "../../wire";

export interface Stretches {
  readonly all: readonly Stretch[];
  readonly earlier: number;
}

const empty: Stretches = { all: [], earlier: 0 };
const byCity = new WeakMap<Ui, Readable<Stretches>>();

export function askStretches(u: Ui): Readable<Stretches> {
  const found = byCity.get(u);
  if (found !== undefined) return found;
  const answer = readable(empty, (set) => {
    const asking = u.conn.asking;
    const stops = new Map<Address, () => void>();
    const answers = new Map<Address, SessionsAnswer>();
    let held: Belief | undefined;
    const report = () => {
      const snapshot = held;
      if (snapshot === undefined) return;
      const listed = [...answers.values()];
      set({
        all: stretchesOf(listed, (room) => heldIn(snapshot, room)),
        earlier: listed.reduce((sum, answer) => sum + answer.earlier, 0),
      });
    };
    const belief = u.conn.belief.subscribe((now) => {
      held = now;
      const rooms = new Set(
        [...held.rooms.keys()].flatMap((room) => {
          const address = heldIn(now, room).at(-1)?.addr;
          return address === undefined || address === null ? [] : [address];
        }),
      );
      for (const [room, stop] of stops) {
        if (rooms.has(room)) continue;
        stop();
        stops.delete(room);
        answers.delete(room);
      }
      for (const room of rooms) {
        if (stops.has(room)) continue;
        const stop = asking.ask({ sessions: { room } }).subscribe((answer) => {
          if (answer !== undefined && "sessions" in answer) {
            answers.set(room, answer.sessions);
            report();
          }
        });
        stops.set(room, stop);
      }
      report();
    });
    return () => {
      belief();
      for (const stop of stops.values()) stop();
    };
  });
  byCity.set(u, answer);
  return answer;
}
