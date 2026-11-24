// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { createMemo, createSignal, createUniqueId, For, Show } from "solid-js";
import { say } from "../../core/lang";
import { storeValue } from "../../core/solid_store";
import { ui } from "../../ui";
import Search from "../finder/search.solid";
import Back from "../pages/back.look.solid";
import { backLookOf } from "../pages/back";
import Palette, { PaletteBox } from "../palette.solid";
import { boxOf, carryPlaces, whyFor, type Asking } from "../palette";
import CommandDialog from "../palette/dialog";
import type { Entry, Listing } from "../palette/entry";
import { rowsLookOf } from "../palette/rows";
import { PaletteRows } from "../palette/rows_view";
import Button from "../parts/button.solid";
import Notice from "../parts/notice_view";
import { SwitcherFrame } from "../world/switcher_view";
import { switcherLookOf, type SwitchItem } from "../world/switcher";
import { GalleryCase } from "./case";
import Stand from "./stand.solid";

import { QUERIES } from "../../core/asking";
import { Address, Seq, TimeMs } from "../../wire";
import type { Answer, FindAnswer, Query, SessionLine } from "../../wire";
import { ENDPOINTS } from "./served";

const LAB = Address.make("lab");
const HALL = Address.make("hall/mayor");
const line = (
  began: number,
  name: string | null,
  preview: string | null,
  at: number,
): SessionLine => ({
  at: TimeMs.make(at),
  began: Seq.make(began),
  last: Seq.make(began + 9),
  runs: 2,
  name,
  preview,
  start: { opened: { carry: "nothing" } },
});
const NOW = Date.now();
const LINES: readonly SessionLine[] = [
  line(4182, "release notes", null, NOW),
  line(
    3990,
    null,
    "The ledger closes short by one entry on the first of the month.",
    NOW - 3_600_000,
  ),
  line(23, null, null, NOW - 86_400_000),
];

const FOUND: readonly Address[] = [
  "Roadmap.md",
  "notes/roadmap-review.md",
  "docs/archive/old-roadmap.txt",
].map((path) => Address.make(path));

function answering(query: Query): Answer | undefined {
  if (query === QUERIES.endpoints) return { endpoints: ENDPOINTS };
  if (typeof query === "object" && "find" in query) {
    const find: FindAnswer = {
      under: LAB,
      text: query.find.text,
      paths: [...FOUND],
      walked: "whole",
    };
    return { find };
  }
  if (typeof query === "object" && "sessions" in query) {
    return {
      sessions: {
        room: query.sessions.room,
        sessions: query.sessions.room === HALL ? [...LINES] : [],
        earlier: 0,
      },
    };
  }
  return undefined;
}
const SWITCH_ROWS: readonly SwitchItem[] = [
  {
    key: "hall/mayor:4182",
    label: "hall/mayor · release notes",
    hint: "current session",
    depth: 0,
    children: true,
    to: { kind: "talk", address: HALL },
  },
  {
    key: "lab/worker:3990",
    label: "lab/worker",
    hint: "1 hour ago",
    depth: 1,
    children: false,
    to: { kind: "talk", address: Address.make("lab/worker") },
  },
];

const ignore = (): void => undefined;
const reach = { here: null, live: false, models: 1 };
const verb = (spelling: string, about: string): Entry => ({
  label: spelling,
  hint: about,
  why: whyFor(spelling, reach),
  act: ignore,
});
const PALETTES = [
  { label: "palette · places, the cursor on the first row", here: undefined, initial: undefined },
  {
    label: "palette · opened from the address line: this room's sessions first",
    here: HALL,
    initial: undefined,
  },
  {
    label: "palette · a pasted address becomes one direct row",
    here: HALL,
    initial: "hall/new-room",
  },
] as const;
const WORKING = Address.make("lab/review");
const CARRIED = carryPlaces({
  here: Address.make("lab/parser"),
  rooms: ["lab/parser", "lab", "lab/review", "docs/notes"].map((room) => Address.make(room)),
  typed: Address.make("lab/lexer"),
  working: (room) => room === WORKING,
}).map((place): Entry => ({ label: place.to, hint: "", why: place.why, act: ignore }));

function PaletteSample(props: {
  readonly asking: Asking;
  readonly initial: string;
  readonly at: number;
  readonly listing: Listing;
  readonly entries: readonly Entry[];
}) {
  const lang = storeValue(ui().lang);
  const list = createUniqueId();
  const [query, write] = createSignal(props.initial);
  const [cursor, point] = createSignal(props.at);
  const box = createMemo(() =>
    boxOf(
      {
        asking: props.asking,
        query: query(),
        cursor: cursor(),
        count: props.entries.length,
        list,
        lang: lang(),
      },
      {
        write,
        point,
        pick: () => {
          props.entries[cursor()]?.act();
        },
      },
    ),
  );
  const rows = createMemo(() =>
    rowsLookOf(
      {
        listing: props.listing,
        shown: props.entries,
        cursor: cursor(),
        id: list,
        lang: lang(),
      },
      { hover: point, pick: (entry) => entry.act() },
    ),
  );
  return (
    <CommandDialog
      seat="specimen"
      named={{ "aria-label": say(lang(), "nav_palette") }}
      backdrop="stays"
      onClose={ignore}
    >
      <PaletteBox box={box()} />
      <PaletteRows look={rows()} />
    </CommandDialog>
  );
}

export default function EntranceCases() {
  const lang = storeValue(ui().lang);
  const actions = createMemo(() => [
    verb("/halt", say(lang(), "slash_halt")),
    verb("/stop", say(lang(), "slash_stop")),
  ]);
  const sessions = createMemo(() => [verb("/model", say(lang(), "slash_model"))]);
  const back = createMemo(() =>
    backLookOf({ kind: "building", address: Address.make("hall") }, lang()),
  );
  const [refused, setRefused] = createSignal(true);
  const [missing, setMissing] = createSignal(true);
  return (
    <>
      <For each={PALETTES}>
        {(sample) => (
          <GalleryCase label={sample.label} width={640}>
            <Stand
              link={{ kind: "live", city: "sprawling" }}
              unread={[]}
              waiting={[]}
              answers={answering}
            >
              {() => (
                <Palette
                  seat="specimen"
                  here={sample.here}
                  initial={sample.initial}
                  onClose={ignore}
                />
              )}
            </Stand>
          </GalleryCase>
        )}
      </For>
      <GalleryCase label="session switcher · a tree with a delegated child" width={640}>
        <CommandDialog
          seat="specimen"
          named={{ "aria-label": say(lang(), "world_session_switcher") }}
          backdrop="stays"
          onClose={ignore}
        >
          <SwitcherFrame
            {...switcherLookOf(
              {
                query: "",
                cursor: 0,
                collapsed: new Set(),
                rows: SWITCH_ROWS,
                list: "entrances-switcher",
                lang: lang(),
              },
              { write: ignore, move: ignore, fold: ignore, open: ignore, hover: ignore },
            )}
          />
        </CommandDialog>
      </GalleryCase>
      <GalleryCase
        label="palette · verbs, one this page cannot run greyed with its reason"
        width={640}
      >
        <PaletteSample
          asking="places"
          initial="/"
          at={1}
          entries={[...actions(), ...sessions()]}
          listing={{
            kind: "verbs",
            groups: [
              { section: "actions", entries: actions() },
              { section: "sessions", entries: sessions() },
            ],
          }}
        />
      </GalleryCase>
      <GalleryCase
        label="palette · carrying this conversation: the rooms it can go to, and why not"
        width={640}
      >
        <PaletteSample
          asking="carry"
          initial="lab/lexer"
          at={3}
          entries={CARRIED}
          listing={{ kind: "places" }}
        />
      </GalleryCase>
      <GalleryCase label="file finder · in its box, the cursor on the first path" width={640}>
        <Stand
          link={{ kind: "live", city: "sprawling" }}
          unread={[]}
          waiting={[]}
          answers={answering}
        >
          {() => (
            <CommandDialog
              seat="specimen"
              named={{ "aria-label": say(lang(), "finder_placeholder") }}
              backdrop="stays"
              onClose={ignore}
            >
              <Search under={LAB} titleId="entrances-finder" onClose={ignore} />
            </CommandDialog>
          )}
        </Stand>
      </GalleryCase>
      <GalleryCase label="back key · to the city, over a building page">
        <Show when={back()}>{(look) => <Back {...look()} />}</Show>
      </GalleryCase>
      <GalleryCase
        label="refusal · two toasts, a refusal and a stop with no run in front"
        width={480}
      >
        <ul class="stack flex flex-col items-stretch gap-snug">
          <Show when={refused()}>
            <li>
              <Notice
                seat="toast"
                weight="alert"
                action="cannot send to hall/mayor"
                code="E_PROVIDER"
                subject="the provider did not answer"
                recovery="check the provider on the settings page, then send again"
                actions={() => (
                  <Button
                    tone="quiet"
                    label={say(lang(), "dismiss")}
                    onPress={() => {
                      setRefused(false);
                    }}
                  />
                )}
              />
            </li>
          </Show>
          <Show when={missing()}>
            <li>
              <Notice
                seat="toast"
                weight="info"
                heading="no_run_in_front"
                next="stop_whole_city"
                actions={() => (
                  <Button
                    tone="quiet"
                    label={say(lang(), "dismiss")}
                    onPress={() => {
                      setMissing(false);
                    }}
                  />
                )}
              />
            </li>
          </Show>
        </ul>
      </GalleryCase>
    </>
  );
}
