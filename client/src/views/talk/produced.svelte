<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts">
  // The line under a finished run's outcome divider in results-only
  // mode: how many files the run changed and how many lines moved,
  // measured from the tree the run opened at (`RoundsAnswer.opened_at`)
  // to the tree it last fenced (`lastFenceIn` in `trace.ts`, the same
  // bound the run page asks with). The numbers are that `Changes`
  // answer summed once in `core/results.ts`. A run that fenced nothing
  // passes `head = null`, the working tree, exactly as the run page does.
  // `place` is where the line stands: centred under the divider in the
  // room, or at the end of a row of the results city.
  import { fill, say } from "../../core/lang";
  import { producedOf } from "../../core/results";
  import { ui } from "../../ui";
  import type { GitOid } from "../../wire";

  interface Props {
    readonly base: GitOid;
    readonly head: GitOid | null;
    readonly place: "divider" | "row";
  }

  const { base, head, place }: Props = $props();
  const u = ui();
  const { lang } = u;

  const asked = $derived(u.conn.asking.ask({ changes: { base, head } }));
  const produced = $derived.by(() => {
    const held = $asked;
    return held !== undefined && "changes" in held ? producedOf(held.changes.files) : null;
  });
</script>

{#if produced !== null && produced.files > 0 && place === "row"}
  <span class="shrink-0 text-note text-text-quiet" aria-label={say($lang, "results_produced")}>
    {fill(say($lang, "results_row_produced"), {
      files: String(produced.files),
      added: String(produced.added),
      removed: String(produced.removed),
    })}
  </span>
{:else if produced !== null && produced.files > 0}
  <p class="-mt-snug mb-wide text-center text-note text-text-quiet" aria-label={say($lang, "results_produced")}>
    {fill(say($lang, "results_produced_line"), {
      files: String(produced.files),
      added: String(produced.added),
      removed: String(produced.removed),
    })}
  </p>
{/if}
