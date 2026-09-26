<!-- This Source Code Form is subject to the terms of the Mozilla Public
     License, v. 2.0. If a copy of the MPL was not distributed with this
     file, You can obtain one at https://mozilla.org/MPL/2.0/.
     Copyright (c) 2026 2youg1 and the sprawling contributors -->

<script lang="ts">
  // The right end of the composer's lower row: what two keys do while
  // there is something to send (ux A3), the microphone where the city
  // can transcribe, the stop square while a run is going, and the send
  // arrow. Round icon buttons, named by the verb each one sends, in the
  // place every chat page puts them.
  import type { Sending } from "../../core/doing";
  import { say } from "../../core/lang";
  import { ui } from "../../ui";
  import type { Address } from "../../wire";
  import Glyph from "../parts/glyph.svelte";
  import Record from "./record.svelte";
  import Send from "./send.svelte";

  interface Props {
    readonly sending: Sending;
    readonly handed: boolean;
    readonly here: Address | null;
    readonly empty: boolean;
    // Whether this city has an endpoint that transcribes and this page
    // can record.
    readonly hearing: boolean;
    readonly onWords: (words: string) => void;
    readonly onStop: () => boolean;
  }

  const { sending, handed, here, empty, hearing, onWords, onStop }: Props = $props();
  const { lang } = ui();
</script>

<div class="flex shrink-0 items-center gap-snug">
  {#if !empty}
    <span class="hidden text-note text-text-faint @lg/page:inline">{say($lang, "talk_enter_hint")}</span>
  {/if}
  {#if hearing}
    <Record {onWords} />
  {/if}
  {#if sending !== "dispatch"}
    <button
      type="button"
      class="relative flex size-control shrink-0 items-center justify-center rounded-pill bg-raised-hover text-alert before:absolute before:-inset-tight before:content-[''] hover:text-text"
      aria-label={say($lang, "talk_stop")}
      onclick={() => {
        onStop();
      }}
    >
      <Glyph name="stop" size="sm" />
    </button>
  {/if}
  <Send {sending} {handed} {here} {empty} />
</div>
