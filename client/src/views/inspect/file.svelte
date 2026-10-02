<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts" module>
  // A call that read one file, as that file opened at the line it read
  // from (client-SPEC 4-45). The text is the worktree's as it stands now
  // (`Query::Document`), read-only through the one code view, so its line
  // numbers are the ones the person's editor will open; the call's own
  // answer stays in the Ledger, where the run page reads it.
  //
  // **What the city did not send is said, not filled in.** A file longer
  // than the window `Document` answers with shows its head; a line past
  // that head is named as out of reach instead of being marked on a line
  // it is not.
</script>

<script lang="ts">
  import { Option, Schema } from "effect";
  import { readable } from "svelte/store";
  import type { Readable } from "svelte/store";

  import { readDocument } from "../../core/document";
  import { fill, say } from "../../core/lang";
  import { kib } from "../../core/time";
  import { ui } from "../../ui";
  import { Address } from "../../wire";
  import type { Answer, Query } from "../../wire";
  import Code from "../parts/code.svelte";
  import Unanswered from "../parts/unanswered.svelte";
  import Reach from "./reach.svelte";
  import type { ReadFrom } from "./reading";

  interface Props {
    readonly read: ReadFrom;
  }

  const { read }: Props = $props();

  const u = ui();
  const lang = u.lang;

  const at = $derived(Option.getOrNull(Schema.decodeUnknownOption(Address)(read.path)));
  const question = $derived.by((): Query | null => (at === null ? null : { document: { at } }));
  const NOTHING: Readable<Answer | undefined> = readable(undefined);
  const asked = $derived(question === null ? NOTHING : u.conn.asking.ask(question));
  const held = $derived(readDocument($asked));
  const doc = $derived(held.kind === "held" ? held.value : null);
  const lines = $derived(doc === null ? 0 : doc.text.split("\n").length);
  const reached = $derived(read.line <= lines);
</script>

{#snippet reach()}
  <Reach path={read.path} line={read.line} shown="current" />
{/snippet}

<div class="flex min-h-0 flex-1 flex-col bg-page">
  {#if question === null}
    <p class="px-wide py-snug text-note text-text-faint">{say($lang, "file_missing")}</p>
  {:else if held.kind === "unavailable"}
    <div class="px-wide py-snug"><Unanswered query={held.query} asked={question} /></div>
  {:else if doc === null}
    <p class="px-wide py-snug text-note text-text-faint">…</p>
  {:else if doc.binary}
    <p class="px-wide py-snug text-note text-text-faint">
      {fill(say($lang, "inspect_not_text"), { bytes: kib(doc.bytes) })}
    </p>
  {:else}
    {#if doc.truncated && !reached}
      <p class="border-b border-edge px-wide py-tight text-note text-text-faint">
        {fill(say($lang, "inspect_line_beyond"), { n: String(read.line), bytes: kib(doc.bytes) })}
      </p>
    {/if}
    <Code path={read.path} text={doc.text} cited={reached ? read.line : undefined} aside={reach} />
  {/if}
</div>
