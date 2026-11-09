<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts" module>
  import type { Snippet } from "svelte";
  import type { Attachment } from "svelte/attachments";

  import type { ModelEditorLook } from "./model_editor";

  export interface ModelEditorLookProps extends ModelEditorLook {
    readonly table: Snippet;
    readonly generation: Snippet;
    readonly holdSummary: Attachment<HTMLElement>;
    readonly open?: boolean;
    readonly onToggle: (open: boolean) => void;
  }
</script>

<script lang="ts">
  // The native disclosure and the row-level controls. The table arrives
  // from the seat as the same ModelTable the attach form uses; every
  // word, refusal and press is already wired by ./model_editor.
  import Button from "../../parts/button.svelte";

  const look: ModelEditorLookProps = $props();
</script>

<details
  class="mt-snug"
  open={look.open ?? false}
  ontoggle={(event) => {
    look.onToggle(event.currentTarget.open);
  }}
>
  <summary
    class="w-fit cursor-pointer rounded-control px-tight text-text-quiet transition-[background-color] ease-leave hover:bg-raised hover:ease-arrive"
    {@attach look.holdSummary}
  >
    {look.summary}
  </summary>
  <div class="flex flex-col gap-snug pt-snug">
    <div class="flex flex-wrap items-center gap-snug">
      <Button {...look.refresh} />
    </div>
    {#if look.removal !== undefined}
      <p class="text-note text-text-quiet">{look.removal}</p>
    {/if}
    <!-- eslint-disable-next-line @typescript-eslint/no-confusing-void-expression (a snippet call is the render itself; the typechecker types local snippet calls as returning void) -->
    {@render look.table()}
  </div>
</details>
<!-- Generation stays outside the model-table disclosure so Access always
  shows the sampling defaults without opening another fold. Save, its
  flight and its refusal sit with it: generation edits need the same
  commit as the table, and a closed fold must not hide that control. -->
<div class="mt-snug flex flex-col gap-snug border-t border-edge pt-snug">
  <h4 class="text-label font-label text-text-quiet">{look.generationHeading}</h4>
  <!-- eslint-disable-next-line @typescript-eslint/no-confusing-void-expression (a snippet call is the render itself; the typechecker types local snippet calls as returning void) -->
  {@render look.generation()}
  <div class="flex flex-wrap items-center gap-snug">
    <Button {...look.save} />
  </div>
  {#if look.busy !== undefined}
    <p role="status" class="text-note text-text-quiet">{look.busy}</p>
  {/if}
  {#if look.alert !== undefined}
    <div
      role="alert"
      class="rounded-card border border-alert/50 bg-chrome px-base py-snug text-note"
    >
      <p class="font-label text-alert">{look.alert.title}</p>
      {#if look.alert.subject !== ""}
        <!-- wording-ok: the city's own refusal subject, which names the model or endpoint as the city spells it -->
        <p class="mt-tight font-mono wrap-anywhere text-text">{look.alert.subject}</p>
      {/if}
      {#if look.alert.code !== undefined}
        <!-- wording-ok: the city's stable error code -->
        <p class="mt-tight font-mono text-text-faint">{look.alert.code}</p>
      {/if}
    </div>
  {/if}
  {#if look.status !== undefined}
    <p role="status" class="text-note text-text-quiet">{look.status}</p>
  {/if}
</div>
