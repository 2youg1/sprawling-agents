<!-- This Source Code Form is subject to the terms of the Mozilla Public
     License, v. 2.0. If a copy of the MPL was not distributed with this
     file, You can obtain one at https://mozilla.org/MPL/2.0/.
     Copyright (c) 2026 2youg1 and the sprawling contributors -->

<script lang="ts">
  // The settings tree, drawn: APG Disclosure Navigation (client/Spec.lean
  // §7-11). A branch is a button, and one branch is open at a time - the
  // one the panel stands in when it opens (client D53, the fold of
  // `client/spec/Views/Fold.lean`); a group entry is a button that draws
  // its group beside the tree; a page entry is a link, so a middle click
  // and a new tab work, and it carries an arrow because following it
  // leaves the panel; a nest and a branch's "more" are buttons that open
  // their own list. What opens arrives with `drop`; what folds goes at
  // once. ↓/↑ walk the entries a person can see, Home/End jump to
  // the ends, and Tab still steps through them one by one; the keys are
  // the line keys of `core/lines.ts`. A letter here is a first letter: it
  // moves to the next group that starts with it, wrapping at the end, and
  // each group draws its letter at the row's end (client D40), so
  // j, k, g and G do not walk this tree.
  //
  // The performance entry carries the city's process reading in one
  // line (docs/frontend-method.md §7D): while the tree is drawn this page asks the monitor for
  // its summary, and the city samples only while somebody watches.

  import { onDestroy } from "svelte";

  import { initialOf, initialTyped, lineWalker } from "../../core/lines";
  import { pressedOf } from "../../core/press";
  import { summary } from "../../core/monitor";
  import { LENSES, toFragment } from "../../core/route";
  import type { SetupGroup, View } from "../../core/route";
  import type { LineMove } from "../../core/lines";
  import { say } from "../../core/lang";
  import { ui } from "../../ui";
  import Glyph from "../parts/glyph.svelte";
  import { Kbd } from "../parts/kbd.svelte";
  import { useBuildings } from "../shared/buildings";
  import { HEADING } from "../setup/groups";
  import { TREE, buildingPages, nestOf, standingOf } from "./tree";
  import type { Entry, Leaf, Nest, Page } from "./tree";

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
  // svelte-ignore state_referenced_locally (the branch the panel opened in starts open; picking a group inside it keeps it so)
  const standing = standingOf(group, beneath);
  let branchOpen = $state<number | null>(standing.branch);
  let moreOpen = $state(standing.more);

  // One branch at a time: opening another folds the one that was open,
  // and its "more" with it; pressing the open one folds it.
  function toggleBranch(at: number): void {
    branchOpen = branchOpen === at ? null : at;
    moreOpen = false;
  }

  // The pages under each nest, read when the nest is drawn.
  const leaves = $derived<Record<Nest, readonly Page[]>>({
    buildings: buildingPages($buildings),
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

  const lines = lineWalker();

  // The letter each group is reached by, in the language on the page.
  function initial(named: SetupGroup): string {
    return initialOf(say($lang, HEADING[named]), named);
  }

  function walk(event: KeyboardEvent): void {
    if (nav === null) return;
    const pressed = pressedOf(event);
    const entries = [...nav.querySelectorAll<HTMLElement>("[data-entry]")];
    const at = entries.findIndex((each) => each === document.activeElement);
    const letter = initialTyped(pressed);
    const next = letter === null ? step(entries, at, lines(pressed, event.timeStamp)) : byInitial(entries, at, letter);
    if (next === undefined) return;
    event.preventDefault();
    next.focus();
  }

  function step(entries: readonly HTMLElement[], at: number, move: LineMove | null): HTMLElement | undefined {
    switch (move) {
      case "line.next":
        return entries[Math.min(at + 1, entries.length - 1)];
      case "line.previous":
        return entries[Math.max(at - 1, 0)];
      case "line.first":
        return entries[0];
      case "line.last":
        return entries.at(-1);
      case "line.open":
      case "line.close":
      case null:
        return undefined;
    }
  }

  // The next group after the focus whose letter this is, from the top
  // again past the last one.
  function byInitial(entries: readonly HTMLElement[], at: number, letter: string): HTMLElement | undefined {
    return [...entries.slice(at + 1), ...entries.slice(0, at + 1)].find((each) => each.dataset.initial === letter);
  }

  const ENTRY =
    "relative flex min-h-control-sm w-full items-center gap-snug rounded-control px-snug text-left text-label " +
    "text-text-quiet hover:bg-raised hover:text-text";
  // Every row ends in one slot of the same width - the arrow of a page,
  // the chevron of a branch, or nothing - so every label is as wide as
  // the next and the rows' words start on one line.
  const MARK = "flex w-glyph-sm shrink-0 justify-center text-text-faint";
  // The 2px accent bar says which group is the one drawn (docs/frontend-method.md §7B).
  // The page under the panel is named too, but only in its ink: two bars
  // in one tree would leave the eye to guess which is the selection.
  const HERE = "bg-raised text-text before:absolute before:inset-y-snug before:left-0 before:w-[2px] before:rounded-pill before:bg-accent";
  const keyOf = (entry: Entry): string => (entry.kind === "group" ? entry.group : entry.word);
</script>

{#snippet leave(page: Page)}
  <a
    data-entry
    href={toFragment(page.view)}
    class={[ENTRY, here(page.view) && "text-text"]}
    aria-current={here(page.view) ? "page" : undefined}
  >
    <span class="min-w-0 flex-1 truncate ps-pane">{wordOf(page)}</span>
    {#if page.view.kind === "monitor" && reading !== null}
      <span class="shrink-0 font-mono text-note text-text-faint">{reading}</span>
    {/if}
    <span class={MARK} aria-hidden="true">→</span>
  </a>
{/snippet}

{#snippet chevron(shown: boolean)}
  <span class={MARK} aria-hidden="true">
    <Glyph name="chevron" size="sm" class={["transition-transform motion-reduce:transition-none", shown && "rotate-90"]} />
  </span>
{/snippet}

{#snippet leaf(entry: Leaf)}
  {#if entry.kind === "group"}
    <button
      type="button"
      data-entry
      data-initial={initial(entry.group)}
      class={[ENTRY, group === entry.group && HERE]}
      aria-current={group === entry.group ? "true" : undefined}
      onclick={() => {
        onPick(entry.group);
      }}
    >
      <span class="min-w-0 flex-1 truncate ps-pane">{say($lang, HEADING[entry.group])}</span>
      <span class={MARK} aria-hidden="true">
        <!-- eslint-disable-next-line @typescript-eslint/no-confusing-void-expression, @typescript-eslint/no-unsafe-call (a snippet call is the render itself; typescript-eslint does not resolve exports of another .svelte module) -->
        {@render Kbd({ initial: initial(entry.group) })}
      </span>
    </button>
  {:else}
    <!-- eslint-disable-next-line @typescript-eslint/no-confusing-void-expression -->
    {@render leave(entry)}
  {/if}
{/snippet}

<!-- svelte-ignore a11y_no_noninteractive_element_interactions (the arrow keys move between the links and buttons inside, the APG pattern for this navigation) -->
<nav
  bind:this={nav}
  class="flex flex-col gap-hair"
  aria-label={say($lang, "settings_tree")}
  onkeydown={walk}
>
  {#each TREE as branch, index (branch.word)}
    <div class="flex flex-col gap-hair">
      <button
        type="button"
        data-entry
        class={[ENTRY, "text-note tracking-wide uppercase", branchOpen === index ? "text-text-quiet" : "text-text-faint"]}
        aria-expanded={branchOpen === index}
        aria-controls={`${uid}-${String(index)}`}
        onclick={() => {
          toggleBranch(index);
        }}
      >
        <span class="min-w-0 flex-1 truncate">{say($lang, branch.word)}</span>
        <!-- eslint-disable-next-line @typescript-eslint/no-confusing-void-expression -->
        {@render chevron(branchOpen === index)}
      </button>
      {#if branchOpen === index}
        <ul id={`${uid}-${String(index)}`} class="drop flex flex-col gap-hair pb-base">
          {#each branch.entries as entry (keyOf(entry))}
            <li>
              {#if entry.kind === "nest"}
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
                  <!-- eslint-disable-next-line @typescript-eslint/no-confusing-void-expression -->
                  {@render chevron(open[entry.nest])}
                </button>
                {#if open[entry.nest]}
                  <ul id={`${uid}-${entry.nest}`} class="drop ml-base flex flex-col gap-hair border-l border-edge pl-tight">
                    {#each leaves[entry.nest] as page (toFragment(page.view))}
                      <!-- eslint-disable-next-line @typescript-eslint/no-confusing-void-expression -->
                      <li>{@render leave(page)}</li>
                    {/each}
                  </ul>
                {/if}
              {:else}
                <!-- eslint-disable-next-line @typescript-eslint/no-confusing-void-expression -->
                {@render leaf(entry)}
              {/if}
            </li>
          {/each}
          {#if branch.more.length > 0}
            <li>
              <button
                type="button"
                data-entry
                class={ENTRY}
                aria-expanded={moreOpen}
                aria-controls={`${uid}-${String(index)}-more`}
                onclick={() => {
                  moreOpen = !moreOpen;
                }}
              >
                <span class="min-w-0 flex-1 truncate">{say($lang, "settings_more")}</span>
                <!-- eslint-disable-next-line @typescript-eslint/no-confusing-void-expression -->
                {@render chevron(moreOpen)}
              </button>
              {#if moreOpen}
                <ul id={`${uid}-${String(index)}-more`} class="drop ml-base flex flex-col gap-hair border-l border-edge pl-tight">
                  {#each branch.more as entry (keyOf(entry))}
                    <!-- eslint-disable-next-line @typescript-eslint/no-confusing-void-expression -->
                    <li>{@render leaf(entry)}</li>
                  {/each}
                </ul>
              {/if}
            </li>
          {/if}
        </ul>
      {/if}
    </div>
  {/each}
</nav>
