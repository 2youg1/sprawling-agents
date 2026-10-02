<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts" module>
  // The city's runs board: the lineage tree of `sprawling view` with a
  // time bar on every run, the runs waiting for the person pinned above
  // it and leading the tree. The keys are the terminal's - the line keys
  // of `core/lines.ts` move (j and k, gg and G, the arrows, Home and End),
  // h and l fold - so a person who learned one learned both.
  //
  // **Only the rows on screen are drawn.** A city of a thousand runs is
  // a thousand rows; the list keeps the viewport's rows and a margin,
  // and pads the rest, so the cost of the board is its height.
  //
  // **The bar draws what the belief knows.** A run's life runs from its
  // start to its end (or now), and its phase is known only as it stands,
  // so the life is one quiet stretch and the phase is the cap at its
  // right-hand end.

  import type { BoardRun } from "./lineage";

  export interface RunsBoardProps {
    readonly runs: readonly BoardRun[];
    readonly now: number;
    readonly level?: 1 | 2;
  }
</script>

<script lang="ts">
  import { Option } from "effect";

  import { lineWalker } from "../../core/lines";
  import { pressedOf } from "../../core/press";
  import { fill, say } from "../../core/lang";
  import { SvelteSet } from "svelte/reactivity";

  import { ui } from "../../ui";
  import Glyph from "../parts/glyph.svelte";
  import { FOLDS, along } from "./fold";
  import type { Row } from "./lineage";
  import { phaseOf, rowsOf, viewOf, windowOf } from "./lineage";
  import { PHASES, PHASE_FILL as FILL, PHASE_MARK as MARK, PHASE_WORD as WORD } from "./phase";

  const { runs, now, level = 1 }: RunsBoardProps = $props();
  const u = ui();
  const { lang } = u;

  // Rows drawn beyond each edge of the viewport, so a fast scroll meets
  // drawn rows rather than padding.
  const MARGIN = 12;

  const INK = { quiet: "text-text-quiet", live: "text-accent", alert: "text-alert" } as const;

  const folded = new SvelteSet<string>();
  let scrolled = $state(0);
  let viewport = $state(0);
  let rowPx = $state(0);
  let list = $state<HTMLUListElement | undefined>();

  const rows = $derived(rowsOf(runs, folded));
  const waiting = $derived(runs.filter((run) => phaseOf(run.doing) === "person"));
  const shown = $derived(windowOf(rows.length, scrolled, viewport, rowPx, MARGIN));
  let cursorKey = $state<string | null>(null);
  const cursor = $derived(Math.max(0, rows.findIndex((row) => row.key === (cursorKey ?? waiting[0]?.run))));

  // Measured here rather than bound: a bound height observes the list,
  // and the padding that stands in for undrawn rows resizes it, which
  // is a resize loop the browser reports as an error.
  $effect(() => {
    if (list === undefined || rows.length === 0) return;
    if (rowPx === 0) rowPx = list.querySelector("li")?.getBoundingClientRect().height ?? 0;
    viewport = list.clientHeight;
  });

  function levelOf(row: Row): number {
    return row.kind === "building" ? 1 : row.kind === "room" ? 2 : 3;
  }

  function moveTo(at: number): void {
    const row = rows[Math.min(Math.max(at, 0), rows.length - 1)];
    if (row === undefined) return;
    cursorKey = row.key;
    const index = rows.indexOf(row);
    if (list !== undefined && rowPx > 0) {
      const top = index * rowPx;
      if (top < list.scrollTop) list.scrollTop = top;
      else if (top + rowPx > list.scrollTop + viewport) list.scrollTop = top + rowPx - viewport;
    }
  }

  function foldAt(key: string, shut: boolean): void {
    if (shut) folded.add(key);
    else folded.delete(key);
  }

  const walk = lineWalker();

  function pressed(event: KeyboardEvent): void {
    const row = rows[cursor];
    switch (walk(pressedOf(event), event.timeStamp)) {
      case "line.next":
        moveTo(cursor + 1);
        break;
      case "line.previous":
        moveTo(cursor - 1);
        break;
      case "line.first":
        moveTo(0);
        break;
      case "line.last":
        moveTo(rows.length - 1);
        break;
      case "line.open":
        if (row !== undefined) open(row);
        break;
      case "line.close":
      case null:
        folding(event, row);
        return;
    }
    event.preventDefault();
  }

  // h and l, and the side arrows, fold and unfold the row under the
  // cursor: the tree's own keys, not a list's.
  function folding(event: KeyboardEvent, row: Row | undefined): void {
    const openable = row !== undefined && row.kind !== "run";
    switch (event.key) {
      case "h":
      case "ArrowLeft":
        if (openable) foldAt(row.key, true);
        break;
      case "l":
      case "ArrowRight":
        if (openable) foldAt(row.key, false);
        break;
      default:
        return;
    }
    event.preventDefault();
  }

  // A row opens the page of what it names (S07 E36).
  function open(row: Row): void {
    Option.map(viewOf(row), u.go);
  }

  function pick(run: BoardRun): void {
    folded.delete((run.addr ?? "").split("/")[0] ?? "");
    folded.delete(run.addr ?? "");
    cursorKey = run.run;
    list?.focus();
  }

  // A run is titled by what the person asked of it, else by what
  // finishing looks like; one dispatched with neither written down is
  // named by where it works instead, so two such runs in one list are
  // still two different lines.
  function titleOf(run: BoardRun): string {
    const words = [run.task, run.goal].find((said) => said !== null && said.trim() !== "");
    if (words !== undefined && words !== null) return words;
    return run.addr === null ? say($lang, "runs_untitled") : fill(say($lang, "runs_untitled_in"), { addr: run.addr });
  }

  function bar(run: BoardRun): { readonly left: string; readonly width: string } {
    const from = along(run.started ?? now, now);
    const to = along(run.ended ?? now, now);
    return { left: `${from.toFixed(3)}%`, width: `${Math.max(to - from, 0).toFixed(3)}%` };
  }
</script>

<div class="flex min-w-0 flex-col gap-base">
  <header class="flex flex-wrap items-baseline gap-x-base gap-y-snug">
    <svelte:element this={level === 1 ? "h1" : "h2"} class="text-title font-title" tabindex="-1">{say($lang, "runs_title")}</svelte:element>
    <p class="figure text-note text-text-quiet">{fill(say($lang, "runs_counts"), { asking: String(waiting.length), runs: String(runs.length) })}</p>
  </header>

  {#if waiting.length > 0}
    <nav class="flex flex-wrap items-center gap-snug text-note" aria-label={say($lang, "runs_asking")}>
      <span class="text-alert">{say($lang, "runs_asking")}</span>
      {#each waiting as run (run.run)}
        <button type="button" class="inline-flex items-center gap-tight rounded-pill border border-edge-input px-snug font-mono text-text" onclick={() => { pick(run); }}>
          <Glyph name="hand" size="sm" class="text-alert" />{run.run.slice(0, 8)}
        </button>
      {/each}
    </nav>
  {/if}

  <ul class="flex flex-wrap gap-x-wide gap-y-tight text-note text-text-quiet" aria-label={say($lang, "runs_legend")}>
    {#each PHASES as phase (phase)}
      <li class="flex items-center gap-snug"><span class={["inline-block h-snug w-base rounded-pill", FILL[phase]]} aria-hidden="true"></span>{say($lang, WORD[phase])}</li>
    {/each}
  </ul>

  <div class="flex min-w-0 items-end gap-snug border-b border-edge pb-tight font-mono figure text-note text-text-quiet" aria-hidden="true">
    <span class="min-w-0 flex-1"></span>
    <span class="relative h-base w-[40%] shrink-0">
      {#each FOLDS.filter((fold) => fold.minutes > 0) as fold (fold.minutes)}
        <span class={["absolute top-0 border-l border-edge-input pl-tight", fold.at === 0 ? "" : "hidden @lg/page:inline"]} style:left="{String(fold.at)}%">{fill(say($lang, "runs_minus"), { n: String(fold.minutes) })}</span>
      {/each}
      <span class="absolute top-0 right-0 font-sans text-text">{say($lang, "runs_now")}</span>
    </span>
  </div>

  <ul
    bind:this={list}
    class="max-h-[70dvh] min-w-0 overflow-y-auto border-b border-edge"
    style:padding-top="{String(shown.from * rowPx)}px"
    style:padding-bottom="{String((rows.length - shown.to) * rowPx)}px"
    role="tree"
    tabindex="0"
    aria-label={say($lang, "runs_tree")}
    aria-activedescendant={rows[cursor] === undefined ? undefined : `runs-row-${rows[cursor].key}`}
    onscroll={(event) => { scrolled = event.currentTarget.scrollTop; viewport = event.currentTarget.clientHeight; }}
    onkeydown={pressed}
  >
    {#each rows.slice(shown.from, shown.to) as row, at (row.key)}
      {@const current = shown.from + at === cursor}
      <!-- svelte-ignore a11y_click_events_have_key_events (the tree owns the keys: Enter opens the row the keyboard holds, through `pressed`) -->
      <li
        id="runs-row-{row.key}"
        role="treeitem"
        aria-level={levelOf(row)}
        aria-expanded={row.kind === "run" ? undefined : !folded.has(row.key)}
        aria-selected={current}
        class={["flex h-step min-w-0 cursor-pointer items-center gap-snug pr-snug text-note hover:wash", current ? "bg-raised shadow-[inset_2px_0_0_var(--color-accent)]" : ""]}
        onclick={() => {
          cursorKey = row.key;
          open(row);
        }}
      >
        <span class="shrink-0 pl-snug font-mono whitespace-pre text-text-quiet" aria-hidden="true">{row.guide}</span>
        {#if row.kind === "building"}
          <span class="min-w-0 flex-1 truncate font-mono font-label text-text">{row.name}</span>
          <span class={["inline-flex shrink-0 items-center gap-tight font-mono figure", row.asking === 0 ? "text-text-quiet" : "text-alert"]}><Glyph name="hand" size="sm" />{String(row.asking)}</span>
          <span class="shrink-0 font-mono figure text-text-quiet">{String(row.runs)}</span>
        {:else if row.kind === "room"}
          <span class="min-w-0 flex-1 truncate font-mono text-text-quiet">{row.name}</span>
        {:else if row.kind === "run"}
          {@const phase = phaseOf(row.run.doing)}
          {@const look = MARK[phase]}
          {@const span = bar(row.run)}
          <Glyph name={look.glyph} size="sm" class={["shrink-0", INK[look.weight]]} />
          <span class="hidden shrink-0 font-mono text-text-quiet @lg/page:inline">{row.run.run.slice(0, 8)}</span>
          <span class="min-w-0 flex-1 truncate text-text">{titleOf(row.run)}</span>
          <span class={["hidden shrink-0 text-note @lg/page:inline", INK[look.weight]]}>{say($lang, WORD[phase])}</span>
          <span class="relative h-snug w-[40%] shrink-0" aria-hidden="true">
            <span class="absolute inset-y-0 rounded-pill bg-edge" style:left={span.left} style:width={span.width}></span>
            <span class={["absolute inset-y-0 w-snug rounded-pill", FILL[phase]]} style:left="clamp(0px, calc({span.left} + {span.width} - var(--spacing-snug)), calc(100% - var(--spacing-snug)))"></span>
          </span>
        {/if}
      </li>
    {/each}
  </ul>

  <p class="text-note text-text-quiet">{say($lang, "runs_keys")}</p>
</div>
