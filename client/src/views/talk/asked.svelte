<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->
<script lang="ts">
  // What an approval card asks about, drawn on the card. The one line
  // of `action_desc` names the question; the artifact is the thing the
  // question is about, and a person who has to open another page to
  // read it answers slower or answers blind. Bounded like a tool's
  // output, and cut by the same content answer the file view reads.

  import { fill, say } from "../../core/lang";
  import { kib } from "../../core/time";
  import { ui } from "../../ui";
  import type { ContentAnswer, Locator } from "../../wire";

  interface Props {
    readonly locator: Locator;
    // The content a caller hands in place of a city; absent, the card
    // asks for it.
    readonly content?: ContentAnswer | undefined;
  }

  const { locator, content }: Props = $props();

  const u = ui();
  const lang = u.lang;

  const asked = $derived(u.conn.asking.ask({ content: { locator } }));
  const shown = $derived.by((): ContentAnswer | undefined => {
    if (content !== undefined) return content;
    const held = $asked;
    return held !== undefined && "content" in held ? held.content : undefined;
  });
</script>

<details class="my-snug text-note" open>
  <summary class="cursor-pointer text-text-faint hover:text-text-quiet">{say($lang, "wait_asks")}</summary>
  {#if shown === undefined}
    <p class="text-text-disabled">…</p>
  {:else if shown.binary}
    <p class="text-text-faint">{fill(say($lang, "file_binary"), { kib: kib(shown.bytes) })}</p>
  {:else}
    <pre
      class="mt-tight max-h-output overflow-auto rounded-card border border-edge bg-page p-snug font-mono text-note whitespace-pre-wrap text-text-quiet">{shown.text}</pre>
    {#if shown.truncated}
      <p class="text-text-disabled">
        {fill(say($lang, "file_truncated"), { kib: kib(shown.text.length), total: kib(shown.bytes) })}
      </p>
    {/if}
  {/if}
</details>
