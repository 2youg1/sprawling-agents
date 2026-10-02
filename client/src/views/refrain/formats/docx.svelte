<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts">
  // A DOCX laid out as its pages, approximately, in a sandboxed frame
  // (client/Spec.lean §4-54). The line above says the layout is approximate
  // and counts what the file holds that the pages do not show: tracked
  // changes drawn as their result, comments and embedded objects left
  // out. An archive past the bounds is refused unopened, and the line
  // says why.
  import { fill, say } from "../../../core/lang";
  import { kib } from "../../../core/time";
  import { ui } from "../../../ui";
  import type { B3Hash } from "../../../wire";
  import type { Read } from "./docx";
  import { NAME, TOOL } from "./format";
  import Frame from "./frame.svelte";
  import Label from "./label.svelte";
  import { UNPACKED_MAX } from "./zip";

  interface Props {
    readonly bytes: Uint8Array;
    readonly version: B3Hash | null;
    readonly name: string;
  }

  const { bytes, version, name }: Props = $props();

  const lang = ui().lang;

  let read = $state<Read | null>(null);

  $effect(() => {
    const from = bytes;
    let gone = false;
    read = null;
    void import("./docx")
      .then(({ readDocx }) => readDocx(from))
      .then((result) => {
        if (!gone) read = result;
      });
    return () => {
      gone = true;
    };
  });

  const notes = $derived.by((): readonly string[] => {
    if (read?.kind !== "drawn") return [];
    const { revisions, comments, objects } = read.coverage;
    return [
      ...(revisions > 0 ? [fill(say($lang, "format_docx_revisions"), { n: String(revisions) })] : []),
      ...(comments > 0 ? [fill(say($lang, "format_docx_comments"), { n: String(comments) })] : []),
      ...(objects > 0 ? [fill(say($lang, "format_docx_objects"), { n: String(objects) })] : []),
    ];
  });

  const refusal = $derived.by((): string | null => {
    if (read === null || read.kind === "drawn") return null;
    const reason =
      read.kind === "broken"
        ? read.reason
        : read.why === "too_large"
          ? fill(say($lang, "format_zip_too_large"), { limit: kib(UNPACKED_MAX) })
          : say($lang, "format_zip_shape");
    return fill(say($lang, "format_broken"), { format: NAME.docx, reason });
  });
</script>

<div class="flex min-h-0 flex-1 flex-col">
  <Label format="docx" tool={TOOL.docx} settings={[say($lang, "format_docx_settings")]} {version} {notes} />
  {#if read === null}
    <p class="px-wide py-snug text-note text-text-faint">{fill(say($lang, "format_reading"), { format: NAME.docx })}</p>
  {:else if read.kind === "drawn"}
    <div class="flex min-h-0 flex-1 flex-col bg-raised">
      <Frame page={read.page} scheme="built" title={name} />
    </div>
  {:else}
    <p class="px-wide py-snug text-note text-text-quiet">{refusal}</p>
  {/if}
</div>
