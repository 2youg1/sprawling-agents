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
  // end (client D40), only once the row is hovered or focused so the
  // tree stays a list of words rather than a wall of initials. What
  // opens arrives with `drop`; what folds goes at once. Every clickable
  // row starts with one mark box of the same size so the nav-column
  // geometry gate reads one x (card-S9.PANEL1): indent lives in the
  // label, never in a margin that moves the row. A closed nest never
  // reads as another branch.

  import Glyph from "../parts/glyph.svelte";
  import { Kbd } from "../parts/kbd.svelte";
  import type { GlyphName } from "../parts/glyph";
  import type { LeafLook, PageLook, TreeLook } from "./tree";

  const { nav, branches }: TreeLook = $props();

  const ENTRY =
    "group/entry relative flex min-h-control-sm w-full items-center gap-snug rounded-control px-snug text-left text-label " +
    "text-text-quiet transition-[background-color,color,scale] ease-leave hover:bg-raised-hover hover:text-text hover:ease-arrive " +
    "active:scale-(--scale-press) active:ease-arrive still:transition-none still:active:scale-100";
  // One box of the same size for every row's first mark - a branch glyph,
  // or an empty seat that keeps the centre - and the same box at the
  // end for the arrow, the chevron or the letter (docs/frontend-method.md
  // §7D geometry; card-S9.PANEL1).
  const MARK = "flex size-glyph-sm shrink-0 items-center justify-center text-text-faint";
  // The accent bar (`chosen`) says which group is the one drawn
  // (docs/frontend-method.md §7B). The page under the panel is named too,
  // but only in its ink: two bars in one tree would leave the eye to guess
  // which is the selection.
  const HERE = "chosen bg-raised text-text";
  // First-level entries share the branch title's first mark. Indent is
  // `ps-pane` on the label. Only a nest or "more" list adds a second
  // step of label padding; the list itself is not shifted.
  const CHILDREN = "drop flex flex-col gap-hair pb-base";
  const NESTED = "drop flex flex-col gap-hair";
  const DEPTH = "min-w-0 flex-1 truncate ps-pane";
  const DEEPER = "min-w-0 flex-1 truncate ps-[calc(var(--spacing-pane)*2)]";
</script>

{#snippet lead(mark: GlyphName | null)}
  <span class={MARK} aria-hidden="true">
    {#if mark !== null}
      <Glyph name={mark} size="sm" />
    {/if}
  </span>
{/snippet}

{#snippet page(each: PageLook, depth: "depth" | "deeper")}
  <a {...each.wire} class={[ENTRY, each.here && "text-text"]}>
    <!-- eslint-disable-next-line @typescript-eslint/no-confusing-void-expression -->
    {@render lead(null)}
    <span class={depth === "depth" ? DEPTH : DEEPER}>{each.label}</span>
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

{#snippet leaf(entry: LeafLook, depth: "depth" | "deeper")}
  {#if entry.kind === "group"}
    <button {...entry.wire} class={[ENTRY, entry.here && HERE]}>
      <!-- eslint-disable-next-line @typescript-eslint/no-confusing-void-expression -->
      {@render lead(null)}
      <span class={depth === "depth" ? DEPTH : DEEPER}>{entry.label}</span>
      <span
        class={[
          MARK,
          "opacity-0 transition-opacity ease-leave group-hover/entry:opacity-100 group-focus-within/entry:opacity-100",
          entry.here && "opacity-100",
        ]}
        aria-hidden="true"
      >
        <!-- eslint-disable-next-line @typescript-eslint/no-confusing-void-expression, @typescript-eslint/no-unsafe-call (a snippet call is the render itself; typescript-eslint does not resolve exports of another .svelte module) -->
        {@render Kbd({ initial: entry.initial })}
      </span>
    </button>
  {:else}
    <!-- eslint-disable-next-line @typescript-eslint/no-confusing-void-expression -->
    {@render page(entry, depth)}
  {/if}
{/snippet}

<!-- The arrow keys the wire bag carries move between the links and
  buttons inside: the APG pattern for this navigation. -->
<nav {...nav} class="flex flex-col gap-tight">
  {#each branches as branch (branch.key)}
    <div class="flex flex-col gap-hair">
      <button
        {...branch.fold.wire}
        class={[
          ENTRY,
          "text-note tracking-wide uppercase",
          branch.fold.open ? "text-text-quiet" : "text-text-faint",
        ]}
      >
        <!-- eslint-disable-next-line @typescript-eslint/no-confusing-void-expression -->
        {@render lead(branch.mark)}
        <span class="min-w-0 flex-1 truncate">{branch.fold.label}</span>
        <!-- eslint-disable-next-line @typescript-eslint/no-confusing-void-expression -->
        {@render chevron(branch.fold.open)}
      </button>
      {#if branch.fold.open}
        <ul id={branch.fold.list} class={CHILDREN}>
          {#each branch.entries as entry (entry.key)}
            <li>
              {#if entry.kind === "nest"}
                <button {...entry.fold.wire} class={[ENTRY, "text-text-quiet"]}>
                  <!-- eslint-disable-next-line @typescript-eslint/no-confusing-void-expression -->
                  {@render lead(null)}
                  <span class={DEPTH}>{entry.fold.label}</span>
                  <!-- eslint-disable-next-line @typescript-eslint/no-confusing-void-expression -->
                  {@render chevron(entry.fold.open)}
                </button>
                {#if entry.fold.open}
                  <ul id={entry.fold.list} class={NESTED}>
                    {#each entry.pages as each (each.key)}
                      <!-- eslint-disable-next-line @typescript-eslint/no-confusing-void-expression -->
                      <li>{@render page(each, "deeper")}</li>
                    {/each}
                  </ul>
                {/if}
              {:else}
                <!-- eslint-disable-next-line @typescript-eslint/no-confusing-void-expression -->
                {@render leaf(entry, "depth")}
              {/if}
            </li>
          {/each}
          {#if branch.more !== null}
            <li class="mt-hair">
              <button
                {...branch.more.fold.wire}
                class={[ENTRY, "text-note text-text-faint hover:text-text-quiet"]}
              >
                <!-- eslint-disable-next-line @typescript-eslint/no-confusing-void-expression -->
                {@render lead(null)}
                <span class={DEPTH}>{branch.more.fold.label}</span>
                <!-- eslint-disable-next-line @typescript-eslint/no-confusing-void-expression -->
                {@render chevron(branch.more.fold.open)}
              </button>
              {#if branch.more.fold.open}
                <ul id={branch.more.fold.list} class={NESTED}>
                  {#each branch.more.entries as entry (entry.key)}
                    <!-- eslint-disable-next-line @typescript-eslint/no-confusing-void-expression -->
                    <li>{@render leaf(entry, "deeper")}</li>
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
