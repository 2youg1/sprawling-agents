// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { untrack } from "svelte";
import { get } from "svelte/store";

import { heldIn } from "../../core/belief/rooms";
import { stretchesOf } from "../../core/stretches";
import type { Stretch } from "../../core/stretches";
import type { Ui } from "../../ui";
import type { SessionsAnswer } from "../../wire";

export interface Stretches {
  readonly all: readonly Stretch[];
  readonly earlier: number;
}

export function askStretches(u: Ui): Stretches {
  const belief = u.conn.belief;
  let held = $state(get(belief));
  $effect(() =>
    belief.subscribe((now) => {
      held = now;
    }),
  );

  const rooms = $derived(
    [...held.rooms.keys()].flatMap((room) => {
      const addr = heldIn(held, room).at(-1)?.addr ?? null;
      return addr === null ? [] : [addr];
    }),
  );
  const roomsKey = $derived(rooms.join("\n"));

  let answers = $state.raw<Readonly<Record<string, SessionsAnswer>>>({});
  $effect(() => {
    const stops = (roomsKey === "" ? [] : untrack(() => rooms)).map((room) =>
      u.conn.asking.ask({ sessions: { room } }).subscribe((answer) => {
        if (answer !== undefined && "sessions" in answer)
          answers = { ...untrack(() => answers), [room]: answer.sessions };
      }),
    );
    return () => {
      for (const stop of stops) stop();
    };
  });

  const all = $derived(stretchesOf(Object.values(answers), (room) => heldIn(held, room)));
  const earlier = $derived(Object.values(answers).reduce((sum, answer) => sum + answer.earlier, 0));
  return {
    get all() {
      return all;
    },
    get earlier() {
      return earlier;
    },
  };
}
