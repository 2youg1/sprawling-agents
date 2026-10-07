<!-- This Source Code Form is subject to the terms of the Mozilla Public
     License, v. 2.0. If a copy of the MPL was not distributed with this
     file, You can obtain one at https://mozilla.org/MPL/2.0/.
     Copyright (c) 2026 2youg1 and the sprawling contributors -->

<script lang="ts">
  // The settings tree, the shipped look: markup and style only. Every
  // word, state and handler arrives as `TreeLook` from the seat
  // (`tree.svelte`), and every wire bag is spread whole on the element
  // it belongs to.
  //
  // A page entry carries an arrow because following it leaves the
  // panel; a group entry draws the letter it is reached by at the row's
  // end (client D40). What opens arrives with `drop`; what folds goes at
  // once.

  import Glyph from "../parts/glyph.svelte";
  import { Kbd } from "../parts/kbd.svelte";
  import type { LeafLook, PageLook, TreeLook } from "./tree";

  const { nav, branches }: TreeLook = $props();

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
  const NESTED = "drop ml-base flex flex-col gap-hair border-l border-edge pl-tight";
</script>

{#snippet page(each: PageLook)}
  <a {...each.wire} class={[ENTRY, each.here && "text-text"]}>
    <span class="min-w-0 flex-1 truncate ps-pane">{each.label}</span>
    {#if each.reading !== null}
      <span class="shrink-0 font-mono text-note text-text-faint">{each.reading}</span>
    {/if}
    <span class={MARK} aria-hidden="true">→</span>
  </a>
{/snippet}

{#snippet chevron(shown: boolean)}
  <span class={[MARK, "chevron"]} class:shown aria-hidden="true">
    <Glyph name="chevron" size="sm" />
  </span>
{/snippet}

{#snippet leaf(entry: LeafLook)}
  {#if entry.kind === "group"}
    <button {...entry.wire} class={[ENTRY, entry.here && HERE]}>
      <span class="min-w-0 flex-1 truncate ps-pane">{entry.label}</span>
      <span class={MARK} aria-hidden="true">
        <!-- eslint-disable-next-line @typescript-eslint/no-confusing-void-expression, @typescript-eslint/no-unsafe-call (a snippet call is the render itself; typescript-eslint does not resolve exports of another .svelte module) -->
        {@render Kbd({ initial: entry.initial })}
      </span>
    </button>
  {:else}
    <!-- eslint-disable-next-line @typescript-eslint/no-confusing-void-expression -->
    {@render page(entry)}
  {/if}
{/snippet}

<!-- The arrow keys the wire bag carries move between the links and
  buttons inside: the APG pattern for this navigation. -->
<nav {...nav} class="flex flex-col gap-hair">
  {#each branches as branch (branch.key)}
    <div class="flex flex-col gap-hair">
      <button
        {...branch.fold.wire}
        class={[ENTRY, "text-note tracking-wide uppercase", branch.fold.open ? "text-text-quiet" : "text-text-faint"]}
      >
        <span class="min-w-0 flex-1 truncate">{branch.fold.label}</span>
        <!-- eslint-disable-next-line @typescript-eslint/no-confusing-void-expression -->
        {@render chevron(branch.fold.open)}
      </button>
      {#if branch.fold.open}
        <ul id={branch.fold.list} class="drop flex flex-col gap-hair pb-base">
          {#each branch.entries as entry (entry.key)}
            <li>
              {#if entry.kind === "nest"}
                <button {...entry.fold.wire} class={ENTRY}>
                  <span class="min-w-0 flex-1 truncate">{entry.fold.label}</span>
                  <!-- eslint-disable-next-line @typescript-eslint/no-confusing-void-expression -->
                  {@render chevron(entry.fold.open)}
                </button>
                {#if entry.fold.open}
                  <ul id={entry.fold.list} class={NESTED}>
                    {#each entry.pages as each (each.key)}
                      <!-- eslint-disable-next-line @typescript-eslint/no-confusing-void-expression -->
                      <li>{@render page(each)}</li>
                    {/each}
                  </ul>
                {/if}
              {:else}
                <!-- eslint-disable-next-line @typescript-eslint/no-confusing-void-expression -->
                {@render leaf(entry)}
              {/if}
            </li>
          {/each}
          {#if branch.more !== null}
            <li>
              <button {...branch.more.fold.wire} class={ENTRY}>
                <span class="min-w-0 flex-1 truncate">{branch.more.fold.label}</span>
                <!-- eslint-disable-next-line @typescript-eslint/no-confusing-void-expression -->
                {@render chevron(branch.more.fold.open)}
              </button>
              {#if branch.more.fold.open}
                <ul id={branch.more.fold.list} class={NESTED}>
                  {#each branch.more.entries as entry (entry.key)}
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

<style>
  /* The chevron turns down as its list opens and back as it folds: the
   * opening arrives, the folding leaves (docs/frontend-method.md §4-43).
   * Motion off zeroes the duration token, so the chevron then turns at
   * once. */
  .chevron {
    transition: rotate var(--transition-duration-short) var(--ease-leave);
  }
  .chevron.shown {
    rotate: 90deg;
    transition-timing-function: var(--ease-arrive);
  }
</style>
