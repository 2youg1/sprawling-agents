<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts">
  import { fill, say } from "../../core/lang";
  import { producedOf } from "../../core/results";
  import { ui } from "../../ui";
  import type { GitOid } from "../../wire";

  interface Props {
    readonly base: GitOid;
    readonly head: GitOid | null;
  }

  const { base, head }: Props = $props();
  const u = ui();
  const { lang } = u;

  const asked = $derived(u.conn.asking.ask({ changes: { base, head } }));
  const produced = $derived.by(() => {
    const held = $asked;
    return held !== undefined && "changes" in held ? producedOf(held.changes.files) : null;
  });
</script>

{#if produced !== null && produced.files > 0}
  <span class="shrink-0 text-note text-text-quiet" aria-label={say($lang, "results_produced")}>
    {fill(say($lang, "results_row_produced"), {
      files: String(produced.files),
      added: String(produced.added),
      removed: String(produced.removed),
    })}
  </span>
{/if}
