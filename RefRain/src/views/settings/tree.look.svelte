<!-- This Source Code Form is subject to the terms of the Mozilla Public
     License, v. 2.0. If a copy of the MPL was not distributed with this
     file, You can obtain one at https://mozilla.org/MPL/2.0/.
     Copyright (c) 2026 2youg1 and the sprawling contributors -->

<script lang="ts">
  import Glyph from "../parts/glyph.svelte";
  import { Kbd } from "../parts/kbd.svelte";
  import type { GlyphName } from "../parts/glyph";
  import type { LeafLook, PageLook, TreeLook } from "./tree";

  const { nav, branches }: TreeLook = $props();

  const ENTRY =
    "group/entry relative flex min-h-control-sm w-full items-center gap-snug rounded-control px-snug text-left text-label " +
    "text-text-quiet transition-[background-color,color,scale] ease-leave hover:bg-raised-hover hover:text-text hover:ease-arrive " +
    "active:scale-(--scale-press) active:ease-arrive still:transition-none still:active:scale-100";
  const MARK = "flex size-glyph-sm shrink-0 items-center justify-center text-text-faint";
  const HERE = "chosen bg-raised text-text";
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
  .chevron {
    transition: rotate var(--transition-duration-short) var(--ease-leave);
  }
  .chevron.shown {
    rotate: 90deg;
    transition-timing-function: var(--ease-arrive);
  }
</style>
