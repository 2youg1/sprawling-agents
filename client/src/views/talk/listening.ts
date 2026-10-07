// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// Who is listening in a room, as the words `listening.look.svelte` draws:
// the city, the building, the resident and the run, one row each in the
// order a prompt stacks their segments, and beside each what the room's
// newest run was told for it. The seat (`listening.svelte`) asks the
// city for the segments; this module only reads them, so the reading is
// testable without a component.

import { fill, say } from "../../core/lang";
import type { Key, Lang } from "../../core/lang";
import { buildingOf } from "../../core/route";
import { count } from "../../core/time";
import type { Address, PrefixSegment, PrefixSlot } from "../../wire";

export interface ListenerLook {
  // The slot, unique among the four; a key for `#each`.
  readonly key: PrefixSlot;
  // What listens: city, building, resident or run.
  readonly word: string;
  // Who it is, absent for the run, which has no name beyond the
  // conversation it already is.
  readonly who: string | undefined;
  // The size of the segment the run was told for it, absent when the
  // run was told none.
  readonly size: string | undefined;
  // What the budget cut off that segment, absent when nothing was.
  readonly cut: string | undefined;
  // The files the segment was read from, absent when it was read from
  // none.
  readonly sources: string | undefined;
}

// Everything the look is given.
export interface ListeningLook {
  readonly heading: string;
  readonly listeners: readonly ListenerLook[];
  // Where the full text is read, said only once a run has been told.
  readonly note: string | undefined;
}

// What the reading is drawn from.
export interface Heard {
  readonly city: string | null | undefined;
  readonly room: Address;
  readonly segments: readonly PrefixSegment[];
  readonly told: boolean;
}

interface Listener {
  readonly slot: PrefixSlot;
  readonly word: Key;
  readonly who: string;
}

export function listeningOf(lang: Lang, heard: Heard): ListeningLook {
  const listeners: readonly Listener[] = [
    { slot: "city", word: "slot_city", who: heard.city ?? "" },
    { slot: "building", word: "slot_building", who: buildingOf(heard.room) },
    { slot: "resident", word: "slot_resident", who: heard.room },
    { slot: "run", word: "slot_run", who: "" },
  ];
  return {
    heading: say(lang, "talk_listening"),
    listeners: listeners.map((listener) => {
      const told = heard.segments.find((each) => each.slot === listener.slot);
      const cut = told === undefined ? 0 : told.sources.reduce((sum, source) => sum + source.dropped, 0);
      return {
        key: listener.slot,
        word: say(lang, listener.word),
        who: listener.who === "" ? undefined : listener.who,
        size: told === undefined ? undefined : fill(say(lang, "run_prompt_bytes"), { n: count(told.bytes) }),
        cut: cut > 0 ? fill(say(lang, "run_prompt_dropped"), { n: count(cut) }) : undefined,
        sources:
          told === undefined || told.sources.length === 0
            ? undefined
            : `${say(lang, "run_prompt_sources")} ${told.sources.map((source) => source.addr).join(" · ")}`,
      };
    }),
    note: heard.told ? say(lang, "talk_listening_note") : undefined,
  };
}
