<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts" module>
  // A PDF drawn page by page with pdf.js (client-SPEC 4-54). Every page
  // has its place from the start, at the first page's proportions until
  // it is drawn, so the scroll bar is the length of the document; a page
  // is drawn only as it nears the view, and drawn again when the pane's
  // width moves by a step, so a hundred pages cost what the few on the
  // screen cost.

  // The width a page is drawn at moves in steps of this many pixels:
  // dragging the pane's edge redraws a page a few times, not once per
  // pixel, and the browser scales the drawing across a step.
  const STEP = 32;
</script>

<script lang="ts">
  import type { Attachment } from "svelte/attachments";
  import type { PDFDocumentProxy } from "pdfjs-dist";

  import { fill, say } from "../../../core/lang";
  import { ui } from "../../../ui";
  import type { B3Hash } from "../../../wire";
  import { NAME, TOOL } from "./format";
  import Label from "./label.svelte";
  import type { Opened, Size } from "./pdf";

  interface Props {
    readonly bytes: Uint8Array;
    readonly version: B3Hash | null;
    readonly name: string;
  }

  const { bytes, version, name }: Props = $props();

  const lang = ui().lang;

  let opened = $state<Opened | null>(null);
  // Each page's own proportions once it is drawn, and the first page's
  // as the stand-in for those not drawn yet.
  let sizes = $state<Readonly<Record<number, Size>>>({});
  let scroller = $state<HTMLDivElement>();
  let measured = $state(0);

  const width = $derived(Math.max(STEP, Math.floor(measured / STEP) * STEP));

  $effect(() => {
    const from = bytes;
    let close = (): void => undefined;
    let gone = false;
    opened = null;
    void import("./pdf").then(({ openPdf }) => {
      if (gone) return;
      const opening = openPdf(from);
      close = opening.close;
      void opening.opened.then((result) => {
        if (!gone) opened = result;
      });
    });
    return () => {
      gone = true;
      close();
    };
  });

  // A page the size of A4 stands in until the first page is measured.
  const A4: Size = { width: 595, height: 842 };
  const standIn = $derived(sizes[1] ?? A4);

  // One page: drawn once it comes within a screen of the view, and drawn
  // again at a new width.
  function page(pdf: PDFDocumentProxy, number: number): Attachment<HTMLCanvasElement> {
    return (canvas) => {
      const at = width;
      const root = scroller;
      let stop: (() => void) | null = null;
      const seen = new IntersectionObserver(
        (entries) => {
          if (stop !== null || !entries.some((entry) => entry.isIntersecting)) return;
          void import("./pdf").then(({ drawPage }) => {
            stop = drawPage({ pdf, number, canvas, width: at }, (size) => {
              sizes = { ...sizes, [number]: size };
            });
          });
        },
        { root: root ?? null, rootMargin: "100% 0px" },
      );
      seen.observe(canvas);
      return () => {
        seen.disconnect();
        stop?.();
      };
    };
  }

  const pdf = $derived(opened?.kind === "open" ? opened.pdf : null);
  const notes = $derived(pdf === null ? [] : [fill(say($lang, "format_pages"), { n: String(pdf.numPages) })]);
</script>

<div class="flex min-h-0 flex-1 flex-col">
  <Label
    format="pdf"
    tool={TOOL.pdf}
    settings={[say($lang, "format_pdf_settings"), fill(say($lang, "format_width"), { width: String(width) })]}
    {version}
    {notes}
  />
  {#if opened === null}
    <p class="px-wide py-snug text-note text-text-faint">{fill(say($lang, "format_reading"), { format: NAME.pdf })}</p>
  {:else if opened.kind === "locked"}
    <p class="px-wide py-snug text-note text-text-quiet">{say($lang, "format_locked")}</p>
  {:else if opened.kind === "broken"}
    <p class="px-wide py-snug text-note text-text-quiet">{fill(say($lang, "format_broken"), { format: NAME.pdf, reason: opened.reason })}</p>
  {:else if pdf !== null}
    <!-- The pages scroll inside the pane; the column inside measures
         the width a page may take. -->
    <!-- svelte-ignore a11y_no_noninteractive_tabindex (the pane takes focus so the keyboard can scroll it) -->
    <div class="min-h-0 flex-1 overflow-auto bg-raised p-pane" bind:this={scroller} tabindex="0" role="group" aria-label={name}>
      <div class="flex flex-col items-center gap-pane" bind:clientWidth={measured}>
        {#each { length: pdf.numPages }, index (index)}
          {@const size = sizes[index + 1] ?? standIn}
          <div
            class="shrink-0 bg-page shadow-raise"
            style:width="{width}px"
            style:aspect-ratio="{size.width} / {size.height}"
            role="img"
            aria-label={fill(say($lang, "format_page"), { n: String(index + 1), total: String(pdf.numPages) })}
          >
            <canvas class="block size-full" aria-hidden="true" {@attach page(pdf, index + 1)}></canvas>
          </div>
        {/each}
      </div>
    </div>
  {/if}
</div>
