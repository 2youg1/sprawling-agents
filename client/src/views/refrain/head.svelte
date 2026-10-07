<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts">
  // RefRain's head line, the seat (client D95): where the document is,
  // which version the editor stands on and what became of the last save,
  // then the readings and the save, drawn by `./head.look.svelte`. The
  // receipt says a word only when there is something unsaved or a save
  // to report; a document nobody touched shows none. A Markdown version
  // can be exported as one HTML file the city writes (4-61).
  import { readAnswer } from "../../core/answered";
  import { say } from "../../core/lang";
  import type { Key } from "../../core/lang";
  import { ui } from "../../ui";
  import type { Address } from "../../wire";
  import Button from "../parts/button.svelte";
  import Segmented from "../parts/segmented.svelte";
  import { receiptWord } from "./head";
  import Look from "./head.look.svelte";
  import type { Session } from "./session.svelte";
  import { short, type Reading } from "./reading";
  import { saveFile } from "./saved_file";

  interface Props {
    readonly at: Address;
    readonly session: Session;
    readonly reading: Reading;
    // Whether the document has a preview: Markdown, read by the city, or
    // HTML, drawn by this browser.
    readonly previewed: boolean;
    readonly onPick: (reading: Reading) => void;
  }

  const { at, session, reading, previewed, onPick }: Props = $props();

  const lang = ui().lang;

  // A document's address starts with its building's (`documentAt`), so
  // its folder already reads from the building's name.
  const cut = $derived(at.lastIndexOf("/") + 1);
  const name = $derived(at.slice(cut));

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

  const said = $derived(receiptWord(session.receipt.kind));
  // Why the save cannot be pressed, when it cannot.
  const blocked = $derived.by((): { readonly why?: string } => {
    const kind = session.receipt.kind;
    if (session.locked !== null || session.positions === null) return { why: say($lang, "refrain_save_locked") };
    if (kind === "saving" || kind === "pending") return { why: say($lang, "refrain_save_wait") };
    return kind === "draft" || kind === "refused" ? {} : { why: say($lang, "refrain_save_nothing") };
  });
  const version = $derived(session.positions === null ? null : short(session.positions.version));

  const u = ui();
  const markdown = $derived(session.file?.kind === "text" && session.file.gathering.format === "markdown");
  let exporting = $state<"asking" | "refused" | null>(null);

  // Asks the city once for the export of the version the editor stands
  // on, and hands the answer to the browser to save.
  function exportHtml(): void {
    const standing = session.positions?.version;
    if (standing === undefined) return;
    exporting = "asking";
    const asked = { finished: false };
    let stop: (() => void) | null = null;
    stop = u.conn.asking.ask({ export: { at, version: standing } }).subscribe((answer) => {
      const read = readAnswer(answer, (held) => ("export" in held ? held.export : undefined));
      if (asked.finished || read.kind === "asking") return;
      asked.finished = true;
      if (read.kind === "held") saveFile(`${name}.html`, "text/html", read.value.html);
      exporting = read.kind === "held" ? null : "refused";
      stop?.();
    });
    if (asked.finished) stop();
  }
</script>

<Look
  place={{ folder: at.slice(0, cut), name }}
  {version}
  receipt={said === null ? null : { text: say($lang, said.key), alert: said.alert }}
  wire={{ role: "status" }}
>
  {#snippet actions()}
    {#if session.file?.kind === "text"}
      <!-- A file that is not text has no source, diff or versions to read. -->
      <Segmented label={say($lang, "refrain_reading")} {options} held={reading} {onPick} />
    {/if}
    {#if exporting === "refused"}
      <span class="text-note text-text-quiet" role="status">{say($lang, "refrain_export_refused")}</span>
    {/if}
    {#if markdown}
      <Button
        label={say($lang, "refrain_export_html")}
        tone="quiet"
        {...exporting === "asking" ? { why: say($lang, "refrain_export_asking") } : {}}
        onPress={exportHtml}
      />
    {/if}
    <Button
      label={say($lang, "refrain_save")}
      tone="quiet"
      {...blocked}
      onPress={() => {
        session.save();
      }}
    />
  {/snippet}
</Look>
