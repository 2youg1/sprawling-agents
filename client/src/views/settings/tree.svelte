<!-- This Source Code Form is subject to the terms of the Mozilla Public
     License, v. 2.0. If a copy of the MPL was not distributed with this
     file, You can obtain one at https://mozilla.org/MPL/2.0/.
     Copyright (c) 2026 2youg1 and the sprawling contributors -->

<script lang="ts">
  // The settings tree, drawn: APG Disclosure Navigation (client-SPEC
  // 7-11). A branch is a label and never folds; a group entry is a
  // button that draws its group beside the tree; a page entry is a link,
  // so a middle click and a new tab work, and it carries an arrow because
  // following it leaves the panel; a nest is a button that opens its
  // own list. ↓/↑ walk the entries a person can see, Home/End jump to
  // the ends, and Tab still steps through them one by one.
  //
  // The performance entry carries the city's process reading in one
  // line (7D): while the tree is drawn this page asks the monitor for
  // its summary, and the city samples only while somebody watches.

  import { onDestroy } from "svelte";

  import { summary } from "../../core/monitor";
  import { LENSES, toFragment } from "../../core/route";
  import type { SetupGroup, View } from "../../core/route";
  import { say } from "../../core/lang";
  import { ui } from "../../ui";
  import Glyph from "../parts/glyph.svelte";
  import { HALL, useBuildings } from "../shared/buildings";
  import { HEADING } from "../setup/groups";
  import { TREE, nestOf } from "./tree";
  import type { Nest, Page } from "./tree";

  interface Props {
    // The group drawn in the body now.
    readonly group: SetupGroup;
    // The page under the panel: its entry says so, and its nest is open.
    readonly beneath: View;
    readonly onPick: (group: SetupGroup) => void;
  }

  const { group, beneath, onPick }: Props = $props();

  const u = ui();
  const { lang } = u;
  const buildings = useBuildings();
  const samples = u.conn.monitor.samples;
  onDestroy(u.conn.monitor.watchSummary());
  const reading = $derived.by(() => {
    const latest = $samples.at(-1);
    if (latest === undefined) return null;
    const read = summary(latest);
    return `${read.cpu} · ${read.memory}`;
  });

  const uid = $props.id();
  // svelte-ignore state_referenced_locally (the nest the panel opened over starts open; the person folds it after)
  const first = nestOf(beneath);
  const open = $state<Record<Nest, boolean>>({ buildings: first === "buildings", record: first === "record" });

  // The pages under each nest, read when the nest is drawn.
  const leaves = $derived<Record<Nest, readonly Page[]>>({
    buildings: $buildings
      .filter((addr) => addr !== HALL)
      .map((address) => ({ kind: "page", view: { kind: "building", address }, word: "settings_buildings" })),
    record: LENSES.map((lens) => ({ kind: "page", view: { kind: "record", lens }, word: `rec_${lens}` })),
  });

  // A building's entry is named by its address, which is the city's
  // word and not this page's.
  function wordOf(page: Page): string {
    return page.view.kind === "building" ? page.view.address : say($lang, page.word);
  }

  function here(view: View): boolean {
    return toFragment(view) === toFragment(beneath);
  }

  let nav = $state<HTMLElement | null>(null);

  function walk(event: KeyboardEvent): void {
    if (nav === null) return;
    const entries = [...nav.querySelectorAll<HTMLElement>("[data-entry]")];
    const at = entries.findIndex((each) => each === document.activeElement);
    const next = (() => {
      switch (event.key) {
        case "ArrowDown":
          return entries[Math.min(at + 1, entries.length - 1)];
        case "ArrowUp":
          return entries[Math.max(at - 1, 0)];
        case "Home":
          return entries[0];
        case "End":
          return entries.at(-1);
        default:
          return undefined;
      }
    })();
    if (next === undefined) return;
    event.preventDefault();
    next.focus();
  }

  const ENTRY =
    "relative flex min-h-control-sm w-full items-center gap-snug rounded-control px-snug text-left text-label " +
    "text-text-quiet hover:bg-raised hover:text-text";
  // Every row ends in one slot of the same width - the arrow of a page,
  // the chevron of a branch, or nothing - so every label is as wide as
  // the next and the rows' words start on one line.
  const MARK = "flex w-glyph-sm shrink-0 justify-center text-text-faint";
  // The 2px accent bar says which group is the one drawn (client-SPEC 7B).
  // The page under the panel is named too, but only in its ink: two bars
  // in one tree would leave the eye to guess which is the selection.
  const HERE = "bg-raised text-text before:absolute before:inset-y-snug before:left-0 before:w-[2px] before:rounded-pill before:bg-accent";
</script>

{#snippet leave(page: Page)}
  <a
    data-entry
    href={toFragment(page.view)}
    class={[ENTRY, here(page.view) && "text-text"]}
    aria-current={here(page.view) ? "page" : undefined}
  >
    <span class="min-w-0 flex-1 truncate">{wordOf(page)}</span>
    {#if page.view.kind === "monitor" && reading !== null}
      <span class="shrink-0 font-mono text-note text-text-faint">{reading}</span>
    {/if}
    <span class={MARK} aria-hidden="true">→</span>
  </a>
{/snippet}

<!-- svelte-ignore a11y_no_noninteractive_element_interactions (the arrow keys move between the links and buttons inside, the APG pattern for this navigation) -->
<nav
  bind:this={nav}
  class="flex flex-col gap-base"
  aria-label={say($lang, "settings_tree")}
  onkeydown={walk}
>
  {#each TREE as branch, index (branch.word)}
    <div class="flex flex-col gap-hair">
      <span id={`${uid}-${String(index)}`} class="px-snug pb-tight text-note tracking-wide text-text-faint uppercase">
        {say($lang, branch.word)}
      </span>
      <ul class="flex flex-col gap-hair" aria-labelledby={`${uid}-${String(index)}`}>
        {#each branch.entries as entry (entry.kind === "group" ? entry.group : entry.word)}
          <li>
            {#if entry.kind === "group"}
              <button
                type="button"
                data-entry
                class={[ENTRY, group === entry.group && HERE]}
                aria-current={group === entry.group ? "true" : undefined}
                onclick={() => {
                  onPick(entry.group);
                }}
              >
                <span class="min-w-0 flex-1 truncate">{say($lang, HEADING[entry.group])}</span>
                <span class={MARK} aria-hidden="true"></span>
              </button>
            {:else if entry.kind === "page"}
              <!-- eslint-disable-next-line @typescript-eslint/no-confusing-void-expression -->
              {@render leave(entry)}
            {:else if entry.kind === "nest"}
              <button
                type="button"
                data-entry
                class={ENTRY}
                aria-expanded={open[entry.nest]}
                aria-controls={`${uid}-${entry.nest}`}
                onclick={() => {
                  open[entry.nest] = !open[entry.nest];
                }}
              >
                <span class="min-w-0 flex-1 truncate">{say($lang, entry.word)}</span>
                <span class={MARK} aria-hidden="true">
                  <Glyph name="chevron" size="sm" class={["transition-transform", open[entry.nest] && "rotate-90"]} />
                </span>
              </button>
              {#if open[entry.nest]}
                <ul id={`${uid}-${entry.nest}`} class="ml-base flex flex-col gap-hair border-l border-edge pl-tight">
                  {#each leaves[entry.nest] as page (toFragment(page.view))}
                    <!-- eslint-disable-next-line @typescript-eslint/no-confusing-void-expression -->
                    <li>{@render leave(page)}</li>
                  {/each}
                </ul>
              {/if}
            {/if}
          </li>
        {/each}
      </ul>
    </div>
  {/each}
</nav>
