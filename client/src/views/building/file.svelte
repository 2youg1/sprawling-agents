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
  // one of the two seats `mx-auto` is allowed in (client/Spec.lean §4-33) -
  // while numbered lines grow to their longest line and scroll, because
  // code and tables are never capped.
  import { putSpine } from "../../core/commands";
  import { readDocument } from "../../core/document";
  import { fill, say } from "../../core/lang";
  import { kib } from "../../core/time";
  import { ui } from "../../ui";
  import { Address, type Query, type SpineDocument } from "../../wire";
  import Path from "../parts/path.svelte";
  import Unanswered from "../parts/unanswered.svelte";
  import Prose from "../prose.svelte";

  // The four documents a person may write through this door. `PutSpine`
  // names one of them and never a path, so a file outside this table has
  // no control to draw and no way to be written from here.
  const SPINE: Record<string, SpineDocument | undefined> = {
    "Roadmap.md": "roadmap",
    "Memo.md": "memo",
    "Handoff.md": "handoff",
    "SPEC.md": "spec",
  };

  interface Props {
    readonly at: Address;
    readonly root: Address;
  }

  const { at, root }: Props = $props();

  const u = ui();
  const lang = u.lang;

  const question = $derived<Query>({ document: { at } });
  const asked = $derived(u.conn.asking.ask(question));
  // Three readings, not two: still asking, the city could not look (a
  // file it does not hold answers this way too), or the head itself.
  const read = $derived(readDocument($asked));
  const doc = $derived(read.kind === "held" ? read.value : undefined);

  const markdown = $derived(at.endsWith(".md"));
  let raw = $state(false);
  let editing = $state(false);
  let draft = $state("");

  // Which spine document this is, and the building it belongs to. The
  // name decides, and the building is whatever stands above it: these
  // four sit at a building's root, so the path already says both.
  const spine = $derived.by((): { which: SpineDocument; building: Address } | null => {
    const cut = at.lastIndexOf("/");
    if (cut < 0) return null;
    const which = SPINE[at.slice(cut + 1)];
    return which === undefined ? null : { which, building: Address.make(at.slice(0, cut)) };
  });

  function change(): void {
    const held = doc;
    if (held === undefined) return;
    draft = held.text;
    editing = true;
  }

  // The body is sent whole with the text the person started from, so a
  // resident that rewrote the file underneath is refused rather than
  // written over.
  function save(): void {
    const to = spine;
    const held = doc;
    if (to === null || held === undefined) return;
    if (u.send(putSpine(to.building, to.which, held.text, draft))) editing = false;
  }

  const lines = $derived(
    doc === undefined
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
    {#if doc !== undefined}
      <span class="text-text-faint">{kib(doc.bytes)}</span>
    {/if}
    <span class="flex-1"></span>
    {#if markdown && doc !== undefined && !doc.binary}
      <button
        type="button"
        class={[
          "h-control-sm rounded-pill px-snug text-note",
          raw ? "bg-raised text-text" : "text-text-faint hover:text-text-quiet",
        ]}
        onclick={() => {
          raw = !raw;
        }}
      >
        <!-- wording-ok: the name of the file format this toggle shows the source of -->
        .md
      </button>
      {#if spine !== null && !editing}
        <button
          type="button"
          class="h-control-sm rounded-pill px-snug text-note text-text-faint hover:text-text-quiet"
          onclick={change}
        >
          {say($lang, "file_edit")}
        </button>
      {/if}
    {/if}
  </div>
  {#if read.kind === "asking"}
    <p class="text-text-faint">…</p>
  {:else if read.kind === "unavailable"}
    <Unanswered query={read.query} asked={question} />
  {:else if doc !== undefined}
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
        {#if editing}
          <textarea
            class="min-h-palette w-full rounded-card border border-edge-input bg-chrome p-snug font-mono text-note leading-relaxed"
            bind:value={draft}
          ></textarea>
          <div class="mt-base flex gap-base">
            <button
              type="button"
              class="h-control-sm rounded-pill bg-raised px-snug text-note text-text"
              onclick={save}
            >
              {say($lang, "file_save")}
            </button>
            <button
              type="button"
              class="h-control-sm rounded-pill px-snug text-note text-text-faint hover:text-text-quiet"
              onclick={() => {
                editing = false;
              }}
            >
              {say($lang, "file_discard")}
            </button>
          </div>
        {:else if markdown && !raw}
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
                  class="shrink-0 select-none pr-base text-right text-text-faint"
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
