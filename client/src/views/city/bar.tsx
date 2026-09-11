// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// One information bar instead of three corners: the city's name, where
// it stands, what waits for the person, what it has spent, and the one
// control that stops or releases it. Every fact is stated once and in
// one place.
//
// The legend below the drawing is the second half of the same job. A
// city drawn in glyphs that nothing names is a picture; a legend is a
// set of labels, which is what makes the picture readable.

import { For, Show } from "solid-js";

import { halt, release } from "../../core/commands";
import { MAYOR, toFragment } from "../../core/route";
import { usd } from "../../core/time";
import type { CityAnswer } from "../../wire";
import { useCommand, useSay, useUi } from "../../ui";
import { Badge } from "../parts/badge";
import { Button } from "../parts/button";

export function CityBar(props: { readonly city: CityAnswer | undefined }) {
  const ui = useUi();
  const say = useSay();
  const command = useCommand();
  const halted = () => ui.conn.belief.halted.includes("city");
  const approvals = ui.conn.asking.ask("approval_queue");
  const waiting = () => {
    const held = approvals();
    return held !== undefined && "approvals" in held ? held.approvals.items.length : 0;
  };
  const cost = ui.conn.asking.ask("cost_view");
  const spent = () => {
    const held = cost();
    return held !== undefined && "cost" in held ? held.cost.total : null;
  };
  const active = () => props.city?.active ?? 0;
  const standing = () => {
    if (halted()) return { text: say("city_stopped"), weight: "alert" } as const;
    if (active() > 0) return { text: say("city_active", { n: String(active()) }), weight: "live" } as const;
    return { text: say("city_quiet"), weight: "quiet" } as const;
  };

  return (
    <header
      class="flex flex-wrap items-center gap-base border-b border-g2 px-pane py-snug"
      aria-label={say("city_bar")}
    >
      <h1 class="text-title font-title">{ui.conn.belief.city ?? say("nav_city")}</h1>
      <Badge text={standing().text} weight={standing().weight} dot />
      <Show when={waiting() > 0}>
        <a
          href={toFragment({ kind: "talk", address: MAYOR })}
          class="rounded-pill bg-g2 px-snug py-tight text-note text-alert hover:bg-g3"
        >
          {say("nav_waiting", { n: String(waiting()) })}
        </a>
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
          command(halted() ? release("city") : halt("city"));
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
        <circle cx="8" cy="5" r="2.4" class="fill-g9" />
        <path d="M4.6 13 q3.4 -6 6.8 0 z" class="fill-g9" />
      </Show>
      <Show when={props.mark === "flag"}>
        <line x1="5" y1="2" x2="5" y2="14" class="stroke-g6" stroke-width="1.2" />
        <path d="M5 3 l7 1.8 l-7 2.2 z" class="fill-accent" />
      </Show>
      <Show when={props.mark === "lamp"}>
        <line x1="8" y1="6" x2="8" y2="14" class="stroke-g5" stroke-width="1.2" />
        <circle cx="8" cy="4.4" r="2.6" class="fill-alert" />
      </Show>
      <Show when={props.mark === "plinth"}>
        <rect x="1" y="6" width="14" height="4" rx="2" class="fill-g3" />
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
