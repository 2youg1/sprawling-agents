<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts">
  // A file as it is on disk: Markdown set as prose unless the reader
  // asks for the source, everything else numbered line by line. The
  // head of the file says where it is, and says how much was left out
  // when the answer was cut short.
  //
  // **Rendered Markdown is a reading column and the source is not.**
  // The prose takes the `measure` tier and is centred inside its pane -
  // one of the two seats `mx-auto` is allowed in (client-SPEC 4-33) -
  // while numbered lines grow to their longest line and scroll, because
  // code and tables are never capped.
  import { fill, say } from "../../core/lang";
  import { kib } from "../../core/time";
  import { ui } from "../../ui";
  import type { Address, DocumentAnswer } from "../../wire";
  import Path from "../parts/path.svelte";
  import Prose from "../prose.svelte";

  interface Props {
    readonly at: Address;
    readonly root: Address;
  }

  const { at, root }: Props = $props();

  const u = ui();
  const lang = u.lang;

  const asked = $derived(u.conn.asking.ask({ document: { at } }));
  // Three answers, not two: still asking, the city says the file is
  // not there, or the head itself. `null` is the missing file and
  // `undefined` is the wait, which the Solid reading kept apart by the
  // same two values.
  const doc = $derived.by((): DocumentAnswer | null | undefined => {
    const answer = $asked;
    if (answer === undefined) return undefined;
    return "document" in answer ? answer.document : null;
  });

  const markdown = $derived(at.endsWith(".md"));
  let raw = $state(false);

  const lines = $derived(
    doc === undefined || doc === null
      ? []
      : doc.text.split("\n").map((text) => ({ text })),
  );
  // The gutter is as wide as the largest line number it holds plus one,
  // in the same face as the code it numbers.
  const gutter = $derived(`${String(String(lines.length).length + 1)}ch`);
</script>

<div class="flex min-h-0 flex-1 flex-col">
  <div class="flex items-center gap-base pb-snug font-mono text-note text-text-faint">
    <Path path={at} base={root} />
    {#if doc !== undefined && doc !== null}
      <span class="text-text-disabled">{kib(doc.bytes)}</span>
    {/if}
    <span class="flex-1"></span>
    {#if markdown && doc !== undefined && doc !== null && !doc.binary}
      <button
        type="button"
        class={[
          "h-control-sm rounded-pill px-snug text-note",
          raw ? "bg-raised text-text" : "text-text-disabled hover:text-text-quiet",
        ]}
        onclick={() => {
          raw = !raw;
        }}
      >
        <!-- wording-ok: the name of the file format this toggle shows the source of -->
        .md
      </button>
    {/if}
  </div>
  {#if doc === undefined}
    <p class="text-text-disabled">…</p>
  {:else if doc === null}
    <p class="text-text-disabled">{say($lang, "file_missing")}</p>
  {:else}
    <div class="min-h-0 flex-1 overflow-auto rounded-panel bg-chrome/60 p-pane">
      {#if doc.binary}
        <p class="text-text-faint">{fill(say($lang, "file_binary"), { kib: kib(doc.bytes) })}</p>
      {/if}
      {#if doc.truncated}
        <p class="mb-base text-note text-alert">
          {fill(say($lang, "file_truncated"), { kib: kib(doc.text.length), total: kib(doc.bytes) })}
        </p>
      {/if}
      {#if !doc.binary}
        {#if markdown && !raw}
          <div class="mx-auto w-full max-w-measure">
            <Prose text={doc.text} />
          </div>
        {:else}
          <ol class="overflow-x-auto font-mono text-note leading-relaxed text-text-quiet">
            <!-- Each line carries its own identity, so a file that
                 repeats a line keys no two rows alike; the number beside
                 it is the position, which is what the number says. -->
            {#each lines as line, index (line)}
              <li class="flex whitespace-pre">
                <span
                  class="shrink-0 select-none pr-base text-right text-text-disabled"
                  style:width={gutter}>{index + 1}</span
                >
                <span>{line.text}</span>
              </li>
            {/each}
          </ol>
        {/if}
      {/if}
    </div>
  {/if}
</div>
