<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts">
  // RefRain's head line: where the document is, which version the editor
  // stands on and what became of the last save, then the readings and
  // the save. One line, the seat docs/frontend-method.md §7F gives the line above the editor.
  // The receipt says a word only when there is something unsaved or a
  // save to report; a document nobody touched shows none.
  import { say } from "../../core/lang";
  import type { Key } from "../../core/lang";
  import type { Receipt } from "../../core/document_save";
  import { ui } from "../../ui";
  import type { Address } from "../../wire";
  import Button from "../parts/button.svelte";
  import Segmented from "../parts/segmented.svelte";
  import type { Session } from "./session.svelte";
  import { short, type Reading } from "./reading";

  interface Props {
    readonly at: Address;
    readonly building: Address;
    readonly session: Session;
    readonly reading: Reading;
    // Whether the document has a preview: Markdown, read by the city, or
    // HTML, drawn by this browser.
    readonly previewed: boolean;
    readonly onPick: (reading: Reading) => void;
  }

  const { at, building, session, reading, previewed, onPick }: Props = $props();

  const lang = ui().lang;

  const folder = $derived(at.slice(0, at.lastIndexOf("/") + 1));
  const name = $derived(at.slice(at.lastIndexOf("/") + 1));
  const inside = $derived(folder.startsWith(`${building}/`) ? folder.slice(building.length + 1) : folder);

  const READINGS: readonly { readonly value: Reading; readonly key: Key }[] = [
    { value: "source", key: "refrain_source" },
    { value: "preview", key: "refrain_preview" },
    { value: "diff", key: "refrain_diff" },
    { value: "versions", key: "refrain_versions" },
  ];
  const options = $derived(
    READINGS.filter((each) => previewed || each.value !== "preview").map((each) => ({
      value: each.value,
      label: say($lang, each.key),
    })),
  );

  // The receipt's word and whether it asks for the person, by state.
  function word(receipt: Receipt): { readonly key: Key; readonly alert: boolean } | null {
    switch (receipt.kind) {
      case "clean":
        return null;
      case "draft":
        return { key: "refrain_receipt_draft", alert: true };
      case "saving":
        return { key: "refrain_receipt_saving", alert: false };
      case "pending":
        return { key: "refrain_receipt_pending", alert: true };
      case "saved":
        return { key: "refrain_receipt_saved", alert: false };
      case "conflict":
        return { key: "refrain_receipt_conflict", alert: true };
      case "refused":
        return { key: "refrain_receipt_refused", alert: true };
    }
  }

  const said = $derived(word(session.receipt));
  // Why the save cannot be pressed, when it cannot.
  const blocked = $derived.by((): { readonly why?: string } => {
    const kind = session.receipt.kind;
    if (session.locked !== null || session.positions === null) return { why: say($lang, "refrain_save_locked") };
    if (kind === "saving" || kind === "pending") return { why: say($lang, "refrain_save_wait") };
    return kind === "draft" || kind === "refused" ? {} : { why: say($lang, "refrain_save_nothing") };
  });
  const version = $derived(session.positions === null ? null : short(session.positions.version));
</script>

<!-- Two groups: where and what state, then the readings and the save.
     A container too narrow for one line puts the second under the first. -->
<div class="refrain-head">
  <div class="refrain-where">
    <p class="flex min-w-0 items-baseline truncate">
      <span class="truncate text-text-faint">{building}/{inside}</span><span class="text-text-quiet">{name}</span>
    </p>
    {#if version !== null}
      <span class="shrink-0 text-text-faint">{version}</span>
    {/if}
    {#if said !== null}
      <span class="refrain-receipt" data-alert={said.alert} role="status">{say($lang, said.key)}</span>
    {:else}
      <span class="sr-only" role="status"></span>
    {/if}
  </div>
  <div class="flex shrink-0 items-center gap-snug">
    {#if session.file?.kind === "text"}
      <!-- A file that is not text has no source, diff or versions to read. -->
      <Segmented label={say($lang, "refrain_reading")} {options} held={reading} {onPick} />
    {/if}
    <Button
      label={say($lang, "refrain_save")}
      tone="quiet"
      {...blocked}
      onPress={() => {
        session.save();
      }}
    />
  </div>
</div>
