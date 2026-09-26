<script lang="ts" module>
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// A file the way a tool handed it over: which file it was, which line
// each row is, and enough colour to tell a comment from a string.
//
// **Read-only by construction.** There is no editor here, no minimap
// and no language server: this view answers "what did that call
// produce", and every one of those three answers a question about
// editing a project instead.
//
// The row that names the file also carries the one secondary action:
// copy, shown when the pointer or the keyboard is inside this view and
// nowhere else, so the code is the only thing that greets the eye.

import type { Ink } from "./code";

// Every colour comes from `theme.css`; this file states no value. The
// fifth ink is the plain text around the four the theme distinguishes.
export const PAINT: Record<Ink, string> = {
  plain: "",
  comment: "text-text-disabled",
  string: "text-alert",
  number: "text-accent",
  word: "text-text",
};

// How long the copy receipt holds its check mark: long enough to see
// one, short enough that the mark never becomes the button's face.
const RECEIPT_MS = 1200;

// The secondary action, at the small control height. It is invisible
// and click-through at rest, so the corner never stands between a hand
// and the text under it, and it arrives as `fade` - the motion
// vocabulary's name for a state change that keeps its position. The
// `::before` widens the touch surface to 44 while the drawn control
// stays 28, and a machine that asks for less motion simply gets the
// mark without the arrival.
const SHAPE =
  "relative ms-auto flex h-control-sm w-control-sm shrink-0 items-center " +
  "justify-center rounded-control text-text-quiet opacity-0 before:absolute " +
  "before:-inset-snug before:content-[''] transition-opacity duration-150 " +
  "ease-standard hover:bg-raised hover:text-text group-hover:opacity-100 " +
  "group-hover:pointer-events-auto group-focus-within:opacity-100 " +
  "group-focus-within:pointer-events-auto motion-reduce:transition-none pointer-events-none";
</script>

<script lang="ts">
  import { say } from "../../core/lang";
  import { ui } from "../../ui";
  import { painted } from "./code";

  interface Props {
    // The file this came from, as the tool named it. An empty string
    // when the call named none: the trail is then not drawn and nothing
    // is coloured.
    readonly path: string;
    readonly text: string;
  }

  const { path, text }: Props = $props();

  const { lang } = ui();

  const pieces = $derived(painted(text, path));
  // Crumbs carry their own identity so a path whose segment repeats -
  // `src/.../src/...` - keys no two rows alike.
  const crumbs = $derived(
    path
      .split("/")
      .filter((part) => part !== "")
      .map((part) => ({ part })),
  );

  // Line numbers start at one because the wire hands over the head of a
  // result and says nothing about where in the file it began. The day a
  // call carries that offset, this becomes a prop rather than a fact
  // decided here (client-SPEC 4-26).
  const gutter = $derived(
    text
      .split("\n")
      .map((_line, at) => String(at + 1))
      .join("\n"),
  );

  let copied = $state(false);
  let receipt: ReturnType<typeof setTimeout> | undefined = undefined;

  // The receipt appears only after the write landed, so a press that
  // quietly failed shows no check rather than a lying one.
  function copy(): void {
    void navigator.clipboard.writeText(text).then(() => {
      copied = true;
      if (receipt !== undefined) clearTimeout(receipt);
      receipt = setTimeout(() => {
        copied = false;
        receipt = undefined;
      }, RECEIPT_MS);
    });
  }
</script>

<div class="group flex min-h-0 min-w-0 flex-col">
  <div class="flex items-center gap-tight border-b border-edge px-snug py-tight text-note">
    {#if crumbs.length > 0}
      <nav class="flex min-w-0 flex-wrap items-center gap-tight" aria-label={say($lang, "code_crumbs")}>
        {#each crumbs as crumb, at (crumb)}
          {#if at > 0}<span class="text-text-disabled" aria-hidden="true">/</span>{/if}<span
            class={at === crumbs.length - 1 ? "text-text-quiet" : "text-text-faint"}>{crumb.part}</span
          >
        {/each}
      </nav>
    {/if}
    <button
      type="button"
      class={SHAPE}
      aria-label={say($lang, copied ? "code_copied" : "code_copy")}
      onclick={copy}
    >
      <!-- Two marks and no motion between them: the receipt is a cut. -->
      {#if copied}
        <svg class="size-glyph" viewBox="0 0 20 20" aria-hidden="true" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round"><path d="M4 10.5 8 14.5 16 5.5" /></svg>
      {:else}
        <svg class="size-glyph" viewBox="0 0 20 20" aria-hidden="true" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round"><rect x="7" y="7" width="10" height="10" rx="2" /><path d="M13 7V5a2 2 0 0 0-2-2H5a2 2 0 0 0-2 2v6a2 2 0 0 0 2 2h2" /></svg>
      {/if}
    </button>
  </div>
  <!-- The code scrolls sideways and never folds a line in half: the
  gutter stays put at the left while the text runs under it. -->
  <div class="min-h-0 flex-1 overflow-auto">
    <div class="flex min-w-max font-mono text-note leading-relaxed">
      <pre class="sticky left-0 shrink-0 select-none bg-chrome px-snug text-right text-text-disabled" aria-hidden="true">{gutter}</pre>
      <pre class="px-snug text-text-quiet">{#each pieces as piece (piece)}<span class={PAINT[piece.ink]}>{piece.text}</span>{/each}</pre>
    </div>
  </div>
</div>
