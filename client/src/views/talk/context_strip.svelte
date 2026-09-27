<!-- This Source Code Form is subject to the terms of the Mozilla Public
     License, v. 2.0. If a copy of the MPL was not distributed with this
     file, You can obtain one at https://mozilla.org/MPL/2.0/.
     Copyright (c) 2026 2youg1 and the sprawling contributors -->

<script lang="ts">
  // The line under the box that says who is listening: the city, the
  // building, the resident, and the run this conversation is on. Its
  // left end is a disclosure - a chevron that turns when it opens - and
  // what it opens is the four segments the run was actually sent, each
  // one folding on its own (`run/prompt.svelte`), because "who is
  // listening" is answered exactly by what they were told.
  //
  // A button with `aria-expanded` rather than `<details>`: the summary
  // line has to stay one row while the body opens under it, and a
  // `<summary>` drawn as a flex row loses its marker in some engines.
  // Enter and Space reach it the way they reach any button.
  import { fill, say } from "../../core/lang";
  import { untrack } from "svelte";

  import { buildingOf, roomOf } from "../../core/route";
  import { ui } from "../../ui";
  import type { Address, RunId } from "../../wire";
  import Glyph from "../parts/glyph.svelte";
  import Prompt from "../run/prompt.svelte";

  interface Props {
    readonly address: Address;
    readonly run: RunId | undefined;
    // The gallery draws it open so the segments are measured; every
    // screen starts closed.
    readonly starts?: "open" | "closed";
  }

  const { address, run, starts = "closed" }: Props = $props();

  const u = ui();
  const { lang } = u;
  const belief = u.conn.belief;
  const uid = $props.id();

  let open = $state(untrack(() => starts) === "open");

  const cells = $derived([
    { key: "slot_city", value: $belief.city ?? "—" },
    { key: "slot_building", value: buildingOf(address) },
    { key: "slot_resident", value: roomOf(address) },
  ] as const);
</script>

<div class="mt-snug text-note text-text-faint">
  <button
    type="button"
    class="flex w-full min-w-0 items-center gap-snug rounded-control px-tight py-tight text-left hover:bg-chrome hover:text-text-quiet"
    aria-expanded={open}
    aria-controls="{uid}-body"
    onclick={() => {
      open = !open;
    }}
  >
    <Glyph name="chevron" size="sm" class={["shrink-0", open ? "rotate-90" : ""]} />
    <span class="sr-only">{say($lang, "talk_context")}</span>
    {#each cells as cell (cell.key)}
      <span class="min-w-0 truncate">
        <span>{say($lang, cell.key)}</span>
        <span class="font-mono text-text-quiet">{cell.value}</span>
      </span>
      <span aria-hidden="true">·</span>
    {/each}
    <span class={["min-w-0 truncate", run === undefined ? "" : "font-mono"]}>
      {run === undefined ? say($lang, "talk_context_none") : fill(say($lang, "talk_context_run"), { run: run.slice(0, 8) })}
    </span>
  </button>
  {#if open}
    <div id="{uid}-body" class="mt-tight max-h-[40vh] overflow-y-auto rounded-card border border-edge-panel px-base">
      {#if run === undefined}
        <p class="py-snug">{say($lang, "talk_context_none")}</p>
      {:else}
        <Prompt {run} />
      {/if}
    </div>
  {/if}
</div>
