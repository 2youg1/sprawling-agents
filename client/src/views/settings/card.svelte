<!-- This Source Code Form is subject to the terms of the Mozilla Public
     License, v. 2.0. If a copy of the MPL was not distributed with this
     file, You can obtain one at https://mozilla.org/MPL/2.0/.
     Copyright (c) 2026 2youg1 and the sprawling contributors -->

<script lang="ts">
  // One settings card that is saved by a press (client/Spec.lean §4-36): a
  // title, one line saying what it governs, its fields, and a foot with
  // where the save stands on the left and the save button on the right.
  // The standing is a live region, so the receipt is announced when it
  // arrives rather than when the button is pressed; a refusal keeps the
  // draft in the fields and says the city's own way on.

  import type { Snippet } from "svelte";

  import { say } from "../../core/lang";
  import type { Key } from "../../core/lang";
  import { ui } from "../../ui";
  import Button from "../parts/button.svelte";
  import Glyph from "../parts/glyph.svelte";
  import type { Saving } from "./saving";

  interface Props {
    readonly title: Key;
    readonly note: Key;
    readonly saving: Saving;
    // When a saved change takes effect, said once it is saved.
    readonly settled: Key;
    readonly onSave: () => void;
    readonly children: Snippet;
  }

  const { title, note, saving, settled, onSave, children }: Props = $props();
  const { lang } = ui();

  const WORD: Record<Saving["kind"], Key | null> = {
    held: null,
    draft: "saving_draft",
    saving: "saving_sent",
    saved: "saving_saved",
    refused: "saving_refused",
    unverified: "saving_unverified",
  };
  const word = $derived(WORD[saving.kind]);
</script>

<div class="flex min-w-0 flex-col gap-snug rounded-card bg-raised px-base py-snug">
  <div class="flex flex-col gap-hair">
    <span class="text-label font-label text-text">{say($lang, title)}</span>
    <p class="text-note text-text-faint">{say($lang, note)}</p>
  </div>
  <!-- eslint-disable-next-line @typescript-eslint/no-confusing-void-expression -->
  {@render children()}
  <div class="mt-tight flex items-center gap-base">
    <span class="flex min-w-0 flex-1 flex-col gap-hair text-note" role="status">
      {#if word !== null}
        <span class={["inline-flex items-center gap-tight", saving.kind === "refused" ? "text-alert" : "text-text-quiet"]}>
          {#if saving.kind === "saved"}
            <Glyph name="check" size="sm" class="shrink-0" />
          {/if}
          {say($lang, word)}
        </span>
      {/if}
      {#if saving.kind === "saved"}
        <span class="text-text-faint">{say($lang, settled)}</span>
      {:else if saving.kind === "refused"}
        <span class="text-text-faint">{saving.error.recovery}</span>
      {/if}
    </span>
    <Button
      label={say($lang, "saving_save")}
      tone="primary"
      loading={saving.kind === "saving"}
      {...(saving.kind === "draft" || saving.kind === "refused" || saving.kind === "unverified" ? {} : { why: say($lang, "saving_nothing") })}
      onPress={onSave}
    />
  </div>
</div>
