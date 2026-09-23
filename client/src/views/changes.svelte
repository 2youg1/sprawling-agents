<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts" module>
  // What moved between two checkpoints: the files as rows, and under
  // the row a hand opened, the patch of that one file. Drawn the same
  // from a run's lens and from a building's commit list, so one
  // reading of a diff exists. The three folds its rows state live in
  // `changes.ts`, the companion module, because a working-tree list
  // reads the same `FileChange` through them.
</script>

<script lang="ts">
  import { readable } from "svelte/store";
  import type { Readable } from "svelte/store";

  import { fill, say } from "../core/lang";
  import { ui } from "../ui";
  import type { Answer, FileChange, GitOid, HunksAnswer } from "../wire";
  import { howWord, linesWord } from "./changes";

  interface Props {
    readonly base: GitOid;
    readonly head: GitOid | null;
  }

  const { base, head }: Props = $props();

  const u = ui();
  const lang = u.lang;

  // The one row a hand opened: a patch is asked per file, and one
  // question at a time is one the page can hold.
  let open = $state<string | null>(null);

  const NOTHING: Readable<Answer | undefined> = readable(undefined);

  const asked = $derived(u.conn.asking.ask({ changes: { base, head } }));
  const files = $derived.by((): readonly FileChange[] | undefined => {
    const held = $asked;
    return held !== undefined && "changes" in held ? held.changes.files : undefined;
  });

  // The patch under the open row. The question exists only while a row
  // is open and both ends are named; `NOTHING` keeps the read below a
  // subscription to a store rather than a subscription to nothing.
  const patchStore = $derived.by((): Readable<Answer | undefined> => {
    const at = open;
    if (at === null || head === null) return NOTHING;
    return u.conn.asking.ask({ hunks: { oid_a: base, oid_b: head, path: at } });
  });
  const patch = $derived.by((): HunksAnswer | undefined => {
    const held = $patchStore;
    return held !== undefined && "hunks" in held ? held.hunks : undefined;
  });

  function toggle(path: string): void {
    open = open === path ? null : path;
  }

  // A patch line reads by its first character, which is the only thing
  // the diff format says about it.
  function lineInk(text: string): string {
    if (text.startsWith("+")) return "text-accent";
    if (text.startsWith("-")) return "text-alert";
    return "text-text-quiet";
  }
</script>

{#if files === undefined}
  <p class="text-text-disabled">…</p>
{:else if files.length > 0}
  <ul class="text-note">
    {#each files as file (file.path)}
      <li class="border-b border-edge">
        <button
          type="button"
          class="flex w-full items-center gap-base py-snug text-left hover:text-text"
          aria-expanded={open === file.path}
          onclick={() => {
            toggle(file.path);
          }}
        >
          <span class="w-figure shrink-0 text-text-faint">{howWord($lang, file.how)}</span>
          <span class="flex-1 truncate font-mono text-text-quiet">{file.path}</span>
          <span class="shrink-0 font-mono text-text-disabled">{linesWord($lang, file.lines)}</span>
        </button>
        {#if open === file.path}
          {#if head === null}
            <p class="pb-base text-text-disabled">{say($lang, "run_patch_needs_fence")}</p>
          {:else if patch === undefined}
            <p class="text-text-disabled">…</p>
          {:else}
            <div class="pb-base">
              <!-- Tight against the tag on purpose: a `<pre>` renders
                   its whitespace, so the indent of this block would
                   become the first line of the patch. -->
              <pre
                class="overflow-x-auto rounded-card border border-edge bg-page p-base font-mono text-note leading-relaxed"
              >{#each patch.lines as line (line.number)}<div class={lineInk(line.text)}>{line.text}</div>{/each}{#each patch.withheld as withheld (withheld.number)}<div class="text-text-disabled">{fill(say($lang, "run_withheld"), { n: String(withheld.number), reason: withheld.reason })}</div>{/each}</pre>
            </div>
          {/if}
        {/if}
      </li>
    {/each}
  </ul>
{:else}
  <p class="text-text-faint">{say($lang, "run_no_changes")}</p>
{/if}
