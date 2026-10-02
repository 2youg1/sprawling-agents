<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts">
  // A file whose bytes are not text, as RefRain opens it (client/Spec.lean
  // §4-54, §4-61). A PDF or a DOCX is fetched from the city by its version,
  // window by window, and handed to `pdf.svelte` or `docx.svelte`; a
  // second version picked from the document's list is fetched the same
  // way and the two are compared by their text. Any other file is named
  // by its size, as before.
  import { readAnswer } from "../../../core/answered";
  import { DRAWN_BYTES_MAX } from "../../../core/document_bytes";
  import { fill, say } from "../../../core/lang";
  import { kib } from "../../../core/time";
  import { ui } from "../../../ui";
  import type { Address, B3Hash } from "../../../wire";
  import { StoredBytes } from "../fetched.svelte";
  import { short } from "../reading";
  import Compare from "./compare.svelte";
  import Docx from "./docx.svelte";
  import { NAME, TOOL, drawnAs } from "./format";
  import Label from "./label.svelte";
  import Pdf from "./pdf.svelte";

  interface Props {
    readonly path: Address;
    readonly version: B3Hash;
    // The file's length in bytes.
    readonly bytes: number;
  }

  const { path, version, bytes }: Props = $props();

  const u = ui();
  const { lang } = u;

  const format = $derived(drawnAs(path));
  const name = $derived(path.slice(path.lastIndexOf("/") + 1));
  const drawable = $derived((format === "pdf" || format === "docx") && bytes <= DRAWN_BYTES_MAX);

  const current = new StoredBytes(u.conn.asking);
  const other = new StoredBytes(u.conn.asking);
  let against = $state<B3Hash | null>(null);

  $effect(() => {
    current.start(drawable ? version : null);
  });
  $effect(() => {
    other.start(drawable ? against : null);
  });

  // The other versions this document had that the store still keeps.
  const asked = $derived(u.conn.asking.ask({ versions: { at: path } }));
  const listed = $derived(readAnswer($asked, (answer) => ("versions" in answer ? answer.versions : undefined)));
  const others = $derived(
    listed.kind === "held"
      ? listed.value.versions.filter(
          (each, index, all) => each.kept && each.version !== version && all.findIndex((seen) => seen.version === each.version) === index,
        )
      : [],
  );

  const mine = $derived(current.value);
  const theirs = $derived(other.value);
</script>

{#if format === "pdf" || format === "docx"}
  {#if !drawable}
    <Label {format} tool={TOOL[format]} settings={[]} {version} />
    <p class="px-wide py-snug text-note text-text-quiet">
      {fill(say($lang, "format_too_large"), { format: NAME[format], kib: kib(bytes), most: kib(DRAWN_BYTES_MAX) })}
    </p>
  {:else}
    {#if others.length > 0}
      <div class="flex items-center gap-base border-b border-edge px-wide py-tight">
        <label class="flex min-w-0 flex-1 items-center gap-snug text-note text-text-faint">
          {say($lang, "format_compare_with")}
          <select
            class="h-control-sm min-w-0 flex-1 rounded-control border border-edge-input bg-raised px-snug text-note text-text"
            value={against ?? ""}
            onchange={(event) => {
              const picked = others.find((each) => each.version === event.currentTarget.value);
              against = picked?.version ?? null;
            }}
          >
            <option value="">{say($lang, "format_compare_none")}</option>
            {#each others as each (each.version)}
              <option value={each.version}>{short(each.version)}</option>
            {/each}
          </select>
        </label>
      </div>
    {/if}
    {#if current.lost !== null || other.lost !== null}
      <p class="px-wide py-snug text-note text-text-quiet">{say($lang, "format_lost")}</p>
    {:else if mine?.kind === "whole" && against !== null && theirs?.kind === "whole"}
      <Compare {format} from={{ version: against, bytes: theirs.bytes }} to={{ version, bytes: mine.bytes }} {name} />
    {:else if mine?.kind === "whole" && against === null}
      {#if format === "pdf"}
        <Pdf bytes={mine.bytes} {version} {name} />
      {:else}
        <Docx bytes={mine.bytes} {version} {name} />
      {/if}
    {:else}
      <Label {format} tool={TOOL[format]} settings={[]} {version} />
      <p class="px-wide py-snug text-note text-text-faint">
        {fill(say($lang, "format_fetching"), {
          format: NAME[format],
          through: kib(mine?.kind === "fetching" ? mine.through : 0),
          kib: kib(bytes),
        })}
      </p>
    {/if}
  {/if}
{:else}
  <p class="px-wide py-snug text-note text-text-quiet">{fill(say($lang, "file_binary"), { kib: kib(bytes) })}</p>
{/if}
