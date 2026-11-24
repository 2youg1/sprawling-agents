// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { createMemo, For, Show } from "solid-js";
import { say } from "../../core/lang";
import { storeValue } from "../../core/solid_store";
import { ui } from "../../ui";
import type { PrefixAnswer, PromptShapeAnswer } from "../../wire";
import { B3Hash, RunId } from "../../wire";
import Glyph from "../parts/glyph.solid";
import { reportOf } from "../talk/context_report";
import { ContextReportLook as Report } from "../talk/context_report_view";
import { barOf, ringOf } from "../talk/gauge";
import { GaugeCheckpoint, GaugeRing } from "../talk/gauge_view";
import { Sparkline } from "../talk/head_view";
import { WearTrack } from "../talk/wear_view";
import { GalleryCase } from "./case";

import type { Context } from "../talk/gauge";
import type { Mark, Stretch, TrackWire } from "../talk/wear";

const WINDOW = 200_000;

const RINGS: readonly { readonly label: string; readonly context: Context }[] = [
  {
    label: "context ring · 41 % used, both reminders ahead",
    context: { used: 82_400, window: WINDOW, first: 30, second: 65 },
  },
  {
    label: "context ring · 88 % used, past the handoff",
    context: { used: 176_000, window: WINDOW, first: 30, second: 65 },
  },
  {
    label: "context ring · an older city that states no first reminder",
    context: { used: 30_000, window: WINDOW, first: null, second: 65 },
  },
];

const BARS: readonly Context[] = [
  { used: 24_000, window: WINDOW, first: 30, second: 65 },
  { used: 150_000, window: WINDOW, first: 30, second: 65 },
  { used: 90_000, window: WINDOW, first: 30, second: null },
];

const STEADY: readonly number[] = Array.from({ length: 20 }, (_, at) => 0.7 + (at % 3) * 0.1);
const STALLED: readonly number[] = [
  0.4, 0.5, 0.6, 0.5, 0.4, 0.5, 0, 0, 0, 0, 0, 0, 0.1, 1, 0.9, 0.8, 0.6, 0.5, 0.4, 0.3,
];

const MARKS: readonly Mark[] = [
  { top: 0.02, height: 0.04, phase: "model" },
  { top: 0.18, height: 0.06, phase: "tool" },
  { top: 0.31, height: 0.02, phase: "person" },
  { top: 0.47, height: 0.05, phase: "reply" },
  { top: 0.63, height: 0.08, phase: "tool" },
  { top: 0.995, height: 0.004, phase: "done" },
];
const READ: readonly Stretch[] = [
  [0, 0.34],
  [0.58, 0.12],
];
const THUMB: Stretch = [0.6, 0.1];
const IDLE: TrackWire = { onpointerdown: () => undefined, onpointermove: () => undefined };
const REPORT_RUN = RunId.make("00000000-0000-4000-8000-0000000000c1");
const REPORT_PREFIX: PrefixAnswer = {
  run: REPORT_RUN,
  segments: [
    {
      slot: "city",
      hash: B3Hash.make("1".repeat(64)),
      bytes: 1_640,
      text: "",
      stored: true,
      sources: [],
    },
    {
      slot: "building",
      hash: B3Hash.make("2".repeat(64)),
      bytes: 8_500,
      text: "",
      stored: true,
      sources: [],
    },
    {
      slot: "resident",
      hash: B3Hash.make("3".repeat(64)),
      bytes: 4_000,
      text: "",
      stored: true,
      sources: [],
    },
    {
      slot: "run",
      hash: B3Hash.make("4".repeat(64)),
      bytes: 1_100,
      text: "",
      stored: true,
      sources: [],
    },
  ],
  tools: {
    hash: B3Hash.make("5".repeat(64)),
    index: 0,
    len: 3_500,
    rows: [
      { name: "Shell", namespace: "core", exposure: "core", from: "default", hint: "", len: 700 },
      { name: "Grep", namespace: "core", exposure: "core", from: "default", hint: "", len: 560 },
    ],
  },
};
const REPORT_SHAPE: PromptShapeAnswer = {
  run: REPORT_RUN,
  tools: { bytes: 3_500, hash: B3Hash.make("5".repeat(64)) },
  conversation: { bytes: 188_600, hash: B3Hash.make("6".repeat(64)) },
  upper_bound: 207_340,
};

export default function RingCases() {
  const lang = storeValue(ui().lang);
  const report = createMemo(() =>
    reportOf(lang(), REPORT_RUN, REPORT_PREFIX, REPORT_SHAPE, {
      used: 211_100,
      window: 256_000,
      first: 30,
      second: 65,
    }),
  );
  const SendKey = () => (
    <span class="grid size-coin place-items-center text-text">
      <Glyph name="send" />
    </span>
  );
  return (
    <>
      <For each={RINGS}>
        {({ label, context }) => (
          <GalleryCase label={label} width={120}>
            <div class="flex justify-center py-wide">
              <GaugeRing ring={ringOf(lang(), context)} onOpen={null}>
                <SendKey />
              </GaugeRing>
            </div>
          </GalleryCase>
        )}
      </For>
      <GalleryCase
        label="context ring · a model that states no window draws the key alone"
        width={120}
      >
        <div class="flex justify-center py-wide">
          <GaugeRing ring={null} onOpen={null}>
            <SendKey />
          </GaugeRing>
        </div>
      </GalleryCase>
      <GalleryCase
        label="context bar · early, past the handoff, and with no handoff stated"
        width={300}
      >
        <div class="flex flex-col gap-base py-snug">
          <For each={BARS}>
            {(context) => (
              <Show when={barOf(lang(), context)}>
                {(bar) => (
                  <>
                    <span
                      class="relative block h-hair rounded-pill bg-edge-panel"
                      aria-hidden="true"
                    >
                      <span
                        class="absolute inset-y-0 left-0 rounded-pill bg-text-quiet transition-[width] duration-page ease-arrive"
                        style={{ width: `${String(bar().share)}%` }}
                      />
                      <Show when={bar().handoff}>
                        {(mark) => <GaugeCheckpoint {...mark()} along="line" />}
                      </Show>
                    </span>
                    <span class="sr-only">{bar().said}</span>
                  </>
                )}
              </Show>
            )}
          </For>
        </div>
      </GalleryCase>
      <GalleryCase
        label="context report · ring picks a bucket; detail unfolds under it"
        width={360}
      >
        <div class="h-[42rem] overflow-hidden rounded-panel border border-edge-panel bg-page">
          <Report look={report()} />
        </div>
      </GalleryCase>
      <GalleryCase label="arrival rhythm · a steady stream and one that stalled" width={300}>
        <div class="flex items-center gap-base text-note text-text-faint">
          <Sparkline rhythm={STEADY} />
          <Sparkline rhythm={STALLED} />
        </div>
      </GalleryCase>
      <GalleryCase label="read wear · every phase, two read stretches and the thumb" width={120}>
        <div class="relative h-output">
          <div class="absolute inset-y-0 right-0 w-snug">
            <WearTrack
              hint={say(lang(), "talk_wear_hint")}
              track={IDLE}
              read={READ}
              thumb={THUMB}
              marks={MARKS}
            />
          </div>
        </div>
      </GalleryCase>
    </>
  );
}
