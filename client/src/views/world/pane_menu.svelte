<!-- This Source Code Form is subject to the terms of the Mozilla Public
     License, v. 2.0. If a copy of the MPL was not distributed with this
     file, You can obtain one at https://mozilla.org/MPL/2.0/.
     Copyright (c) 2026 2youg1 and the sprawling contributors -->

<script lang="ts">
  // A workbench pane's label, which is also the menu that moves the pane
  // (client-SPEC 7K, 7-11): APG Menu Button. Enter, Space or Down opens
  // it on its first item, Up and Down walk the items, Enter or Space
  // moves the pane, and Escape or Tab closes it with the focus back on
  // the label. The order is the person's and is kept in this browser
  // (`core/workbench.ts`, `prefs.ts`).
  import { tick } from "svelte";

  import { fill, say } from "../../core/lang";
  import { moved } from "../../core/workbench";
  import type { Pane, Side } from "../../core/workbench";
  import { ui } from "../../ui";

  interface Props {
    readonly pane: Pane;
    // The pane's name, already in the person's language.
    readonly label: string;
    // Whether the label is a control: in the blend tier the world layer
    // takes no input, and its labels are only words.
    readonly arranged: "menu" | "words";
  }

  const { pane, label, arranged }: Props = $props();

  const u = ui();
  const { lang } = u;
  const bench = u.prefs.workbench;
  const seat = $props.id();

  const at = $derived($bench.findIndex((column) => column.pane === pane));
  const SIDES: readonly Side[] = ["left", "right"];
  const items = $derived(
    SIDES.map((side) => ({
      side,
      word: say($lang, side === "left" ? "world_move_left" : "world_move_right"),
      possible: side === "left" ? at > 0 : at < $bench.length - 1,
    })),
  );

  let open = $state(false);
  let trigger = $state<HTMLButtonElement | undefined>(undefined);
  const entries: (HTMLButtonElement | undefined)[] = [];

  function show(): void {
    open = true;
    queueMicrotask(() => {
      entries.find((entry) => entry !== undefined && !entry.disabled)?.focus();
    });
  }

  // The focus goes back to the label once the page has redrawn: a move
  // re-seats the pane in the grid, and an element moved in the document
  // loses the focus it held.
  function close(): void {
    open = false;
    void tick().then(() => {
      trigger?.focus();
    });
  }

  function move(side: Side): void {
    u.prefs.setWorkbench(moved($bench, pane, side));
    close();
  }

  function walk(event: KeyboardEvent): void {
    const live = entries.filter((entry): entry is HTMLButtonElement => entry !== undefined && !entry.disabled);
    const now = live.findIndex((entry) => entry === document.activeElement);
    switch (event.key) {
      case "ArrowDown":
        event.preventDefault();
        live[(now + 1) % live.length]?.focus();
        return;
      case "ArrowUp":
        event.preventDefault();
        live[(now - 1 + live.length) % live.length]?.focus();
        return;
      case "Escape":
        event.preventDefault();
        close();
        return;
      case "Tab":
        open = false;
        return;
      default:
        return;
    }
  }
</script>

{#if arranged === "words"}
  <h2 class="flex h-control shrink-0 items-center text-note text-text-faint">{label}</h2>
{:else}
  <div class="relative flex h-control shrink-0 items-center">
    <h2 class="contents">
    <button
      bind:this={trigger}
      type="button"
      class="-mx-snug flex h-control-sm items-center rounded-control px-snug text-note text-text-faint hover:wash hover:text-text aria-expanded:wash aria-expanded:text-text"
      aria-haspopup="menu"
      aria-expanded={open}
      aria-controls="{seat}-menu"
      aria-label={fill(say($lang, "world_arrange"), { pane: label })}
      onclick={() => {
        if (open) close();
        else show();
      }}
      onkeydown={(event) => {
        if (event.key === "ArrowDown" && !open) {
          event.preventDefault();
          show();
        }
      }}
    >
      {label}
    </button>
    </h2>
    {#if open}
      <ul
        id="{seat}-menu"
        role="menu"
        tabindex="-1"
        aria-label={label}
        class="absolute top-full left-0 flex min-w-[16ch] flex-col rounded-card bg-raised p-tight shadow-float"
        onkeydown={walk}
        onfocusout={(event) => {
          if (!(event.relatedTarget instanceof Node && event.currentTarget.contains(event.relatedTarget))) open = false;
        }}
      >
        {#each items as item, index (item.side)}
          <li role="none">
            <button
              bind:this={entries[index]}
              type="button"
              role="menuitem"
              class="flex h-control-sm w-full items-center rounded-control px-snug text-left text-note text-text hover:wash focus-visible:wash disabled:text-text-disabled"
              disabled={!item.possible}
              onclick={() => {
                move(item.side);
              }}
            >
              {item.word}
            </button>
          </li>
        {/each}
      </ul>
    {/if}
  </div>
{/if}
