<!-- This Source Code Form is subject to the terms of the Mozilla Public
     License, v. 2.0. If a copy of the MPL was not distributed with this
     file, You can obtain one at https://mozilla.org/MPL/2.0/.
     Copyright (c) 2026 2youg1 and the sprawling contributors -->

<script lang="ts">
  // The head of the world's sheet on one column (client/Spec.lean §4-52): the
  // back key at the top of the side the sheet came from, and one tab per
  // pane in the person's order, one pane shown at a time. The panes are
  // the workbench's own, mounted by `workspace.svelte` and laid out there,
  // so turning a phone or widening a window keeps them, their scroll and
  // their questions; that is why this is a tab list of its own and not
  // `parts/tabs.svelte`, which mounts each panel inside itself.
  //
  // APG Tabs with automatic activation, as `parts/tabs.svelte` reads it:
  // one stop on the way in, the arrows, Home and End once inside.
  import { say } from "../../core/lang";
  import type { Pane } from "../../core/workbench";
  import { ui } from "../../ui";
  import Glyph from "../parts/glyph.svelte";
  import { leaveSheet } from "../sheets.svelte";

  interface Props {
    // The panes in the person's order, each with the name its column
    // carries on a wide shell.
    readonly tabs: readonly { readonly pane: Pane; readonly label: string }[];
    readonly shown: Pane;
    // The prefix of the ids the panes carry, `<prefix>-<pane>`; a tab is
    // `<prefix>-tab-<pane>`, which its pane is labelled by.
    readonly prefix: string;
    readonly onShow: (pane: Pane) => void;
  }

  const { tabs, shown, prefix, onShow }: Props = $props();
  const { lang } = ui();

  const drawn: Partial<Record<Pane, HTMLButtonElement>> = {};

  function travel(event: KeyboardEvent): void {
    const at = tabs.findIndex((tab) => tab.pane === shown);
    const last = tabs.length - 1;
    const to = new Map([
      ["ArrowRight", at === last ? 0 : at + 1],
      ["ArrowLeft", at <= 0 ? last : at - 1],
      ["Home", 0],
      ["End", last],
    ]).get(event.key);
    const tab = to === undefined ? undefined : tabs[to];
    if (tab === undefined) return;
    event.preventDefault();
    onShow(tab.pane);
    drawn[tab.pane]?.focus();
  }
</script>

<div class="flex min-w-0 items-center gap-snug border-b border-edge">
  <button
    type="button"
    class="-ml-snug grid size-bar shrink-0 place-items-center rounded-control text-text-quiet hover:wash hover:text-text"
    aria-label={say($lang, "world_back")}
    onclick={() => {
      leaveSheet("world");
    }}
  >
    <Glyph name="chevron" class="rotate-180" />
  </button>
  <div class="flex min-w-0 items-stretch overflow-x-auto" role="tablist" aria-label={say($lang, "world_panes")}>
    {#each tabs as tab (tab.pane)}
      <button
        bind:this={drawn[tab.pane]}
        type="button"
        role="tab"
        id="{prefix}-tab-{tab.pane}"
        aria-controls="{prefix}-{tab.pane}"
        aria-selected={tab.pane === shown}
        tabindex={tab.pane === shown ? 0 : -1}
        class={[
          "flex h-bar items-center border-b-2 px-base text-label whitespace-nowrap",
          tab.pane === shown ? "border-accent text-text" : "border-transparent text-text-quiet hover:text-text",
        ]}
        onclick={() => {
          onShow(tab.pane);
        }}
        onkeydown={travel}
      >
        {tab.label}
      </button>
    {/each}
  </div>
</div>
