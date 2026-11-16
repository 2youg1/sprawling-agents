<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts" module>
  
  
</script>

<script lang="ts">
  import { readable } from "svelte/store";
  import type { Readable } from "svelte/store";
  import type { Snippet } from "svelte";

  import { readAnswer } from "../core/answered";
  import { say } from "../core/lang";
  import { ui } from "../ui";
  import type { Address, Answer, GitOid, Query } from "../wire";
  import { disclosureWire } from "./building/disclosure";
  import Disclosure from "./building/disclosure.look.svelte";
  import { howWord, linesWord } from "./changes";
  import Patch from "./inspect/patch.svelte";
  import Unanswered from "./parts/unanswered.svelte";

  interface Props {
    readonly base: GitOid;
    readonly head: GitOid | null;
    
    
    readonly talk?: Address | undefined;
    
    
    readonly act?: Snippet<[string]> | undefined;
  }

  const { base, head, talk, act }: Props = $props();

  const u = ui();
  const lang = u.lang;

  
  let open = $state<string | null>(null);

  const NOTHING: Readable<Answer | undefined> = readable(undefined);

  const question = $derived<Query>({ changes: { base, head } });
  const asked = $derived(u.conn.asking.ask(question));
  const read = $derived(
    readAnswer($asked, (held) => ("changes" in held ? held.changes.files : undefined)),
  );
  const files = $derived(read.kind === "held" ? read.value : undefined);

  
  const patchQuestion = $derived.by((): Query | null => {
    const at = open;
    return at === null || head === null ? null : { hunks: { oid_a: base, oid_b: head, path: at } };
  });
  const patchStore = $derived(patchQuestion === null ? NOTHING : u.conn.asking.ask(patchQuestion));
  const patchRead = $derived(
    readAnswer($patchStore, (held) => ("hunks" in held ? held.hunks : undefined)),
  );
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
          {#snippet cells()}
            <span class="truncate text-text-faint">{howWord($lang, file.how)}</span>
            <span class="truncate font-mono text-text-quiet">{file.path}</span>
            <span class="font-mono text-text-faint">{linesWord($lang, file.lines)}</span>
          {/snippet}
          <div class="min-w-0 flex-1">
            <Disclosure
              layout="change"
              open={open === file.path}
              wire={disclosureWire(open === file.path, () => {
                toggle(file.path);
              })}
              {cells}
            />
          </div>
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
