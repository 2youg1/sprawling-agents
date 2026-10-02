<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts">
  // What the mailbox holds, as one scrolling column (client-SPEC 4-49):
  // its head - the name, the link when it is not live, and on a phone
  // the way back - then deciding, working, recent and the notices, in
  // the order of what needs the person. `mailbox.svelte` seats it in the
  // popover; `#/gallery` seats it in a frame of the same width.
  import { say } from "../../core/lang";
  import { linkRecovery } from "../../core/recovering";
  import { ui } from "../../ui";
  import { recover, recoveryLabel } from "../notice_recovery";
  import Button from "../parts/button.svelte";
  import Deciding from "./deciding.svelte";
  import { walk } from "./entries";
  import { linkWord } from "./link_word";
  import Notices from "./notices.svelte";
  import Recent from "./recent.svelte";
  import Working from "./working.svelte";

  interface Props {
    // Puts the mailbox away: after following a row, and from the way
    // back on a phone.
    readonly onClose: () => void;
  }

  const { onClose }: Props = $props();

  const u = ui();
  const { lang } = u;
  const link = u.conn.state;

  let scroller = $state<HTMLElement | undefined>(undefined);

  // j, k and the digits are heard on the column, below every entry.
  $effect(() => {
    const held = scroller;
    if (held === undefined) return;
    const heard = (event: KeyboardEvent): void => {
      walk(held, event);
    };
    held.addEventListener("keydown", heard);
    return () => {
      held.removeEventListener("keydown", heard);
    };
  });
</script>

<div class="flex h-full min-h-0 flex-col">
  <header class="flex h-bar shrink-0 items-center gap-snug border-b border-edge px-base">
    <!-- On a phone the column is a whole-screen sheet, and its way back
    stands at the top of the side it came from (refrain U9). -->
    <span class="hidden narrow:inline-flex">
      <Button tone="quiet" label={say($lang, "mailbox_back")} onPress={onClose} />
    </span>
    <h2 class="min-w-0 flex-1 truncate text-label font-label">{say($lang, "edge_mailbox")}</h2>
    {#if $link.kind !== "live"}
      <span class="shrink-0 text-note text-text-faint">{linkWord($lang, $link)}</span>
      {#if $link.kind === "refused"}
        {@const lever = linkRecovery($link.error.code)}
        <Button
          tone="quiet"
          label={recoveryLabel(lever, $lang)}
          onPress={() => {
            recover(u, lever, { error: $link.error, about: null });
          }}
        />
      {/if}
    {/if}
  </header>
  <!-- One scroller for every section, walked by j and k and reached by
  the digits (client-SPEC 7-11). -->
  <div bind:this={scroller} class="mailbox min-h-0 flex-1 overflow-y-auto px-base pb-wide">
    <Deciding />
    <Working onLeave={onClose} />
    <Recent scroller={() => scroller} onLeave={onClose} />
    <Notices />
  </div>
</div>
