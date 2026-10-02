<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts" module>
  // The inspector's tab strip (docs/frontend-method.md §7F, client/Spec.lean §7-11): one tab per open
  // item, in the order they were opened, the one in front drawn on the
  // page's own fill so it reads as the sheet the region below is cut
  // from; a terminal's tab carries the terminal mark. At the right end,
  // the key that closes the whole inspector.
  //
  // An APG Tabs pattern with a roving tab stop: the strip is one stop,
  // ←/→ walk it and bring each tab forward, Home/End go to either end,
  // and Delete closes the tab under the focus. Each tab also has its own
  // close mark for a pointer; it is not a second Tab stop, because Delete
  // is that action's key.
</script>

<script lang="ts">
  import { tick } from "svelte";

  import { fill, say } from "../../core/lang";
  import { ui } from "../../ui";
  import Glyph from "../parts/glyph.svelte";
  import { itemKey, sameItem, type RightItem } from "./open.svelte";
  import type { Region, Tab } from "./reading";

  interface Props {
    readonly tabs: readonly Tab[];
    readonly front: RightItem | null;
    // Which region's panel each kind of tab controls.
    readonly panels: Readonly<Record<Region, string>>;
    readonly onPick: (item: RightItem) => void;
    readonly onClose: (item: RightItem) => void;
    readonly onCloseAll: () => void;
  }

  const { tabs, front, panels, onPick, onClose, onCloseAll }: Props = $props();

  const lang = ui().lang;

  let strip = $state<HTMLDivElement | undefined>(undefined);

  async function focusTab(at: number): Promise<void> {
    await tick();
    strip?.querySelectorAll<HTMLElement>('[role="tab"]')[at]?.focus();
  }

  function keys(event: KeyboardEvent, at: number): void {
    const last = tabs.length - 1;
    const to = ((): number | null => {
      switch (event.key) {
        case "ArrowLeft":
          return at === 0 ? last : at - 1;
        case "ArrowRight":
          return at === last ? 0 : at + 1;
        case "Home":
          return 0;
        case "End":
          return last;
        default:
          return null;
      }
    })();
    const here = tabs[at];
    if (event.key === "Delete" && here !== undefined) {
      event.preventDefault();
      onClose(here.item);
      void focusTab(Math.min(at, last - 1));
      return;
    }
    const there = to === null ? undefined : tabs[to];
    if (to === null || there === undefined) return;
    event.preventDefault();
    onPick(there.item);
    void focusTab(to);
  }
</script>

<div class="flex h-[calc(6*var(--spacing-baseline))] shrink-0 items-stretch border-b border-edge pr-snug text-note">
  <div bind:this={strip} class="flex min-w-0 flex-1 items-stretch overflow-x-auto" role="tablist" aria-label={say($lang, "inspect_tabs")}>
    {#each tabs as tab, at (itemKey(tab.item))}
      {@const on = sameItem(front, tab.item)}
      <div
        class={[
          "group/tab relative flex shrink-0 items-center gap-snug border-r border-edge pr-snug first:pl-wide",
          on ? "bg-page text-text after:absolute after:inset-x-0 after:-bottom-px after:h-px after:bg-page after:content-['']" : "pl-pane text-text-faint hover:text-text-quiet",
        ]}
      >
        <button
          type="button"
          role="tab"
          class="flex h-full max-w-[calc(24*var(--spacing-baseline))] items-center gap-snug pl-pane whitespace-nowrap group-first/tab:pl-0"
          aria-selected={on}
          aria-controls={panels[tab.region]}
          tabindex={on || (front === null && at === 0) ? 0 : -1}
          onclick={() => {
            onPick(tab.item);
          }}
          onkeydown={(event) => {
            keys(event, at);
          }}
        >
          {#if tab.region === "terminal"}<Glyph name="terminal" size="sm" />{/if}
          <span class="truncate">{tab.label}</span>
        </button>
        <button
          type="button"
          tabindex="-1"
          class={[
            "relative flex size-glyph-sm items-center justify-center rounded-control text-text-faint before:absolute before:-inset-tight before:content-[''] hover:bg-raised hover:text-text",
            on ? "" : "opacity-0 group-hover/tab:opacity-100",
          ]}
          aria-label={fill(say($lang, "inspect_close_item"), { name: tab.label })}
          onclick={() => {
            onClose(tab.item);
          }}
        >
          <Glyph name="cross" size="sm" />
        </button>
      </div>
    {/each}
  </div>
  <button
    type="button"
    class="my-auto ml-snug flex size-control-sm items-center justify-center rounded-control text-text-faint hover:bg-raised hover:text-text"
    aria-label={say($lang, "inspect_close")}
    onclick={onCloseAll}
  >
    <Glyph name="cross" />
  </button>
</div>
