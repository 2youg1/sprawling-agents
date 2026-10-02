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
  // reading of a diff exists; the patch under a row is the inspector's
  // own drawing, `inspect/patch.svelte`. The three folds its rows state
  // live in `changes.ts`, the companion module, because a working-tree
  // list reads the same `FileChange` through them.
</script>

<script lang="ts">
  import { readable } from "svelte/store";
  import type { Readable } from "svelte/store";
  import type { Snippet } from "svelte";

  import { readAnswer } from "../core/answered";
  import { say } from "../core/lang";
  import { ui } from "../ui";
  import type { Address, Answer, GitOid, Query } from "../wire";
  import { howWord, linesWord } from "./changes";
  import Patch from "./inspect/patch.svelte";
  import Unanswered from "./parts/unanswered.svelte";

  interface Props {
    readonly base: GitOid;
    readonly head: GitOid | null;
    // Where a chosen line is sent: the conversation whose composer it
    // prefills. Absent where the page has no conversation to talk to.
    readonly talk?: Address | undefined;
    // A control at the end of each file's row, handed the file's path:
    // a building's commit list puts "take back" there (client/Spec.lean §4-50).
    readonly act?: Snippet<[string]> | undefined;
  }

  const { base, head, talk, act }: Props = $props();

  const u = ui();
  const lang = u.lang;

  // The one row a hand opened: a patch is asked per file, and one
  // question at a time is one the page can hold.
  let open = $state<string | null>(null);

  const NOTHING: Readable<Answer | undefined> = readable(undefined);

  const question = $derived<Query>({ changes: { base, head } });
  const asked = $derived(u.conn.asking.ask(question));
  const read = $derived(readAnswer($asked, (held) => ("changes" in held ? held.changes.files : undefined)));
  const files = $derived(read.kind === "held" ? read.value : undefined);

  // The patch under the open row. The question exists only while a row
  // is open and both ends are named; `NOTHING` keeps the read below a
  // subscription to a store rather than a subscription to nothing.
  const patchQuestion = $derived.by((): Query | null => {
    const at = open;
    return at === null || head === null ? null : { hunks: { oid_a: base, oid_b: head, path: at } };
  });
  const patchStore = $derived(patchQuestion === null ? NOTHING : u.conn.asking.ask(patchQuestion));
  const patchRead = $derived(readAnswer($patchStore, (held) => ("hunks" in held ? held.hunks : undefined)));
  const patch = $derived(patchRead.kind === "held" ? patchRead.value : undefined);

  function toggle(path: string): void {
    open = open === path ? null : path;
  }
</script>

{#if read.kind === "unavailable"}
  <Unanswered query={read.query} asked={question} />
{:else if files === undefined}
  <p class="text-text-faint">…</p>
{:else if files.length > 0}
  <ul class="text-note">
    {#each files as file (file.path)}
      <li class="border-b border-edge">
        <div class="flex items-center gap-snug">
          <button
            type="button"
            class="flex min-w-0 flex-1 items-center gap-base py-snug text-left hover:text-text"
            aria-expanded={open === file.path}
            onclick={() => {
              toggle(file.path);
            }}
          >
            <span class="w-figure shrink-0 text-text-faint">{howWord($lang, file.how)}</span>
            <span class="flex-1 truncate font-mono text-text-quiet">{file.path}</span>
            <span class="shrink-0 font-mono text-text-faint">{linesWord($lang, file.lines)}</span>
          </button>
          {#if act !== undefined}
            {@render act(file.path)}
          {/if}
        </div>
        {#if open === file.path}
          {#if head === null}
            <p class="pb-base text-text-faint">{say($lang, "run_patch_needs_checkpoint")}</p>
          {:else if patchRead.kind === "unavailable" && patchQuestion !== null}
            <Unanswered query={patchRead.query} asked={patchQuestion} />
          {:else if patch === undefined}
            <p class="text-text-faint">…</p>
          {:else}
            <div class="pb-base">
              <div class="overflow-x-auto rounded-card border border-edge bg-page">
                <Patch {patch} {talk} />
              </div>
            </div>
          {/if}
        {/if}
      </li>
    {/each}
  </ul>
{:else}
  <p class="text-text-faint">{say($lang, "run_no_changes")}</p>
{/if}
