<!-- This Source Code Form is subject to the terms of the Mozilla Public
     License, v. 2.0. If a copy of the MPL was not distributed with this
     file, You can obtain one at https://mozilla.org/MPL/2.0/.
     Copyright (c) 2026 2youg1 and the sprawling contributors -->

<script lang="ts">
  // The chosen session, the middle pane of the panorama workbench
  // (client-SPEC 7K): its title and state, then the instrument sheet and
  // the timeline. The sheet and the timeline need six figures the wire
  // does not carry yet (7K, current state), so today the pane names the
  // session and says what is missing, with the run's own page one step
  // away - a pane that drew a sheet of dashes would read as a session
  // that has nothing in it.
  import { newestWorking } from "../../core/belief/live";
  import { heldIn } from "../../core/belief/rooms";
  import { say } from "../../core/lang";
  import { toFragment } from "../../core/route";
  import { ui } from "../../ui";
  import type { Address } from "../../wire";
  import Empty from "../parts/empty.svelte";

  interface Props {
    readonly here: Address;
    // The room's name as the conversation calls it.
    readonly title: string;
  }

  const { here, title }: Props = $props();

  const u = ui();
  const { lang } = u;
  const belief = u.conn.belief;

  const live = $derived(newestWorking($belief, here));
  const latest = $derived(live ?? heldIn($belief, here).at(-1));
</script>

<section
  class="flex min-h-0 flex-col overflow-hidden pb-[calc(var(--spacing-band)+var(--spacing-wide))]"
  aria-label={say($lang, "world_session")}
>
  <div class="flex shrink-0 items-baseline justify-between gap-pane">
    <h2 class="text-heading font-heading text-text">{title}</h2>
    <span class="flex items-center gap-snug text-note text-text-quiet">
      {#if live !== undefined}
        <span class="size-dot animate-pulse rounded-pill bg-accent" aria-hidden="true"></span>
        {say($lang, "world_running")}
      {:else if latest !== undefined}
        {say($lang, "world_frozen")}
      {/if}
    </span>
  </div>
  <p class="shrink-0 text-note text-text-faint">{here}</p>
  <div class="mt-snug border-t border-edge">
    <Empty missing="world_session_pending" seat="inset">
      {#snippet action()}
        {#if latest !== undefined}
          <a class="text-note text-accent hover:text-accent-hover" href={toFragment({ kind: "run", run: latest.run })}>
            {say($lang, "world_open_run")}
          </a>
        {/if}
      {/snippet}
    </Empty>
  </div>
</section>
