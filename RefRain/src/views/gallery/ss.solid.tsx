// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { For, onSettled } from "solid-js";
import { say } from "../../core/lang";
import type { Tier } from "../../core/prefs";
import { storeValue } from "../../core/solid_store";
import { ui } from "../../ui";
import { SessionList } from "../shell/session_list";
import SessionMenu from "../world/session_menu_view";
import { GalleryCase } from "./case";
import { pressing } from "./pressing";
import ShellSpecimen from "./shell_specimen";
import Stand from "./stand.solid";

import type {
  Answer,
  EventKind,
  EventRecord,
  Query,
  SessionLine,
  SessionTags,
  Turn,
} from "../../wire";
import { Address, B3Hash, RunId, Seq, Tag, TimeMs } from "../../wire";

const CITY = Address.make("sprawling");
const ROOM = Address.make("lab/room1");
export const PAST = Seq.make(10);
const run = (n: number): RunId =>
  RunId.make(`0199c0de-0000-4000-8000-5500000000${n.toString(16).padStart(2, "0")}`);

const RUNS: readonly (readonly [number, string, string, number, EventKind | null])[] = [
  [1, "hall/mayor", "plan the week", 2, "run_frozen"],
  [2, "lab/room1", "read the city's document contract", 4, "run_frozen"],
  [3, "lab/room1", "the parser's error positions", 10, "run_frozen"],
  [4, "lab/room2", "nextest coverage for the city", 12, "run_frozen"],
  [5, "shop/back", "rewrite the sieve's cut threshold", 14, "approval_requested"],
  [6, "hall/mayor", "what is left before the release", 20, "run_frozen"],
  [7, "lab/room1", "change the city's document reading contract", 30, null],
];

function record(
  n: number,
  seq: number,
  at: number,
  kind: EventKind,
  room: string,
  data: Record<string, unknown>,
): EventRecord {
  return {
    run: run(n),
    seq: Seq.make(seq),
    kind,
    t: TimeMs.make(at),
    who: "city",
    addr: Address.make(room),
    prev: B3Hash.make("0".repeat(64)),
    v: 1,
    data,
  };
}

export function recordsAt(now: number): readonly EventRecord[] {
  const named = {
    ...record(0, 0, now - 7_200_000, "city_initialized", "sprawling", {}),
    run: RunId.make("00000000-0000-0000-0000-000000000000"),
  };
  return [
    named,
    ...RUNS.flatMap(([n, room, task, seq, ending]) => {
      const at = now - (40 - seq) * 120_000;
      const started = record(n, seq, at, "run_started", room, { task });
      if (ending === null) return [started];
      const data =
        ending === "run_frozen"
          ? { completion: "done" }
          : { action_desc: "run cargo publish --dry-run" };
      return [started, record(n, seq + 1, at + 90_000, ending, room, data)];
    }),
  ];
}

const SAID: Readonly<Record<number, Partial<SessionLine>>> = {
  30: {
    name: "document contract",
    model: "claude-opus-4-1",
    effort: "high",
    workspace: "lab-1",
    preview:
      "The reading contract now names every field the document page draws, and the three it used to guess are asked of the city.",
  },
  10: {
    model: "claude-sonnet-4-5",
    effort: "medium",
    workspace: "lab-1",
    preview: "Error positions are byte offsets from the start of the input.",
  },
  12: { model: "gpt-5", workspace: "lab-2", preview: "Coverage is 81 % of lines." },
  14: {
    model: "claude-sonnet-4-5",
    effort: "low",
    preview: "Waiting for the User to allow the dry run.",
  },
};

function linesOf(room: string, now: number): SessionLine[] {
  return RUNS.filter(([, at]) => at === room)
    .map(([, , , seq], index): SessionLine => ({
      ...SAID[seq],
      began: Seq.make(seq),
      last: Seq.make(seq + 1),
      at: TimeMs.make(now - (40 - seq) * 120_000 + 90_000),
      runs: 1,
      start:
        index === 0
          ? { dispatched: { by: null } }
          : { opened: { carry: index === 1 ? "handoff" : "nothing", from: null } },
    }))
    .reverse();
}

const TAGS: readonly SessionTags[] = [
  { city: CITY, room: ROOM, began: PAST, tags: [Tag.make("parser"), Tag.make("pin")] },
  { city: CITY, room: Address.make("lab/room2"), began: Seq.make(12), tags: [Tag.make("later")] },
  {
    city: CITY,
    room: Address.make("shop/back"),
    began: Seq.make(14),
    tags: [Tag.make("parser")],
  },
  { city: CITY, room: Address.make("hall/mayor"), began: Seq.make(2), tags: [Tag.make("later")] },
];

function turn(n: number, said: string, now: number): Turn {
  return {
    calls: [],
    notes: [],
    number: 1,
    opened: Seq.make(n * 10),
    t: TimeMs.make(now),
    timing: "measured",
    said,
  };
}

export function answering(now: number): (query: Query) => Answer | undefined {
  return (query) => {
    if (query === "preferences") return { preferences: { tags: TAGS } };
    if (typeof query !== "object") return undefined;
    if ("sessions" in query)
      return {
        sessions: {
          room: query.sessions.room,
          sessions: linesOf(query.sessions.room, now),
          earlier: 0,
        },
      };
    if ("rounds" in query) {
      const found = RUNS.find(([n]) => run(n) === query.rounds.run);
      const said =
        found === undefined ? "" : `The work on ${found[2]} is written down in the room's Memo.md.`;
      return {
        rounds: {
          run: query.rounds.run,
          turns: [turn(found?.[0] ?? 0, said, now)],
          opened_at: null,
          opening: {
            at: TimeMs.make(now),
            goal: "",
            task: found?.[2] ?? "",
            policy: { mode: "work", write: "full", admit: "standing", landing: "ordinary" },
          },
          worktree: null,
        },
      };
    }
    return undefined;
  };
}

const MAINS: readonly [string, Tier][] = [
  ["sessions · an earlier session in main, blend, the way back to the current session", "blend"],
  [
    "sessions · an earlier session in main, panorama, the way back to the current session",
    "panorama",
  ],
];
const MENUS = [
  [
    "sessions · a pinned group, tags, and the pane filtered by one",
    '[role="group"] button:nth-of-type(4)',
    560,
  ],
  [
    "sessions · every session, the row menu open",
    'ul li:nth-of-type(2) button[aria-haspopup="menu"]',
    640,
  ],
  [
    "sessions · the current session's menu: rename, mode and write limit",
    'li:has(a[aria-current="page"]) button[aria-haspopup="menu"]',
    640,
  ],
] as const;

function ListSample(props: { readonly selector: string; readonly height: number }) {
  const lang = storeValue(ui().lang);
  let host: HTMLDivElement | undefined;
  onSettled(() => (host === undefined ? undefined : pressing(props.selector)(host)));
  return (
    <div
      class="flex flex-col"
      style={{ height: `${props.height}px` }}
      ref={(node) => {
        host = node;
      }}
    >
      <h2 class="flex h-control shrink-0 items-center text-note text-text-faint">
        {say(lang(), "world_sessions")}
      </h2>
      <SessionList here={ROOM} />
    </div>
  );
}

export default function SessionListCases() {
  const now = ui().now();
  const records = recordsAt(now);
  const answers = answering(now);
  const link = { kind: "live", city: "sprawling" } as const;
  return (
    <>
      <For each={MENUS}>
        {([label, selector, height]) => (
          <GalleryCase label={label} width={360}>
            <Stand link={link} unread={[]} waiting={[]} answers={answers} records={records}>
              {() => <ListSample selector={selector} height={height} />}
            </Stand>
          </GalleryCase>
        )}
      </For>
      <GalleryCase label="sessions · a row's menu key while the city has no name" width={360}>
        <div class="flex justify-end p-base">
          <SessionMenu
            named={null}
            label="room1"
            tags={[]}
            pinning="none"
            session={{ name: "", run: null }}
            read={null}
          />
        </div>
      </GalleryCase>
      <For each={MAINS}>
        {([label, tier]) => (
          <GalleryCase label={label} width={1440}>
            <Stand link={link} unread={[]} waiting={[]} answers={answers} records={records}>
              {() => <ShellSpecimen address={ROOM} session={PAST} tier={tier} height={760} />}
            </Stand>
          </GalleryCase>
        )}
      </For>
    </>
  );
}
