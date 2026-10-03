// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// What the verbs about the session in main reach for (`SessionHands`),
// read the moment a verb runs, for the two places that fill the hands:
// the box under a conversation and the Ctrl-K palette.

import { get } from "svelte/store";

import { onceFrozen } from "../../core/belief/live";
import type { SessionHands } from "../../core/slash_hands";
import { lineIn } from "../../core/stretches";
import { namedIn, tagsOf } from "../../core/tags";
import type { Ui } from "../../ui";
import type { Address, Seq } from "../../wire";

// `asked` is the stretch the address bar names, absent for the room's
// current one. The room's answer is the one the sessions pane already
// holds, so reading it here asks the city nothing new.
export function sessionHands(u: Ui, room: Address | null, asked?: Seq): SessionHands {
  const city = get(u.conn.belief).city;
  const answer = room === null ? undefined : get(u.conn.asking.ask({ sessions: { room } }));
  const line = lineIn(answer !== undefined && "sessions" in answer ? answer.sessions : undefined, asked);
  const named = room === null || line === null ? null : namedIn(city, room, line.began, line.workspace ?? null);
  return {
    tagged: named === null ? null : { city: named.city, room: named.room, began: named.began, tags: tagsOf(get(u.tags.held), named) },
    retag: u.tags.retag,
    whenFrozen: (run, then) => {
      onceFrozen(u.conn.belief, run, then);
    },
  };
}
