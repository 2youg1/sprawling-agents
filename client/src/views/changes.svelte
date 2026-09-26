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

  import { readAnswer } from "../core/answered";
  import { fill, say } from "../core/lang";
  import { toFragment } from "../core/route";
  import { ui } from "../ui";
  import type { Address, Answer, GitOid, HunksAnswer, Query } from "../wire";
  import { howWord, linesWord, numbered, quoteLine } from "./changes";
  import type { CodeLine } from "./changes";
  import Unanswered from "./parts/unanswered.svelte";

  interface Props {
    readonly base: GitOid;
    readonly head: GitOid | null;
    // Where a chosen line is sent: the conversation whose composer it
    // prefills. Absent where the page has no conversation to talk to.
    readonly talk?: Address | undefined;
  }

  const { base, head, talk }: Props = $props();

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

  const INK: Record<CodeLine["kind"], string> = {
    added: "text-accent",
    removed: "text-alert",
    context: "text-text-quiet",
  };

  // A chosen line joins whatever the person had already started to
  // write there; the link then opens that conversation, whose composer
  // reads the draft door when it mounts.
  function choose(to: Address, held: HunksAnswer, line: CodeLine): void {
    const draft = u.prefs.draft(to);
    const quote = quoteLine(held, line);
    u.prefs.setDraft(to, draft === "" ? quote : [draft, quote].join("\n"));
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
          <span class="shrink-0 font-mono text-text-faint">{linesWord($lang, file.lines)}</span>
        </button>
        {#if open === file.path}
          {#if head === null}
            <p class="pb-base text-text-faint">{say($lang, "run_patch_needs_fence")}</p>
          {:else if patchRead.kind === "unavailable" && patchQuestion !== null}
            <Unanswered query={patchRead.query} asked={patchQuestion} />
          {:else if patch === undefined}
            <p class="text-text-faint">…</p>
          {:else}
            <div class="pb-base">
              <!-- Rows rather than one `<pre>`: each line carries its
                   two numbers in a gutter, and a gutter is a flex
                   child; only the code itself keeps its whitespace. -->
              <div
                class="overflow-x-auto rounded-card border border-edge bg-page py-base font-mono text-note leading-relaxed"
              >
                {#each numbered(patch) as line (line.number)}
                  {#if line.kind === "withheld"}
                    <div class="px-base text-text-faint">
                      {fill(say($lang, "run_withheld"), { n: String(line.number), reason: line.reason })}
                    </div>
                  {:else if line.kind === "head"}
                    <div class="px-base whitespace-pre text-text-faint">{line.text}</div>
                  {:else if line.kind === "hunk"}
                    <div class="my-tight flex gap-base bg-chrome px-base text-text-faint">
                      {#if line.folded > 0}
                        <span class="shrink-0">{fill(say($lang, "change_folded"), { n: String(line.folded) })}</span>
                      {/if}
                      <span class="whitespace-pre">{line.text}</span>
                    </div>
                  {:else}
                    {@const old = line.kind === "added" ? null : line.old}
                    {@const now = line.kind === "removed" ? null : line.new}
                    <div class="flex {INK[line.kind]}">
                      {#if talk === undefined}
                        <span class="flex shrink-0 text-text-faint select-none">
                          <span class="w-[5ch] pr-tight text-right">{old ?? ""}</span>
                          <span class="w-[5ch] pr-tight text-right">{now ?? ""}</span>
                        </span>
                      {:else}
                        {@const to = talk}
                        <a
                          class="flex shrink-0 text-text-faint select-none hover:bg-chrome hover:text-text-quiet"
                          href={toFragment({ kind: "talk", address: to })}
                          aria-label={fill(say($lang, "change_line_quote"), { n: String(now ?? old ?? line.number) })}
                          onclick={() => {
                            choose(to, patch, line);
                          }}
                        >
                          <span class="w-[5ch] pr-tight text-right">{old ?? ""}</span>
                          <span class="w-[5ch] pr-tight text-right">{now ?? ""}</span>
                        </a>
                      {/if}
                      <!-- wording-ok: the diff format's own marks, not words. -->
                      <span class="pr-base whitespace-pre">{line.kind === "added" ? "+" : line.kind === "removed" ? "-" : " "}{line.text}</span>
                    </div>
                  {/if}
                {/each}
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
