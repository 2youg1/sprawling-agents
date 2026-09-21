// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// One information bar instead of three corners: the city's name, six
// figures for how it stands, what it has spent, and the one control
// that stops or releases it. Every fact is stated once and in one
// place.
//
// **The six figures come from `Query::Metrics` and from nothing else.**
// Each of them is also provable from a view this page could have asked
// for separately - the approval queue, the city view, the recycle bin -
// and a bar that assembled them that way would hold six numbers under
// six refresh rules, which is how two figures on one line start
// disagreeing. That the city is stopped is not among them: the shell
// states it once, as a banner over every page.
//
// The legend below the drawing is the second half of the same job. A
// city drawn in glyphs that nothing names is a picture; a legend is a
// set of labels, which is what makes the picture readable.

import { For, Show } from "solid-js";

import { QUERIES } from "../../core/asking";
import { halt, release } from "../../core/commands";
import type { Key } from "../../core/lang";
import { MAYOR, toFragment } from "../../core/route";
import type { View } from "../../core/route";
import { cityIsShut, CITY } from "../../core/scope";
import { count, usd } from "../../core/time";
import type { MetricsAnswer } from "../../wire";
import { useCommand, useSay, useUi } from "../../ui";
import { Button } from "../parts/button";

// One figure on the bar: the word it is offered under, the field it
// reads, the page a person acts on it from, and whether a figure above
// zero is something waiting for them rather than something the city is
// getting on with.
interface Figure {
  readonly label: Key;
  readonly of: (held: MetricsAnswer) => number;
  readonly to: View | null;
  readonly waits: boolean;
}

// The six, in the order a person reads them: what the city has
// recorded, what is moving, what has stopped, and the three queues.
//
// `buildings` is the seventh field and is deliberately not here: the
// drawing under this bar and the list beside it are already the count
// of buildings, and a number repeating them would be a second home for
// one fact.
const FIGURES: readonly Figure[] = [
  { label: "metric_events", of: (held) => held.events, to: { kind: "record", lens: "ledger" }, waits: false },
  { label: "metric_runs_active", of: (held) => held.runs_active, to: null, waits: false },
  { label: "metric_runs_frozen", of: (held) => held.runs_frozen, to: null, waits: false },
  {
    label: "metric_approvals",
    of: (held) => held.approvals_waiting,
    to: { kind: "talk", address: MAYOR },
    waits: true,
  },
  { label: "metric_signals", of: (held) => held.signals_waiting, to: null, waits: true },
  {
    label: "metric_discards",
    of: (held) => held.discards_outstanding,
    to: { kind: "record", lens: "bin" },
    waits: true,
  },
];

// One figure and the word for it. A figure somebody has to answer is
// drawn in the alert tone only while it is above zero, so the bar of a
// city with nothing outstanding carries no red at all.
function Reading(props: { readonly figure: Figure; readonly metrics: MetricsAnswer }) {
  const say = useSay();
  const n = () => props.figure.of(props.metrics);
  const tone = () => (props.figure.waits && n() > 0 ? "text-alert" : "text-text");
  return (
    <div class="flex items-baseline gap-tight">
      <dt class="text-text-faint">{say(props.figure.label)}</dt>
      <dd class="font-mono">
        <Show when={props.figure.to} fallback={<span class={tone()}>{count(n())}</span>}>
          {(to) => (
            <a
              href={toFragment(to())}
              class={`underline decoration-g3 underline-offset-2 hover:decoration-accent ${tone()}`}
            >
              {count(n())}
            </a>
          )}
        </Show>
      </dd>
    </div>
  );
}

export function CityBar() {
  const ui = useUi();
  const say = useSay();
  const command = useCommand();
  const halted = () => cityIsShut(ui.conn.belief.halted);
  const asked = ui.conn.asking.ask(QUERIES.metrics);
  const metrics = () => {
    const held = asked();
    return held !== undefined && "metrics" in held ? held.metrics : undefined;
  };
  const cost = ui.conn.asking.ask(QUERIES.cost);
  const spent = () => {
    const held = cost();
    return held !== undefined && "cost" in held ? held.cost.total : null;
  };

  return (
    <header
      class="flex flex-wrap items-baseline gap-base border-b border-edge px-pane py-snug"
      aria-label={say("city_bar")}
    >
      <h1 class="text-title font-title">{ui.conn.belief.city ?? say("nav_city")}</h1>
      <Show when={metrics()}>
        {(held) => (
          <dl class="flex flex-wrap items-baseline gap-base text-note">
            <For each={FIGURES}>{(figure) => <Reading figure={figure} metrics={held()} />}</For>
          </dl>
        )}
      </Show>
      <Show when={spent()}>
        {(total) => (
          <a href={toFragment({ kind: "cost" })} class="text-note text-text-faint hover:text-text-quiet">
            {say("city_spent", { usd: usd(total()) })}
          </a>
        )}
      </Show>
      <span class="flex-1" />
      <Button
        label={halted() ? say("city_release") : say("city_stop")}
        tone={halted() ? "secondary" : "quiet"}
        onPress={() => {
          command(halted() ? release(CITY) : halt(CITY));
        }}
      />
    </header>
  );
}

// One glyph and the word for it. The drawing carries five marks, and a
// reader who has met none of them before is told what each one is.
const MARKS = ["window", "figure", "flag", "lamp", "plinth"] as const;
type Mark = (typeof MARKS)[number];

function Glyph(props: { readonly mark: Mark }) {
  return (
    <svg viewBox="0 0 16 16" class="size-glyph shrink-0" aria-hidden="true">
      <Show when={props.mark === "window"}>
        <rect x="5" y="3" width="6" height="8" rx="1" class="fill-accent-solid" />
      </Show>
      <Show when={props.mark === "figure"}>
        <circle cx="8" cy="5" r="2.4" class="fill-drawn-figure" />
        <path d="M4.6 13 q3.4 -6 6.8 0 z" class="fill-drawn-figure" />
      </Show>
      <Show when={props.mark === "flag"}>
        <line x1="5" y1="2" x2="5" y2="14" class="stroke-drawn-stem" stroke-width="1.2" />
        <path d="M5 3 l7 1.8 l-7 2.2 z" class="fill-accent" />
      </Show>
      <Show when={props.mark === "lamp"}>
        <line x1="8" y1="6" x2="8" y2="14" class="stroke-drawn-part" stroke-width="1.2" />
        <circle cx="8" cy="4.4" r="2.6" class="fill-alert" />
      </Show>
      <Show when={props.mark === "plinth"}>
        <rect x="1" y="6" width="14" height="4" rx="2" class="fill-drawn-line" />
        <rect x="1" y="6" width="8" height="4" rx="2" class="fill-accent" />
      </Show>
    </svg>
  );
}

export function CityLegend() {
  const say = useSay();
  return (
    <ul class="mx-auto mt-base flex max-w-page flex-wrap items-center justify-center gap-wide text-note text-text-faint">
      <For each={MARKS}>
        {(mark) => (
          <li class="flex items-center gap-tight">
            <Glyph mark={mark} />
            {say(`legend_${mark}`)}
          </li>
        )}
      </For>
    </ul>
  );
}
