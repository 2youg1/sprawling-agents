<!-- This Source Code Form is subject to the terms of the Mozilla Public
License, v. 2.0. If a copy of the MPL was not distributed with this
file, You can obtain one at https://mozilla.org/MPL/2.0/.
Copyright (c) 2026 2youg1 and the sprawling contributors -->

<!-- How one tool call's line is drawn: `kind  subject  time  result` on a
four-column grid (client/Spec.lean §4-44), the whole line one button. The
line the right side holds is raised and carries the accent bar on its
left; any other line takes the row's wash under the pointer. The time
cell is at least nine characters and grows past that rather than spilling
over the subject, which truncates. -->
<script lang="ts">
  import { Kbd } from "../parts/kbd.svelte";
  import Tip from "../parts/tip.svelte";
  import type { CallLineLook } from "./call_line";
  import SteerPin from "./steer_pin.look.svelte";

  const look: CallLineLook = $props();
</script>

<Tip text={look.hint}>
  {#snippet children(id)}
    <button
      {...look.wire}
      aria-describedby={id}
      class={[
        "group relative grid h-control w-full grid-cols-[8ch_minmax(0,1fr)_minmax(9ch,max-content)_auto] items-center gap-x-pane rounded-control px-snug text-left text-note narrow:grid-cols-[6ch_minmax(0,1fr)_auto_auto] narrow:gap-x-snug",
        "before:absolute before:inset-y-snug before:left-0 before:w-hair before:rounded-pill",
        look.opened ? "bg-raised text-text before:bg-accent" : "text-text-quiet hover:wash",
      ]}
    >
      <span class="truncate text-text-faint">{look.kind}</span>
      <span class={["min-w-0 truncate", look.running ? "text-text" : ""]}>{look.subject}</span>
      <span class="figure flex items-center justify-end gap-tight whitespace-nowrap text-text-faint">
        {#if look.running}
          <span class="inline-block size-dot shrink-0 pulse rounded-pill bg-accent" aria-hidden="true"></span>
        {/if}
        {#if look.time.kind === "landed"}
          {look.time.text}
        {:else if look.time.kind === "running"}
          <span class="text-text">{look.time.text}</span>
        {/if}
      </span>
      <span class="flex items-center gap-tight">
        {#if look.verdict !== null}
          <span class={["whitespace-nowrap", look.verdict.tone === "alert" ? "text-alert" : "text-text-faint"]}>{look.verdict.text}</span>
        {/if}
        <span
          data-line-keys
          class={["items-center gap-hair narrow:hidden", look.opened ? "inline-flex" : "hidden group-focus-visible:inline-flex"]}
          aria-hidden="true"
        >
          {#each look.moves as move (move)}
            <!-- eslint-disable-next-line @typescript-eslint/no-confusing-void-expression, @typescript-eslint/no-unsafe-call (a snippet call is the render itself; typescript-eslint does not resolve exports of another .svelte module) -->
            {@render Kbd({ move })}
          {/each}
        </span>
        {#if look.pinned}
          <SteerPin />
        {/if}
      </span>
    </button>
  {/snippet}
</Tip>
