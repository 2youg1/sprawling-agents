<!-- This Source Code Form is subject to the terms of the Mozilla Public
License, v. 2.0. If a copy of the MPL was not distributed with this
file, You can obtain one at https://mozilla.org/MPL/2.0/.
Copyright (c) 2026 2youg1 and the sprawling contributors -->

<script lang="ts">
  import { ui } from "../../ui";
  import { clock } from "../../core/time";
  import ForkButton from "./fork_button.svelte";
  import type { ForkEntry, ForkPlan } from "./forking";
  import NotePlace from "./note_place.svelte";
  import type { RunId } from "../../wire";
  interface Props {
    readonly text: string;
    readonly label: string;

    readonly labelHref?: string | undefined;
    readonly at: number | undefined;
    readonly entry: ForkEntry | null;
    readonly run: RunId;
    readonly onFork?: ((plan: ForkPlan) => void) | undefined;
    readonly onHover: (entry: ForkEntry | null) => void;
  }

  const { text, label, labelHref, at, entry, run, onFork, onHover }: Props = $props();
  const u = ui();
  const { lang } = u;
</script>

<NotePlace rhythm="shape">
  <div class="group flex flex-col items-end">
    <div
      class="max-w-[83%] rounded-panel bg-speech px-pane py-base font-read text-body leading-read whitespace-pre-wrap wrap-anywhere"
    >
      {text}
    </div>
    <div class="mt-tight flex items-center gap-snug text-note text-text-faint">
      {#if entry !== null && onFork !== undefined}
        <ForkButton {entry} {run} {onFork} {onHover} />
      {/if}
      <span
        >{#if labelHref === undefined}{label}{:else}<a
            href={labelHref}
            class="text-text-quiet hover:text-text">{label}</a
          >{/if}{#if at !== undefined}&nbsp;· {clock($lang, at)}{/if}</span
      >
    </div>
  </div>
</NotePlace>
