<!-- This Source Code Form is subject to the terms of the Mozilla Public
     License, v. 2.0. If a copy of the MPL was not distributed with this
     file, You can obtain one at https://mozilla.org/MPL/2.0/.
     Copyright (c) 2026 2youg1 and the sprawling contributors -->

<script lang="ts">
  // The chosen session's instrument sheet (client/Spec.lean §7K): one cell per
  // question a person running a coding agent asks of a session - which
  // model, how full its window is, what it read and wrote, how much of
  // that the cache served, what it cost against the whole city, how fast
  // the model answered, what bounds it and what its work must prove to
  // land. Every figure is read from an answer the page already asks; a
  // figure the city has not told the page is a dash or is left out, never
  // a guess.
  //
  // This is the one screen that draws cost (docs/frontend-method.md §7D): the session's
  // own spend and the city's beside it, so neither appears again anywhere
  // the panorama shows.
  import { readAnswer } from "../../core/answered";
  import { QUERIES } from "../../core/asking";
  import { fill, say } from "../../core/lang";
  import { kilo, usd } from "../../core/time";
  import { ui } from "../../ui";
  import type { Address, RoundsAnswer, RunId } from "../../wire";
  import { costReading, runSpend } from "../pricing";
  import Bounds from "../talk/bounds.svelte";
  import { contextOf, remindersOf, remindersSaid, usedPercent } from "../talk/gauge";
  import { frozenSaid } from "../talk/frozen";
  import { speedOf } from "./speed";

  interface Props {
    readonly here: Address;
    readonly run: RunId | null;
    readonly rounds: RoundsAnswer | undefined;
  }

  const { here, run, rounds }: Props = $props();

  const u = ui();
  const { lang } = u;
  const endpoints = u.conn.asking.ask(QUERIES.endpoints);
  const cost = u.conn.asking.ask(QUERIES.cost);
  const config = $derived(u.conn.asking.ask({ config: { addr: here } }));

  const DASH = "—";

  const served = $derived($endpoints !== undefined && "endpoints" in $endpoints ? $endpoints.endpoints.endpoints : []);
  const turns = $derived(rounds?.turns ?? []);
  const context = $derived(contextOf(turns, served, remindersOf($config)));
  // The four values the run was dispatched under, from its opening.
  const policy = $derived(rounds?.opening?.policy ?? null);

  // The model the session's latest turn asked, and what its endpoint
  // quotes for it, verbatim: a price is the provider's words.
  const model = $derived(turns.filter((turn) => typeof turn.model === "string").at(-1)?.model ?? null);
  const facts = $derived(served.flatMap((endpoint) => endpoint.models).find((row) => row.id === model));
  const price = $derived(
    typeof facts?.input_price === "string" && typeof facts.output_price === "string"
      ? fill(say($lang, "world_price"), { input: facts.input_price, output: facts.output_price })
      : "",
  );
  // What the model was frozen with besides its name: the effort its
  // requests froze and the mode the run was dispatched in, as its opening
  // records them - so a run that has not committed yet says its effort.
  const frozen = $derived(frozenSaid(rounds?.opening, $lang));

  // Every turn the provider reported tokens for, summed: the input is
  // every prompt token of each call, cached or not, so the share of it
  // the cache served is the hit rate.
  const used = $derived(
    turns.reduce(
      (sum, turn) => ({
        input: sum.input + (turn.used?.input ?? 0),
        output: sum.output + (turn.used?.output ?? 0),
        cached: sum.cached + (turn.used?.cached ?? 0),
        written: sum.written + (turn.used?.cache_write ?? 0),
        told: sum.told || (turn.used !== undefined && turn.used !== null),
        toldWritten: sum.toldWritten || (turn.used?.cache_write !== undefined && turn.used.cache_write !== null),
      }),
      { input: 0, output: 0, cached: 0, written: 0, told: false, toldWritten: false },
    ),
  );

  const read = $derived(readAnswer($cost, (held) => ("cost" in held ? held.cost : undefined)));
  const spend = $derived(runSpend(read, run));
  const spent = $derived.by((): string => {
    switch (spend.kind) {
      case "none":
        return DASH;
      case "unreadable":
        return say($lang, "world_unreadable");
      case "unpriced":
        return say($lang, "cost_none");
      case "spent":
        return usd(spend.micros);
    }
  });
  const city = $derived(
    read.kind === "held" && costReading(read.value).kind === "priced"
      ? fill(say($lang, "world_cost_city"), { usd: usd(read.value.total) })
      : "",
  );

  const speed = $derived(speedOf(turns));
</script>

<!-- Three cells to a row where the pane holds them, two where it does
not, so a figure is never cut to a few letters. -->
<div class="@container shrink-0">
<dl class="grid grid-flow-row-dense grid-cols-2 gap-x-gutter gap-y-pane border-y border-edge py-pane @min-[400px]:grid-cols-3">
  <div class="flex min-w-0 flex-col">
    <dt class="text-note text-text-faint">{say($lang, "world_model")}</dt>
    <dd class="truncate text-text">{model ?? DASH}</dd>
    {#if frozen !== ""}
      <dd class="truncate text-note text-text-faint">{frozen}</dd>
    {/if}
  </div>
  <div class="col-span-2 flex min-w-0 flex-col">
    <dt class="text-note text-text-faint">{say($lang, "ring_name")}</dt>
    {#if context === null}
      <dd class="text-text-faint">{DASH}</dd>
    {:else}
      <dd class="figure truncate text-text">
        {fill(say($lang, "ring_used"), { used: kilo(context.used), window: kilo(context.window) })}
        <span class="text-text-faint">· {fill(say($lang, "ring_share"), { n: String(usedPercent(context)) })}</span>
      </dd>
      <dd class="truncate text-note text-text-faint">{remindersSaid($lang, context).join(" · ")}</dd>
    {/if}
  </div>
  <div class="flex min-w-0 flex-col">
    <dt class="text-note text-text-faint">{say($lang, "world_tokens")}</dt>
    <dd class="figure truncate text-text">
      {used.told ? fill(say($lang, "world_tokens_io"), { input: kilo(used.input), output: kilo(used.output) }) : DASH}
    </dd>
    {#if price !== ""}
      <dd class="truncate text-note text-text-faint">{price}</dd>
    {/if}
  </div>
  <div class="flex min-w-0 flex-col">
    <dt class="text-note text-text-faint">{say($lang, "world_cache")}</dt>
    {#if used.input > 0}
      <dd class="figure truncate text-text">
        {fill(say($lang, "world_cache_hit"), { n: String(Math.round((used.cached / used.input) * 100)) })}
      </dd>
      <dd class="truncate text-note text-text-faint">
        {[
          fill(say($lang, "world_cache_read"), { n: kilo(used.cached) }),
          used.toldWritten ? fill(say($lang, "world_cache_write"), { n: kilo(used.written) }) : "",
        ]
          .filter((part) => part !== "")
          .join(" · ")}
      </dd>
    {:else}
      <dd class="text-text-faint">{DASH}</dd>
    {/if}
  </div>
  <div class="flex min-w-0 flex-col">
    <dt class="text-note text-text-faint">{say($lang, "world_cost")}</dt>
    <dd class="figure truncate text-text">{spent}</dd>
    {#if city !== ""}
      <dd class="truncate text-note text-text-faint">{city}</dd>
    {/if}
  </div>
  <div class="flex min-w-0 flex-col">
    <dt class="text-note text-text-faint">{say($lang, "world_speed")}</dt>
    {#if speed === null}
      <dd class="text-text-faint">{DASH}</dd>
    {:else}
      <dd class="figure truncate text-text">
        {fill(say($lang, "world_ttft_spread"), { median: String(speed.ttft), mean: String(speed.ttftMean) })}
      </dd>
      <dd class="truncate text-note text-text-faint">{fill(say($lang, "world_ttft_turns"), { n: String(speed.turns) })}</dd>
      {#if speed.tps !== null}
        <dd class="figure truncate text-text">
          {fill(say($lang, "world_tps_spread"), { p50: String(Math.round(speed.tps.p50)), p99: String(Math.round(speed.tps.p99)) })}
        </dd>
        <dd class="truncate text-note text-text-faint">{fill(say($lang, "world_tps_turns"), { n: String(speed.tps.turns) })}</dd>
      {/if}
    {/if}
  </div>
  <div class="flex min-w-0 flex-col">
    <dt class="text-note text-text-faint">{say($lang, "world_bounds")}</dt>
    <dd class="figure truncate text-text">
      {policy === null ? DASH : fill(say($lang, "world_write"), { limit: policy.write })}
    </dd>
    <dd class="-ml-snug flex min-w-0 flex-wrap items-center">
      <Bounds room={here} />
    </dd>
  </div>
  <div class="flex min-w-0 flex-col">
    <dt class="text-note text-text-faint">{say($lang, "world_admission")}</dt>
    {#if policy === null}
      <dd class="text-text-faint">{DASH}</dd>
    {:else}
      <dd class="figure truncate text-text">{policy.admit}</dd>
      <dd class="truncate text-note text-text-faint">{fill(say($lang, "world_landing"), { landing: policy.landing })}</dd>
    {/if}
  </div>
</dl>
</div>
