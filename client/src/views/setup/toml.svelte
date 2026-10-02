<!-- This Source Code Form is subject to the terms of the Mozilla Public
     License, v. 2.0. If a copy of the MPL was not distributed with this
     file, You can obtain one at https://mozilla.org/MPL/2.0/.
     Copyright (c) 2026 2youg1 and the sprawling contributors -->

<script lang="ts">
  // The city's own `CONFIG.toml`, as the city's tree holds it.
  //
  // **A proofing tool, not a setting.** It is collapsed at the foot of
  // every group rather than standing as a column: a block that costs the
  // cards their width is a block nobody reads (client-SPEC 4-36). What
  // stands inside is the file itself, read through `Query::Document`
  // and never spelled by this page, so it cannot disagree with the
  // file; a setting's value and the layer it came from are answered
  // beside the control that owns it (4-30).

  import { readDocument } from "../../core/document";
  import { say } from "../../core/lang";
  import { ui } from "../../ui";
  import EmptyState from "../parts/empty.svelte";
  import { CITY_CONFIG } from "../settings/files";

  const u = ui();
  const { lang } = u;
  const file = u.conn.asking.ask({ document: { at: CITY_CONFIG } });
  const read = $derived(readDocument($file));
</script>

<aside class="min-w-0" aria-label={say($lang, "setup_toml")}>
  <details class="rounded-card bg-chrome/60 px-base py-tight">
    <summary class="cursor-pointer text-label text-text-quiet hover:text-text">
      {say($lang, "setup_toml_toggle")}
      <code class="ml-snug font-mono text-note text-text-faint">{CITY_CONFIG}</code>
    </summary>
    {#if read.kind === "held" && read.value.text !== ""}
      <pre class="overflow-x-auto py-snug font-mono text-note text-text-quiet">{read.value.text}</pre>
    {:else}
      <EmptyState missing="setup_toml_unread" />
    {/if}
  </details>
</aside>
