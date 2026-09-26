<script lang="ts" module>
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// Every file the run changed, one after another, each cut down to the
// stretches that moved (Zed's multibuffer). A line carries its number
// on the side it exists on: a removed line its old number, an added
// line its new one, so a person can find either in the file. The file
// header sticks to the top while its hunks scroll under it, which keeps
// the question "which file am I in" answered without looking up.
</script>

<script lang="ts">
  import { say } from "../../core/lang";
  import { ui } from "../../ui";
  import { painted } from "../parts/code";
  import { PAINT } from "../parts/code.svelte";
  import type { Sign, Touched } from "./trace";

  interface Props {
    readonly files: readonly Touched[];
  }

  const { files }: Props = $props();
  const { lang } = ui();

  const ROW: Record<Sign, string> = { added: "bg-accent/10", removed: "bg-alert/12", kept: "" };
  const SIGN: Record<Sign, string> = { added: "+", removed: "−", kept: "" };
  const SIGN_INK: Record<Sign, string> = { added: "text-accent", removed: "text-alert", kept: "text-text-faint" };
</script>

{#if files.length === 0}
  <p class="px-snug py-snug text-note text-text-faint">{say($lang, "mon_nothing_changed")}</p>
{/if}
{#each files as file (file.path)}
  <div data-path={file.path}>
    <div class="sticky top-0 z-[1] flex h-control items-center gap-snug border-b border-edge bg-chrome px-snug text-note">
      <span class="min-w-0 truncate font-mono text-text">{file.path}</span>
      <span class="shrink-0 rounded-pill bg-raised px-snug text-text-quiet"
        >{say($lang, file.how === "added" ? "change_added" : "change_modified")}</span
      >
    </div>
    {#each file.hunks as hunk, h (h)}
      <div class="border-b border-edge bg-page font-mono text-note leading-[1.65]">
        {#each hunk.lines as line, at (at)}
          <div class="flex {ROW[line.sign]}">
            <span class="flex shrink-0 select-none text-text-faint figure" aria-hidden="true">
              <span class="w-[4.5ch] pe-tight text-right">{line.old ?? ""}</span>
              <span class="w-[4.5ch] pe-tight text-right">{line.new ?? ""}</span>
              <span class="w-[2.5ch] text-center {SIGN_INK[line.sign]}">{SIGN[line.sign]}</span>
            </span>
            <span class="min-w-0 flex-1 ps-[2ch] pe-snug -indent-[2ch] whitespace-pre-wrap wrap-break-word text-text-quiet"
              >{#each painted(line.text, file.path) as piece, p (p)}<span class={PAINT[piece.ink]}>{piece.text}</span
                >{/each}</span
            >
          </div>
        {/each}
      </div>
    {/each}
  </div>
{/each}
