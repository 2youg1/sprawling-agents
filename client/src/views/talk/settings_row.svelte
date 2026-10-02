<!-- This Source Code Form is subject to the terms of the Mozilla Public
     License, v. 2.0. If a copy of the MPL was not distributed with this
     file, You can obtain one at https://mozilla.org/MPL/2.0/.
     Copyright (c) 2026 2youg1 and the sprawling contributors -->

<script lang="ts">
  // The row under the composer's line (client-SPEC 7I): the room, the gate
  // and the sandbox on the left, and - only before a session begins - the
  // model, the effort and the mode on the right. Once a session has begun
  // those three are the frozen facts of its first message head, and a
  // choice drawn here would say them a second time (client-SPEC 7D).
  import { say } from "../../core/lang";
  import { ui } from "../../ui";
  import type { Address } from "../../wire";
  import Bounds from "./bounds.svelte";
  import type { Pill } from "./composer";
  import PillView from "./pill.svelte";

  interface Props {
    // The four pills `composer.ts` builds: model, room, effort, mode.
    readonly specs: readonly [Pill, Pill, Pill, Pill];
    readonly room: Address | null;
    // Whether this room's session has begun, which takes the three
    // choices off the row.
    readonly begun: boolean;
    // Whether the last message stayed in the box because the link would
    // not take it.
    readonly kept: boolean;
  }

  const { specs, room, begun, kept }: Props = $props();
  const { lang } = ui();
</script>

<div class="mt-tight flex flex-wrap items-center justify-between gap-tight">
  <div class="-ml-snug flex min-w-0 flex-wrap items-center narrow:ml-0">
    <PillView spec={specs[1]} />
    <Bounds {room} />
    {#if kept}
      <span class="px-snug text-note text-alert">{say($lang, "talk_not_live")}</span>
    {/if}
  </div>
  {#if !begun}
    <div class="-mr-snug flex min-w-0 flex-wrap items-center narrow:mr-0">
      <PillView spec={specs[0]} />
      <PillView spec={specs[2]} />
      <PillView spec={specs[3]} />
    </div>
  {/if}
</div>
