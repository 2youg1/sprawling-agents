<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts">
  // Two versions of a PDF or a DOCX compared by the text each yields
  // (client/Spec.lean §4-54, client D32): a PDF by the text of its pages in reading
  // order, a page mark between pages; a DOCX by its body's paragraphs as
  // the tracked changes leave them. The comparison is RefRain's own
  // read-only diff, so a changed word looks the way it does in a
  // Markdown version. The line above names what is and is not
  // compared, and which pages had no text to compare - a scan is not
  // read by OCR, and its silence is not "no change". The comparison can
  // be saved as a Markdown file a format-capable tool acts on (4-61).
  import { fill, say } from "../../../core/lang";
  import { ui } from "../../../ui";
  import type { B3Hash } from "../../../wire";
  import Button from "../../parts/button.svelte";
  import { phrasesIn, short } from "../reading";
  import { saveFile } from "../saved_file";
  import { NAME, TOOL } from "./format";
  import Label from "./label.svelte";

  interface Side {
    readonly version: B3Hash;
    readonly bytes: Uint8Array;
  }

  interface Props {
    readonly format: "pdf" | "docx";
    readonly from: Side;
    readonly to: Side;
    readonly name: string;
  }

  const { format, from, to, name }: Props = $props();

  const lang = ui().lang;

  // The text one version yields, and its pages that yielded none.
  interface Yield {
    readonly text: string;
    readonly silent: readonly number[];
  }

  let texts = $state<{ readonly from: Yield; readonly to: Yield } | "broken" | null>(null);
  let host = $state<HTMLDivElement>();

  function yieldOf(bytes: Uint8Array): Promise<Yield | null> {
    if (format === "docx") {
      return import("./docx")
        .then(({ docxLines }) => docxLines(bytes))
        .then((lines) => (lines === null ? null : { text: lines.join("\n"), silent: [] }));
    }
    return import("./pdf").then(({ openPdf, pagesText }) => {
      const opening = openPdf(bytes);
      return opening.opened
        .then((opened) => (opened.kind === "open" ? pagesText(opened.pdf) : null))
        .then((pages) => {
          opening.close();
          if (pages === null) return null;
          const mark = (n: number): string => `— ${fill(say($lang, "format_page_mark"), { n: String(n) })} —`;
          return {
            text: pages.map((page, index) => `${mark(index + 1)}\n${page}`).join("\n"),
            silent: pages.flatMap((page, index) => (page.trim() === "" ? [index + 1] : [])),
          };
        });
    });
  }

  $effect(() => {
    const a = from.bytes;
    const b = to.bytes;
    let gone = false;
    texts = null;
    void Promise.all([yieldOf(a), yieldOf(b)]).then(([left, right]) => {
      if (!gone) texts = left === null || right === null ? "broken" : { from: left, to: right };
    });
    return () => {
      gone = true;
    };
  });

  $effect(() => {
    const parent = host;
    const shown = texts;
    if (parent === undefined || shown === null || shown === "broken") return;
    const phrases = phrasesIn($lang);
    let opened: { readonly destroy: () => void } | null = null;
    let gone = false;
    void import("../editing").then(({ openComparison }) => {
      if (!gone) opened = openComparison(parent, { from: shown.from.text, to: shown.to.text }, name, phrases);
    });
    return () => {
      gone = true;
      opened?.destroy();
    };
  });

  const notes = $derived.by((): readonly string[] => {
    const versions = fill(say($lang, "format_compared_versions"), { from: short(from.version), to: short(to.version) });
    if (texts === null || texts === "broken") return [versions];
    const silent = [...new Set([...texts.from.silent, ...texts.to.silent])].sort((a, b) => a - b);
    return silent.length === 0 ? [versions] : [versions, fill(say($lang, "format_no_text"), { pages: silent.join(", ") })];
  });

  const settings = $derived(say($lang, format === "pdf" ? "format_compare_pdf" : "format_compare_docx"));

  function exportComparison(): void {
    if (texts === null || texts === "broken") return;
    const comparison = {
      name,
      from: { label: short(from.version), text: texts.from.text },
      to: { label: short(to.version), text: texts.to.text },
      about: [`${NAME[format]} · ${TOOL[format]} · ${settings}`, ...notes],
    };
    void import("./compared").then(({ comparedMarkdown, comparedName }) => {
      saveFile(comparedName(name), "text/markdown", comparedMarkdown($lang, comparison));
    });
  }
</script>

<div class="refrain flex min-h-0 flex-1 flex-col">
  <Label
    {format}
    tool={TOOL[format]}
    settings={[settings]}
    version={null}
    {notes}
  />
  {#if texts === null}
    <p class="px-wide py-snug text-note text-text-faint">{fill(say($lang, "format_reading"), { format: NAME[format] })}</p>
  {:else if texts === "broken"}
    <p class="px-wide py-snug text-note text-text-quiet">{say($lang, "format_compare_broken")}</p>
  {:else if texts.from.text === texts.to.text}
    <p class="px-wide py-snug text-note text-text-quiet">{say($lang, "format_compare_same")}</p>
  {:else}
    <div class="flex justify-end border-b border-edge px-wide py-tight">
      <Button label={say($lang, "refrain_export_comparison")} tone="quiet" onPress={exportComparison} />
    </div>
    <div class="refrain-editor min-h-0 flex-1" bind:this={host}></div>
  {/if}
</div>
