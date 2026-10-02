<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts">
  // One path, drawn the same way everywhere a path is printed: what the
  // reader already knows is cut from the front, the rest is one line cut
  // off with an ellipsis at the end, the left button opens it inside the
  // page, and a second control offers to show it in the file manager.
  //
  // **A path is one line.** It is an identifier, and the rule this
  // client settled on for identifiers is the single-line ellipsis:
  // breaking it after a `/` let the engine break inside a segment when
  // one did not fit, and the value a person came to read became two
  // lines of which neither was the path. What a person already knows is
  // the prefix, so `base` cuts that end and the reveal control is how
  // the whole value is reached.
  //
  // The second control asks the city to hand the path to the desktop's
  // own file manager. A path this build cannot turn into an address
  // stays text with the control saying why (client/Spec.lean §7-2): the
  // control keeps its seat in the Tab order under `aria-disabled`, so a
  // keyboard reaches it and a screen reader reads the reason through the
  // hint, and pressing it lands in a no-op rather than in a surprise.
  import { Option, Schema } from "effect";

  import { reveal } from "../../core/commands";
  import { say } from "../../core/lang";
  import { ui } from "../../ui";
  import { Address } from "../../wire";
  import Glyph from "./glyph.svelte";
  import Tip from "./tip.svelte";

  interface Props {
    // As the city spells it: relative to the city, never to a disk.
    readonly path: string;
    // The part of the path the reader is already looking at, cut from
    // the front of what is drawn.
    readonly base?: string | undefined;
    // Absent leaves the path as text, which is what a path this build
    // cannot open should look like.
    readonly onOpen?: (() => void) | undefined;
  }

  const { path, base, onOpen }: Props = $props();

  const { lang, send } = ui();

  const TEXT = "min-w-0 truncate font-mono text-note text-text-quiet";

  const address = $derived(Schema.decodeOption(Address)(path));
  const shown = $derived(
    base !== undefined && path.startsWith(`${base}/`) ? path.slice(base.length + 1) : path,
  );
</script>

<span class="inline-flex min-w-0 max-w-full items-baseline gap-tight">
  {#if onOpen}
    <button
      type="button"
      class="{TEXT} text-left underline decoration-edge underline-offset-2 hover:text-text"
      onclick={() => {
        onOpen();
      }}
    >
      {shown}
    </button>
  {:else}
    <span class={TEXT}>{shown}</span>
  {/if}
  <Tip text={Option.isNone(address) ? say($lang, "path_reveal_inert") : say($lang, "path_reveal")}>
    {#snippet children(hint)}
      <button
        type="button"
        class="relative flex h-control-sm w-control-sm shrink-0 items-center justify-center rounded-control text-text-faint before:absolute before:-inset-snug before:content-[''] hover:text-text-quiet"
        aria-disabled={Option.isNone(address) ? "true" : undefined}
        aria-label={say($lang, "path_reveal")}
        aria-describedby={hint}
        onclick={() => {
          if (Option.isSome(address)) {
            send(reveal(address.value));
          }
        }}
      >
        <Glyph name="reveal" />
      </button>
    {/snippet}
  </Tip>
</span>
