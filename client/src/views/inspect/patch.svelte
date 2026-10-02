<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts" module>
  // One file's patch between two checkpoints, as a reader points at it
  // (docs/frontend-method.md §7F, client/Spec.lean §7-2): a number column, a sign column and the line,
  // the added and removed lines on the accent and alert washes, and the
  // line a person last chose marked by a 2 px accent bar. The inspector's
  // diff and `changes.svelte`'s open row both draw a patch through this
  // file, so one reading of a diff exists.
  //
  // **One number per line, the one the line has in the tree it lives in**:
  // the new tree's for a kept or added line, the old tree's for a removed
  // one, so the column reads as the file reads. The quote a chosen line
  // puts in the conversation names the tree as well (`changes.ts`'s
  // `quoteLine`), which is where the two numbers a line could have are
  // told apart.
  //
  // **Lines do not fold.** A patch is read by column, so a long line runs
  // sideways inside the scroller and the rows stay one baseline step
  // apart.
</script>

<script lang="ts">
  import { fill, say } from "../../core/lang";
  import { toFragment } from "../../core/route";
  import { ui } from "../../ui";
  import type { Address, HunksAnswer } from "../../wire";
  import { numbered, quoteLine } from "../changes";
  import type { CodeLine } from "../changes";
  import Inked from "../parts/inked.svelte";
  import { quoteInto } from "../talk/quoting";

  interface Props {
    readonly patch: HunksAnswer;
    // The conversation a chosen line is quoted into; absent where the
    // page has no conversation, and then the numbers are not controls.
    readonly talk?: Address | undefined;
    // The patch line a person last chose, by its place in the patch.
    readonly cursor?: number | null;
    readonly onCursor?: (line: CodeLine) => void;
  }

  const { patch, talk, cursor = null, onCursor }: Props = $props();

  const u = ui();
  const lang = u.lang;

  const lines = $derived(numbered(patch));

  const WASH: Record<CodeLine["kind"], string> = {
    added: "bg-accent/12",
    removed: "bg-alert/12",
    context: "",
  };
  const MARK: Record<CodeLine["kind"], string> = {
    added: "text-accent",
    removed: "text-alert",
    context: "text-text-faint",
  };
  const SIGN: Record<CodeLine["kind"], string> = { added: "+", removed: "−", context: "" };

  // A chosen line joins whatever the person had already started to
  // write there - in the box when one is open there, in its draft
  // otherwise - and the link then opens that conversation.
  function choose(to: Address, line: CodeLine): void {
    quoteInto(u.prefs, to, quoteLine(patch, line));
    onCursor?.(line);
  }
</script>

<div class="w-max min-w-full py-snug font-mono text-note leading-[calc(3*var(--spacing-baseline))]">
  {#each lines as line (line.number)}
    {#if line.kind === "withheld"}
      <div class="pl-[calc(9*var(--spacing-baseline))] text-text-faint">
        {fill(say($lang, "run_withheld"), { n: String(line.number), reason: line.reason })}
      </div>
    {:else if line.kind === "head"}
      <div class="pl-[calc(9*var(--spacing-baseline))] whitespace-pre text-text-faint">{line.text}</div>
    {:else if line.kind === "hunk"}
      <div class="my-tight flex gap-base bg-raised pl-[calc(9*var(--spacing-baseline))] pr-wide text-text-faint">
        {#if line.folded > 0}
          <span class="shrink-0">{fill(say($lang, "change_folded"), { n: String(line.folded) })}</span>
        {/if}
        <span class="whitespace-pre">{line.text}</span>
      </div>
    {:else}
      {@const place = line.kind === "removed" ? line.old : line.new}
      <div
        class={[
          "grid grid-cols-[calc(7*var(--spacing-baseline))_calc(2*var(--spacing-baseline))_auto] pr-wide",
          WASH[line.kind],
          cursor === line.number && "shadow-[inset_var(--spacing-hair)_0_0_var(--color-accent)]",
        ]}
        data-line={line.number}
      >
        {#if talk === undefined}
          <span class="pr-pane text-right select-none {MARK[line.kind]}">{place ?? ""}</span>
        {:else}
          {@const to = talk}
          <a
            class="pr-pane text-right select-none hover:text-text {MARK[line.kind]}"
            href={toFragment({ kind: "talk", address: to })}
            aria-label={fill(say($lang, "change_line_quote"), { n: String(place ?? line.number) })}
            onclick={() => {
              choose(to, line);
            }}>{place ?? ""}</a
          >
        {/if}
        <!-- wording-ok: the diff format's own marks, not words. -->
        <span class="select-none {MARK[line.kind]}" aria-hidden="true">{SIGN[line.kind]}</span>
        <span class="whitespace-pre text-text"><Inked text={line.text} source={patch.path} /></span>
      </div>
    {/if}
  {/each}
</div>
