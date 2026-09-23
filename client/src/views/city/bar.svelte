<!-- This Source Code Form is subject to the terms of the Mozilla Public
     License, v. 2.0. If a copy of the MPL was not distributed with this
     file, You can obtain one at https://mozilla.org/MPL/2.0/.
     Copyright (c) 2026 2youg1 and the sprawling contributors -->

<script lang="ts">
  // The one information bar at the top of the city page: the city's
  // name, six figures for how it stands, what it has spent, and the one
  // control that stops or releases it. Every fact is stated once and in
  // one place.
  //
  // **The six figures come from `Query::Metrics` and from nothing
  // else.** Each of them is also provable from a view this page could
  // have asked for separately - the approval queue, the city view, the
  // recycle bin - and a bar that assembled them that way would hold six
  // numbers under six refresh rules, which is how two figures on one
  // line start disagreeing. That the city is stopped is not among them:
  // the shell states it once, as a banner over every page.

  import { QUERIES } from "../../core/asking";
  import { halt, release } from "../../core/commands";
  import type { Key } from "../../core/lang";
  import { fill, say } from "../../core/lang";
  import { MAYOR, toFragment } from "../../core/route";
  import type { View } from "../../core/route";
  import { cityIsShut, CITY } from "../../core/scope";
  import { count, usd } from "../../core/time";
  import { ui } from "../../ui";
  import type { MetricsAnswer } from "../../wire";
  import Button from "../parts/button.svelte";

  // One figure on the bar: the word it is offered under, the field it
  // reads, the page a person acts on it from, and whether a figure
  // above zero is something waiting for them rather than something the
  // city is getting on with.
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
  // of buildings, and a number repeating them would be a second home
  // for one fact.
  const FIGURES: readonly Figure[] = [
    {
      label: "metric_events",
      of: (held) => held.events,
      to: { kind: "record", lens: "ledger" },
      waits: false,
    },
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

  // One figure and the word for it, flattened where the question was
  // answered so the template draws strings and links and decides
  // nothing. A figure somebody has to answer is drawn in the alert tone
  // only while it is above zero, so the bar of a city with nothing
  // outstanding carries no red at all.
  interface Reading {
    readonly key: string;
    readonly label: string;
    readonly value: string;
    readonly href: string | null;
    readonly tone: string;
  }

  const u = ui();
  const lang = u.lang;
  const belief = u.conn.belief;
  const metrics = u.conn.asking.ask(QUERIES.metrics);
  const cost = u.conn.asking.ask(QUERIES.cost);

  const halted = $derived(cityIsShut($belief.halted));

  const readings = $derived.by((): Reading[] | null => {
    const held = $metrics;
    if (held === undefined || !("metrics" in held)) return null;
    return FIGURES.map((figure) => {
      const n = figure.of(held.metrics);
      return {
        key: figure.label,
        label: say($lang, figure.label),
        value: count(n),
        href: figure.to === null ? null : toFragment(figure.to),
        tone: figure.waits && n > 0 ? "text-alert" : "text-text",
      };
    });
  });

  const spent = $derived.by((): number | null => {
    const held = $cost;
    return held !== undefined && "cost" in held ? held.cost.total : null;
  });
</script>

<header
  class="flex flex-wrap items-baseline gap-base border-b border-edge px-pane py-snug"
  aria-label={say($lang, "city_bar")}
>
  <h1 tabindex="-1" class="text-title font-title">{$belief.city ?? say($lang, "nav_city")}</h1>
  {#if readings !== null}
    <dl class="flex flex-wrap items-baseline gap-base text-note">
      {#each readings as reading (reading.key)}
        <div class="flex items-baseline gap-tight">
          <dt class="text-text-faint">{reading.label}</dt>
          <dd class="font-mono">
            {#if reading.href !== null}
              <a
                href={reading.href}
                class={["underline decoration-edge-panel underline-offset-2 hover:decoration-accent", reading.tone]}
              >
                {reading.value}
              </a>
            {:else}
              <span class={reading.tone}>{reading.value}</span>
            {/if}
          </dd>
        </div>
      {/each}
    </dl>
  {/if}
  {#if spent !== null}
    <a href={toFragment({ kind: "cost" })} class="text-note text-text-faint hover:text-text-quiet">
      {fill(say($lang, "city_spent"), { usd: usd(spent) })}
    </a>
  {/if}
  <span class="flex-1"></span>
  <Button
    label={halted ? say($lang, "city_release") : say($lang, "city_stop")}
    tone={halted ? "secondary" : "quiet"}
    onPress={() => {
      u.send(halted ? release(CITY) : halt(CITY));
    }}
  />
</header>
