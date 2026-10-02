<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts">
  // A file whose bytes are not text, as RefRain opens it (client/Spec.lean
  // §4-54). A PDF or a DOCX is drawn by `pdf.svelte` or `docx.svelte`
  // from its bytes, and no answer on the wire carries a file's bytes to
  // the page yet (§3-4), so here the format's line says which tool would
  // draw it and that the page has nothing to draw it from. Any other
  // file is named by its size, as before.
  import { fill, say } from "../../../core/lang";
  import { kib } from "../../../core/time";
  import { ui } from "../../../ui";
  import type { B3Hash } from "../../../wire";
  import { NAME, TOOL, drawnAs } from "./format";
  import Label from "./label.svelte";

  interface Props {
    readonly path: string;
    readonly version: B3Hash;
    // The file's length in bytes.
    readonly bytes: number;
  }

  const { path, version, bytes }: Props = $props();

  const lang = ui().lang;

  const format = $derived(drawnAs(path));
</script>

{#if format === "pdf" || format === "docx"}
  <Label
    {format}
    tool={TOOL[format]}
    settings={[say($lang, format === "pdf" ? "format_pdf_settings" : "format_docx_settings")]}
    {version}
  />
  <p class="px-wide py-snug text-note text-text-quiet">{fill(say($lang, "format_no_bytes"), { format: NAME[format] })}</p>
{:else}
  <p class="px-wide py-snug text-note text-text-quiet">{fill(say($lang, "file_binary"), { kib: kib(bytes) })}</p>
{/if}
