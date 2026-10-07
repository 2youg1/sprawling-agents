<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts">
  // How the one labelled box is drawn, and nothing else: the words
  // arrive in the person's language and every id, ARIA property and the
  // input gate arrive in the bags (`./field.ts`), so another look draws
  // the same field by taking the same `FieldLook`.
  //
  // **Two authorities decide the red border, and they say different
  // things.** `refused` is the city's answer, so it is drawn the moment
  // the page is told; `:user-invalid` is the browser's own reading of
  // `type` and `pattern`, and it holds off until the person has left the
  // box, so a URL is never red while it is half typed. A box nobody may
  // edit is never graded on what it holds, so `:user-invalid` is
  // switched off while disabled.
  //
  // The box carries no `outline` rule of its own. The focus ring is one
  // declaration in `theme/base.css`, and a box that hid it was the reason
  // a keyboard user could not see where they were on the settings page.
  import type { FieldLook } from "./field";

  const look: FieldLook = $props();

  const AFFIX = "shrink-0 font-mono text-note text-text-faint";
</script>

<div class="flex w-full min-w-0 flex-col gap-tight">
  <label {...look.labelWire} class={["text-note text-text-quiet", look.labelling === "hidden" && "sr-only"]}>
    {look.label}
  </label>
  <div
    class={[
      "flex h-control min-w-0 items-center gap-tight rounded-control border bg-raised px-base",
      look.refused ? "border-alert" : "border-edge-input",
      !look.disabled && "has-[:user-invalid]:border-alert",
    ]}
  >
    {#if look.prefix !== undefined}
      <span class={AFFIX}>{look.prefix}</span>
    {/if}
    <input
      {...look.box}
      class={[
        "min-w-0 flex-1 bg-transparent text-body placeholder:text-text-faint",
        look.disabled ? "aria-disabled:text-text-disabled" : "text-text",
        look.mono && "font-mono",
      ]}
    />
    {#if look.suffix !== undefined}
      <span class={AFFIX}>{look.suffix}</span>
    {/if}
  </div>
  {#if look.note !== undefined}
    <p {...look.note.wire} class={["text-note", look.note.kind === "error" ? "text-alert" : "text-text-faint"]}>
      {look.note.text}
    </p>
  {/if}
</div>
