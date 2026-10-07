<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts">
  // How the code column is drawn, and nothing else (client D95): every
  // file the run changed, one after another, each cut down to the
  // stretches that moved (Zed's multibuffer). A line carries its number
  // on the side it exists on: a removed line its old number, an added
  // line its new one, so a person can find either in the file. The file
  // header sticks to the top while its hunks scroll under it, which
  // keeps the question "which file am I in" answered without looking up.
  // Every word and every action's handler or address arrives in
  // `CodeColumnLook` (`./code_column.ts`); a wire bag is spread
  // unchanged on the element it is for.
  import Inked from "../parts/inked.svelte";
  import type { CodeColumnLook } from "./code_column";
  import type { Sign } from "./trace";

  const look: CodeColumnLook = $props();

  // A hunk action is a quiet control: its fill arrives decelerating and
  // leaves accelerating (docs/frontend-method.md §4-43).
  const ACTION =
    "inline-flex h-control-sm items-center rounded-control px-snug text-label text-text-quiet transition-colors ease-leave hover:bg-raised hover:text-text hover:ease-arrive";

  const ROW: Record<Sign, string> = { added: "bg-accent/10", removed: "bg-alert/12", kept: "" };
  const SIGN: Record<Sign, string> = { added: "+", removed: "−", kept: "" };
  const SIGN_INK: Record<Sign, string> = { added: "text-accent", removed: "text-alert", kept: "text-text-faint" };
</script>

{#if look.empty !== undefined}
  <p class="px-snug py-snug text-note text-text-faint">{look.empty}</p>
{/if}
{#each look.files as file (file.path)}
  <div data-path={file.path}>
    <div class="file sticky top-0 flex h-control items-center gap-snug border-b border-edge bg-chrome px-snug text-note">
      <span class="min-w-0 truncate font-mono text-text">{file.path}</span>
      <span class="shrink-0 rounded-pill bg-raised px-snug text-text-quiet">{file.how}</span>
    </div>
    {#each file.hunks as hunk, h (h)}
      {#if hunk.actions.length > 0}
        <div class="flex justify-end gap-tight border-b border-edge bg-page px-snug">
          {#each hunk.actions as action (action.key)}
            {#if action.kind === "press"}
              <button class={ACTION} {...action.wire}>{action.label}</button>
            {:else}
              <a class={ACTION} {...action.wire}>{action.label}</a>
            {/if}
          {/each}
        </div>
      {/if}
      <div class="hunk border-b border-edge bg-page font-mono text-note">
        {#each hunk.lines as line, at (at)}
          <div class="flex {ROW[line.sign]}">
            <span class="flex shrink-0 select-none text-text-faint figure" aria-hidden="true">
              <span class="number pe-tight text-right">{line.old}</span>
              <span class="number pe-tight text-right">{line.new}</span>
              <span class="sign text-center {SIGN_INK[line.sign]}">{SIGN[line.sign]}</span>
            </span>
            <span class="text min-w-0 flex-1 pe-snug whitespace-pre-wrap wrap-break-word text-text-quiet"
              ><Inked text={line.text} source={file.path} /></span
            >
          </div>
        {/each}
      </div>
    {/each}
  </div>
{/each}

<style>
  /* The header stands over the hunk lines that scroll under it. */
  .file {
    z-index: 1;
  }

  .hunk {
    line-height: 1.65;
  }

  /* Room for a five-digit line number and its sign, in the width of the
  digits themselves. */
  .number {
    width: 4.5ch;
  }

  .sign {
    width: 2.5ch;
  }

  /* A wrapped line hangs under its own first character, so the
  continuation never reads as a new line. */
  .text {
    padding-inline-start: 2ch;
    text-indent: -2ch;
  }
</style>
