// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { fill, say } from "../../core/lang";
import type { Key, Lang } from "../../core/lang";
import { buildingOf } from "../../core/route";
import { count } from "../../core/time";
import type { Address, PrefixSegment, PrefixSlot } from "../../wire";

export interface ListenerLook {
  readonly key: PrefixSlot;
  readonly word: string;
  readonly who: string | undefined;
  readonly size: string | undefined;
  readonly cut: string | undefined;
  readonly sources: string | undefined;
}

export interface ListeningLook {
  readonly heading: string;
  readonly listeners: readonly ListenerLook[];
  readonly note: string | undefined;
}

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
    listeners: listeners.flatMap((listener) => {
      const told = heard.segments.find((each) => each.slot === listener.slot);
      if (listener.who === "" && told === undefined) return [];
      const cut =
        told === undefined ? 0 : told.sources.reduce((sum, source) => sum + source.dropped, 0);
      return {
        key: listener.slot,
        word: say(lang, listener.word),
        who: listener.who === "" ? undefined : listener.who,
        size:
          told === undefined
            ? undefined
            : fill(say(lang, "run_prompt_bytes"), { n: count(told.bytes) }),
        cut: cut > 0 ? fill(say(lang, "run_prompt_dropped"), { n: count(cut) }) : undefined,
        sources:
          told === undefined || told.sources.length === 0
            ? undefined
            : `${say(lang, "run_prompt_sources")} ${told.sources.map((source) => source.addr).join(" · ")}`,
      } satisfies ListenerLook;
    }),
    note: heard.told ? say(lang, "talk_listening_note") : undefined,
  };
}
